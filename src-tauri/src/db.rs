use chrono::prelude::*;
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub db: Mutex<Connection>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub archived_at: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct TaskSchedule {
    pub id: i64,
    pub task_id: i64,
    pub effective_from: i64,
    pub effective_to: Option<i64>,
    #[serde(rename = "type")]
    pub type_: String, // "daily", "weekly", "monthly", "custom"
    pub weekday_mask: Option<i64>,
    pub monthday: Option<i64>,
    pub interval_days: Option<i64>,
    pub params_json: Option<String>,
}

pub fn init_db(app_handle: &AppHandle) -> Result<Connection> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to get app data dir");
    std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");
    let db_path = app_dir.join("db");

    let conn = Connection::open(db_path)?;

    // PRAGMAs
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 3000;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -20000;
         PRAGMA mmap_size = 268435456;
         PRAGMA wal_autocheckpoint = 1000;
         PRAGMA journal_size_limit = 67108864;",
    )?;

    // Schema
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS task (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            title         TEXT NOT NULL,
            notes         TEXT,
            is_active     INTEGER NOT NULL DEFAULT 1,
            created_at    INTEGER NOT NULL DEFAULT (unixepoch()),
            archived_at   INTEGER
        );

        CREATE TABLE IF NOT EXISTS task_schedule (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id         INTEGER NOT NULL,
            effective_from  INTEGER NOT NULL,
            effective_to    INTEGER,
            type            TEXT NOT NULL CHECK(type IN ('daily','weekly','monthly','custom')),
            weekday_mask    INTEGER,
            monthday        INTEGER CHECK(monthday BETWEEN 1 AND 28),
            interval_days   INTEGER,
            params_json     TEXT,
            FOREIGN KEY(task_id) REFERENCES task(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_schedule_task_effective
            ON task_schedule(task_id, effective_from, effective_to);

        CREATE TABLE IF NOT EXISTS task_completion (
            task_id   INTEGER NOT NULL,
            day       INTEGER NOT NULL,
            status    INTEGER NOT NULL CHECK(status IN (1,2)),
            done_at   INTEGER,
            PRIMARY KEY (task_id, day),
            FOREIGN KEY(task_id) REFERENCES task(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_completion_day ON task_completion(day);

        CREATE TABLE IF NOT EXISTS task_stats (
            task_id            INTEGER PRIMARY KEY,
            current_streak     INTEGER NOT NULL DEFAULT 0,
            best_streak        INTEGER NOT NULL DEFAULT 0,
            last_completed_day INTEGER,
            updated_at         INTEGER NOT NULL DEFAULT (unixepoch()),
            FOREIGN KEY(task_id) REFERENCES task(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS week_stats (
            week_start_day  INTEGER PRIMARY KEY,
            all_done        INTEGER NOT NULL DEFAULT 0,
            due_count       INTEGER NOT NULL DEFAULT 0,
            done_count      INTEGER NOT NULL DEFAULT 0,
            updated_at      INTEGER NOT NULL DEFAULT (unixepoch())
        );
        ",
    )?;

    // MIGRATION: Ensure params_json exists (for users with older DB version)
    // We ignore the error if column already exists
    let _ = conn.execute("ALTER TABLE task_schedule ADD COLUMN params_json TEXT", []);

    Ok(conn)
}

pub fn get_day_index() -> i64 {
    let now = Utc::now();
    now.timestamp() / 86400
}

#[derive(serde::Serialize)]
pub struct TaskWithStats {
    pub task: Task,
    pub schedule: TaskSchedule,
    pub current_streak: i64,
    pub best_streak: i64,
    pub today_status: bool, // true if done today
}

#[derive(serde::Serialize)]
pub struct MonthTask {
    pub id: i64,
    pub title: String,
    pub is_done: bool,
}

#[derive(serde::Serialize)]
pub struct MonthViewDay {
    pub day: i64,
    pub due_count: i64,
    pub done_count: i64,
    pub all_done: bool,
    pub tasks: Vec<MonthTask>,
}

pub fn add_task(
    conn: &Connection,
    title: String,
    frequency_type: String,
    weekday_mask: Option<i64>,
    monthday: Option<i64>,
    interval_days: Option<i64>,
) -> Result<()> {
    let day_index = get_day_index();

    conn.execute_batch("BEGIN TRANSACTION;")?;

    conn.execute(
        "INSERT INTO task (title, created_at) VALUES (?, ?)",
        params![title, Utc::now().timestamp()],
    )?;
    let task_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO task_schedule (task_id, effective_from, type, weekday_mask, monthday, interval_days)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![task_id, day_index, frequency_type, weekday_mask, monthday, interval_days],
    )?;

    conn.execute(
        "INSERT INTO task_stats (task_id, current_streak, best_streak) VALUES (?, 0, 0)",
        params![task_id],
    )?;

    conn.execute_batch("COMMIT;")?;
    Ok(())
}

pub fn list_tasks(conn: &Connection, day: Option<i64>) -> Result<Vec<TaskWithStats>> {
    let target_day = day.unwrap_or_else(get_day_index);

    // We need to join task, current schedule, stats, and completion for today
    // Note: This query assumes one active schedule per task (effective_to IS NULL check)
    let mut stmt = conn.prepare(
        "SELECT 
            t.id, t.title, t.notes, t.is_active, t.created_at, t.archived_at,
            s.id, s.effective_from, s.effective_to, s.type, s.weekday_mask, s.monthday, s.interval_days, s.params_json,
            st.current_streak, st.best_streak,
            tc.status
         FROM task t
         JOIN task_schedule s ON t.id = s.task_id
         LEFT JOIN task_stats st ON t.id = st.task_id
         LEFT JOIN task_completion tc ON t.id = tc.task_id AND tc.day = ?
         WHERE t.archived_at IS NULL AND s.effective_to IS NULL
         ORDER BY t.created_at DESC"
    )?;

    let task_iter = stmt.query_map([target_day], |row| {
        let task = Task {
            id: row.get(0)?,
            title: row.get(1)?,
            notes: row.get(2)?,
            is_active: row.get(3)?,
            created_at: row.get(4)?,
            archived_at: row.get(5)?,
        };
        let schedule = TaskSchedule {
            id: row.get(6)?,
            task_id: task.id,
            effective_from: row.get(7)?,
            effective_to: row.get(8)?,
            type_: row.get(9)?,
            weekday_mask: row.get(10)?,
            monthday: row.get(11)?,
            interval_days: row.get(12)?,
            params_json: row.get(13)?,
        };
        let current_streak: i64 = row.get(14).unwrap_or(0);
        let best_streak: i64 = row.get(15).unwrap_or(0);
        let status: Option<i64> = row.get(16)?;

        Ok(TaskWithStats {
            task,
            schedule,
            current_streak,
            best_streak,
            today_status: status.is_some(),
        })
    })?;

    let mut tasks = Vec::new();
    for task in task_iter {
        tasks.push(task?);
    }
    Ok(tasks)
}

pub fn delete_task(conn: &Connection, task_id: i64) -> Result<()> {
    conn.execute("DELETE FROM task WHERE id = ?", params![task_id])?;
    Ok(())
}

pub fn delete_all_tasks(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM task", [])?;
    Ok(())
}

pub fn toggle_completion(conn: &Connection, task_id: i64, day: i64) -> Result<()> {
    conn.execute_batch("BEGIN TRANSACTION;")?;
    let completed: Option<i64> = conn
        .query_row(
            "SELECT status FROM task_completion WHERE task_id = ? AND day = ?",
            params![task_id, day],
            |row| row.get(0),
        )
        .optional()?;

    if completed.is_some() {
        conn.execute(
            "DELETE FROM task_completion WHERE task_id = ? AND day = ?",
            params![task_id, day],
        )?;
    } else {
        conn.execute(
            "INSERT INTO task_completion (task_id, day, status, done_at) VALUES (?, ?, 1, unixepoch())",
            params![task_id, day]
        )?;
    }
    update_task_stats(conn, task_id)?;
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

pub fn get_month_view(conn: &Connection, year: i32, month: u32) -> Result<Vec<MonthViewDay>> {
    // 1. Determine the first day of the month
    let start_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();

    // 2. Determine the first day of the calendar grid (Monday)
    // weekday(): Mon=0, Sun=6
    let weekday_offset = start_of_month.weekday().num_days_from_monday() as i64;
    let grid_start_date = start_of_month - chrono::Duration::days(weekday_offset);

    // 3. Determine the end of the grid. Standard calendar view is often 6 rows (42 days)
    // to accommodate all months structure.
    // User requested "show only 4 weeks" but also "fill in all boxes".
    // A safe bet for a UI that scrolls is to always return 42 days (6 weeks)
    // or just enough to cover the month + padding.
    // Let's return 42 days (6 weeks) fixed to ensure consistent grid size.
    let grid_size = 28;
    let grid_end_date = grid_start_date + chrono::Duration::days(grid_size - 1);

    let start_ts = grid_start_date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let end_ts = grid_end_date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    let start_day = start_ts / 86400;
    let end_day = end_ts / 86400;

    let mut result = Vec::new();

    // Query: Get all schedules overlapping this GRID range + titles
    let mut stmt = conn.prepare(
        "SELECT s.task_id, s.effective_from, s.effective_to, s.type, s.weekday_mask, s.monthday, t.title, s.interval_days
         FROM task_schedule s
         JOIN task t ON s.task_id = t.id
         WHERE s.effective_from <= ? AND (s.effective_to IS NULL OR s.effective_to >= ?)",
    )?;

    struct Sched {
        task_id: i64,
        effective_from: i64,
        effective_to: Option<i64>,
        type_: String,
        weekday_mask: Option<i64>,
        monthday: Option<i64>,
        title: String,
        interval_days: Option<i64>,
    }

    let scheds = stmt.query_map(params![end_day, start_day], |row| {
        Ok(Sched {
            task_id: row.get(0)?,
            effective_from: row.get(1)?,
            effective_to: row.get(2)?,
            type_: row.get(3)?,
            weekday_mask: row.get(4)?,
            monthday: row.get(5)?,
            title: row.get(6)?,
            interval_days: row.get(7)?, // Add interval_days fetch
        })
    })?;

    let mut sched_list = Vec::new();
    for s in scheds {
        sched_list.push(s?);
    }

    // Get completions for the GRID range
    let mut stmt_comp =
        conn.prepare("SELECT task_id, day FROM task_completion WHERE day BETWEEN ? AND ?")?;

    let comps = stmt_comp.query_map(params![start_day, end_day], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut completions = std::collections::HashSet::new();
    for c in comps {
        completions.insert(c?);
    }

    for day in start_day..=end_day {
        let mut due_count = 0;
        let mut done_count = 0;

        // Convert day index back to date to check weekday/monthday
        let d = DateTime::from_timestamp(day * 86400, 0)
            .unwrap()
            .date_naive();
        let weekday_0 = d.weekday().num_days_from_monday() as i64; // Mon=0, Sun=6
        let day_of_month = d.day() as i64;

        let mut due_tasks = std::collections::HashSet::new();

        let mut daily_tasks = Vec::new();

        for s in &sched_list {
            if day < s.effective_from {
                continue;
            }
            if let Some(to) = s.effective_to {
                if day > to {
                    continue;
                }
            }

            let is_due = match s.type_.as_str() {
                "daily" => true,
                "weekly" => {
                    if let Some(mask) = s.weekday_mask {
                        (mask >> weekday_0) & 1 == 1
                    } else {
                        false
                    }
                }
                "monthly" => Some(day_of_month) == s.monthday,
                "custom" => {
                    if let Some(interval) = s.interval_days {
                        if interval > 0 && day >= s.effective_from {
                            (day - s.effective_from) % interval == 0
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if is_due {
                if !due_tasks.contains(&s.task_id) {
                    due_tasks.insert(s.task_id);
                    due_count += 1;

                    let is_done = completions.contains(&(s.task_id, day));
                    if is_done {
                        done_count += 1;
                    }

                    daily_tasks.push(MonthTask {
                        id: s.task_id,
                        title: s.title.clone(),
                        is_done,
                    });
                }
            }
        }

        // Sort tasks by ID or something stable
        daily_tasks.sort_by_key(|t| t.id);

        result.push(MonthViewDay {
            day,
            due_count,
            done_count,
            all_done: due_count > 0 && due_count == done_count,
            tasks: daily_tasks,
        });
    }

    Ok(result)
}

pub fn edit_task(
    conn: &Connection,
    task_id: i64,
    new_title: String,
    new_frequency_type: String,
    new_weekday_mask: Option<i64>,
    new_monthday: Option<i64>,
    new_interval_days: Option<i64>,
) -> Result<()> {
    conn.execute_batch("BEGIN TRANSACTION;")?;

    conn.execute(
        "UPDATE task SET title = ? WHERE id = ?",
        params![new_title, task_id],
    )?;

    let (current_id, current_type, current_mask, current_monthday, current_interval): (i64, String, Option<i64>, Option<i64>, Option<i64>) = conn.query_row(
        "SELECT id, type, weekday_mask, monthday, interval_days FROM task_schedule WHERE task_id = ? AND effective_to IS NULL",
        params![task_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    )?;

    let schedule_changed = current_type != new_frequency_type
        || current_mask != new_weekday_mask
        || current_monthday != new_monthday
        || current_interval != new_interval_days;

    if schedule_changed {
        let today = get_day_index();

        conn.execute(
            "UPDATE task_schedule SET effective_to = ? WHERE id = ?",
            params![today - 1, current_id],
        )?;

        conn.execute(
            "INSERT INTO task_schedule (task_id, effective_from, type, weekday_mask, monthday, interval_days)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                task_id,
                today,
                new_frequency_type,
                new_weekday_mask,
                new_monthday,
                new_interval_days
            ],
        )?;

        conn.execute(
            "UPDATE task_stats SET current_streak = 0, last_completed_day = NULL WHERE task_id = ?",
            params![task_id],
        )?;
    }

    conn.execute_batch("COMMIT;")?;
    Ok(())
}

fn is_task_due(schedule: &TaskSchedule, day: i64) -> bool {
    let d = DateTime::from_timestamp(day * 86400, 0)
        .unwrap()
        .date_naive();

    if day < schedule.effective_from {
        return false;
    }

    match schedule.type_.as_str() {
        "daily" => true,
        "weekly" => {
            if let Some(mask) = schedule.weekday_mask {
                let weekday = d.weekday().num_days_from_monday() as i64; // Mon=0
                (mask & (1 << weekday)) != 0
            } else {
                false
            }
        }
        "monthly" => {
            if let Some(mday) = schedule.monthday {
                d.day() as i64 == mday
            } else {
                false
            }
        }
        "custom" => {
            if let Some(interval) = schedule.interval_days {
                if interval <= 0 {
                    return false;
                } // Safety
                  // Task starts on effective_from. Repeats every `interval` days.
                  // Due if (day - start) >= 0 && (day - start) % interval == 0
                if day >= schedule.effective_from {
                    (day - schedule.effective_from) % interval == 0
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

fn update_task_stats(conn: &Connection, task_id: i64) -> Result<()> {
    let mut stmt =
        conn.prepare("SELECT * FROM task_schedule WHERE task_id = ? AND effective_to IS NULL")?;
    let schedule = stmt
        .query_row(params![task_id], |row| {
            Ok(TaskSchedule {
                id: row.get(0)?,
                task_id: row.get(1)?,
                effective_from: row.get(2)?,
                effective_to: row.get(3)?,
                type_: row.get(4)?,
                weekday_mask: row.get(5)?,
                monthday: row.get(6)?,
                interval_days: row.get(7)?,
                params_json: row.get(8)?,
            })
        })
        .optional()?;

    let schedule = match schedule {
        Some(s) => s,
        None => return Ok(()),
    };

    let mut stmt_comp =
        conn.prepare("SELECT day FROM task_completion WHERE task_id = ? ORDER BY day DESC")?;
    let completions_iter = stmt_comp.query_map(params![task_id], |row| row.get::<_, i64>(0))?;
    let mut completions = std::collections::HashSet::new();
    for c in completions_iter {
        completions.insert(c?);
    }

    let today = get_day_index();
    let mut current_streak = 0;

    let mut loop_day = today;
    if !completions.contains(&loop_day) {
        loop_day -= 1;
    }

    let min_day = schedule.effective_from;

    while loop_day >= min_day {
        if is_task_due(&schedule, loop_day) {
            if completions.contains(&loop_day) {
                current_streak += 1;
            } else {
                break;
            }
        }
        loop_day -= 1;
        if today - loop_day > 1000 {
            break;
        }
    }

    conn.execute(
        "UPDATE task_stats SET current_streak = ?, best_streak = MAX(best_streak, ?) WHERE task_id = ?",
        params![current_streak, current_streak, task_id],
    )?;

    Ok(())
}

fn check_week_perfect(conn: &Connection, monday_day_index: i64) -> Result<bool> {
    let start_day = monday_day_index;
    let end_day = monday_day_index + 6;

    // 1. Get Schedules overlapping this week
    let mut stmt = conn.prepare(
        "SELECT s.task_id, s.effective_from, s.effective_to, s.type, s.weekday_mask, s.monthday, s.interval_days
         FROM task_schedule s
         WHERE s.effective_from <= ? AND (s.effective_to IS NULL OR s.effective_to >= ?)",
    )?;

    // We need to fetch into a struct to use with is_task_due
    let scheds_iter = stmt.query_map(params![end_day, start_day], |row| {
        Ok(TaskSchedule {
            id: 0, // Not needed for is_task_due
            task_id: row.get(0)?,
            effective_from: row.get(1)?,
            effective_to: row.get(2)?,
            type_: row.get(3)?,
            weekday_mask: row.get(4)?,
            monthday: row.get(5)?,
            interval_days: row.get(6)?, // Retrieve interval
            params_json: None,
        })
    })?;

    let mut scheds = Vec::new();
    for s in scheds_iter {
        scheds.push(s?);
    }

    // 2. Get Completions in this week
    let mut stmt_comp =
        conn.prepare("SELECT task_id, day FROM task_completion WHERE day BETWEEN ? AND ?")?;
    let comps_iter = stmt_comp.query_map(params![start_day, end_day], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut completions = std::collections::HashSet::new();
    for c in comps_iter {
        completions.insert(c?);
    }

    // 3. Check every day
    let mut due_count = 0;

    for day in start_day..=end_day {
        for s in &scheds {
            if is_task_due(s, day) {
                due_count += 1;
                // If a task is due, it MUST be completed
                if !completions.contains(&(s.task_id, day)) {
                    return Ok(false);
                }
            }
        }
    }

    // Require activity to count as streak?
    // User logic: "increments ... when every task from monday to sunday is done"
    // If NO tasks, then this condition is trivially true (0 tasks, 0 done).
    // HOWEVER, usually streaks imply doing something.
    // But if the user takes a break week with NO tasks scheduled, maybe they want to keep the streak?
    // "If two weeks are perfectly done".
    // I will assume due_count > 0 is NOT required for now, to be safe with "every task ... is done" (vacuously true).
    // Wait, if I have 0 tasks, I have done every task.
    // If I return `true`, then a user with NO tasks gets infinite streak? That seems wrong.
    // Let's stick to `due_count > 0`.
    Ok(due_count > 0)
}

pub fn get_weekly_streak(conn: &Connection) -> Result<i64> {
    let today = get_day_index();
    let d = DateTime::from_timestamp(today * 86400, 0)
        .unwrap()
        .date_naive();
    let weekday_offset = d.weekday().num_days_from_monday() as i64;
    let this_monday = today - weekday_offset;

    let mut streak = 0;

    // 1. Check current week
    if check_week_perfect(conn, this_monday)? {
        streak += 1;
    }

    // 2. Check past weeks
    let mut check_monday = this_monday - 7;
    for _ in 0..260 {
        // 5 years cap
        if check_monday < 0 {
            break;
        }

        let perfect = check_week_perfect(conn, check_monday)?;
        if perfect {
            streak += 1;
            check_monday -= 7;
        } else {
            break;
        }
    }

    Ok(streak)
}
