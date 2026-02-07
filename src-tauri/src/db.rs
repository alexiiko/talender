use chrono::prelude::*;
use libsql::{Builder, Connection, Result};

pub struct AppState {
    pub db: tokio::sync::Mutex<Connection>,
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

pub async fn init_db(url: &str, token: &str) -> Result<Connection> {
    let db = Builder::new_remote(url.to_string(), token.to_string())
        .build()
        .await
        .expect("Failed to build db");
    let conn = db.connect().expect("Failed to connect to db");
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
    )
    .await?;

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

pub async fn add_task(
    conn: &Connection,
    title: String,
    frequency_type: String,
    weekday_mask: Option<i64>,
    monthday: Option<i64>,
    interval_days: Option<i64>,
) -> Result<()> {
    let day_index = get_day_index();
    let created_at = Utc::now().timestamp();

    conn.execute(
        "INSERT INTO task (title, created_at) VALUES (?, ?)",
        (title, created_at),
    )
    .await?;

    let mut rows = conn.query("SELECT last_insert_rowid()", ()).await?;
    let task_id: i64 = if let Some(row) = rows.next().await? {
        row.get(0)?
    } else {
        0
    };

    // Use positional parameters and handle NULLs properly
    conn.execute(
        "INSERT INTO task_schedule (task_id, effective_from, type, weekday_mask, monthday, interval_days)
            VALUES (?, ?, ?, ?, ?, ?)",
        (
            task_id,
            day_index,
            frequency_type,
            weekday_mask,
            monthday,
            interval_days,
        ),
    ).await?;

    conn.execute(
        "INSERT INTO task_stats (task_id, current_streak, best_streak) VALUES (?, 0, 0)",
        [task_id],
    )
    .await?;

    Ok(())
}

pub async fn list_tasks(conn: &Connection, day: Option<i64>) -> Result<Vec<TaskWithStats>> {
    let target_day = day.unwrap_or_else(get_day_index);

    let mut rows = conn.query(
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
            ORDER BY t.created_at DESC",
            [target_day],
    ).await?;

    let mut tasks = Vec::new();

    while let Some(row) = rows.next().await? {
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

        tasks.push(TaskWithStats {
            task,
            schedule,
            current_streak,
            best_streak,
            today_status: status.is_some(),
        });
    }

    Ok(tasks)
}

pub async fn delete_task(conn: &Connection, task_id: i64) -> Result<()> {
    conn.execute("DELETE FROM task WHERE id = ?", [task_id])
        .await?;
    Ok(())
}

pub async fn delete_all_tasks(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM task", ()).await?;
    Ok(())
}

pub async fn toggle_completion(conn: &Connection, task_id: i64, day: i64) -> Result<()> {
    let mut rows = conn
        .query(
            "SELECT status FROM task_completion WHERE task_id = ? AND day = ?",
            (task_id, day),
        )
        .await?;
    let completed = rows.next().await?;

    if completed.is_some() {
        conn.execute(
            "DELETE FROM task_completion WHERE task_id = ? AND day = ?",
            (task_id, day),
        )
        .await?;
    } else {
        conn.execute(
            "INSERT INTO task_completion (task_id, day, status, done_at) VALUES (?, ?, 1, unixepoch())",
            (task_id, day),
        ).await?;
    }
    update_task_stats(conn, task_id).await?;
    Ok(())
}

pub async fn get_month_view(conn: &Connection, year: i32, month: u32) -> Result<Vec<MonthViewDay>> {
    let start_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let weekday_offset = start_of_month.weekday().num_days_from_monday() as i64;
    let grid_start_date = start_of_month - chrono::Duration::days(weekday_offset);
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

    let mut rows = conn.query(
        "SELECT s.task_id, s.effective_from, s.effective_to, s.type, s.weekday_mask, s.monthday, t.title, s.interval_days
        FROM task_schedule s
        JOIN task t ON s.task_id = t.id
        WHERE s.effective_from <= ? AND (s.effective_to IS NULL OR s.effective_to >= ?)",
        (end_day, start_day),
    ).await?;

    let mut sched_list = Vec::new();
    while let Some(row) = rows.next().await? {
        sched_list.push(Sched {
            task_id: row.get(0)?,
            effective_from: row.get(1)?,
            effective_to: row.get(2)?,
            type_: row.get(3)?,
            weekday_mask: row.get(4)?,
            monthday: row.get(5)?,
            title: row.get(6)?,
            interval_days: row.get(7)?,
        });
    }

    let mut rows_comp = conn
        .query(
            "SELECT task_id, day FROM task_completion WHERE day BETWEEN ? AND ?",
            (start_day, end_day),
        )
        .await?;
    let mut completions = std::collections::HashSet::new();
    while let Some(row) = rows_comp.next().await? {
        let tid: i64 = row.get(0)?;
        let d: i64 = row.get(1)?;
        completions.insert((tid, d));
    }

    let mut result = Vec::new();

    for day in start_day..=end_day {
        let mut due_count = 0;
        let mut done_count = 0;

        let d = DateTime::from_timestamp(day * 86400, 0)
            .unwrap()
            .date_naive();
        let weekday_0 = d.weekday().num_days_from_monday() as i64;
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

pub async fn edit_task(
    conn: &Connection,
    task_id: i64,
    new_title: String,
    new_frequency_type: String,
    new_weekday_mask: Option<i64>,
    new_monthday: Option<i64>,
    new_interval_days: Option<i64>,
) -> Result<()> {
    conn.execute(
        "UPDATE task SET title = ? WHERE id = ?",
        (new_title.clone(), task_id),
    )
    .await?;

    let mut rows = conn.query(
        "SELECT id, type, weekday_mask, monthday, interval_days FROM task_schedule WHERE task_id = ? AND effective_to IS NULL",
        [task_id]
    ).await?;

    if let Some(row) = rows.next().await? {
        let current_id: i64 = row.get(0)?;
        let current_type: String = row.get(1)?;
        let current_mask: Option<i64> = row.get(2)?;
        let current_monthday: Option<i64> = row.get(3)?;
        let current_interval: Option<i64> = row.get(4)?;

        let schedule_changed = current_type != new_frequency_type
            || current_mask != new_weekday_mask
            || current_monthday != new_monthday
            || current_interval != new_interval_days;

        if schedule_changed {
            let today = get_day_index();

            conn.execute(
                "UPDATE task_schedule SET effective_to = ? WHERE id = ?",
                (today - 1, current_id),
            )
            .await?;

            conn.execute(
                "INSERT INTO task_schedule (task_id, effective_from, type, weekday_mask, monthday, interval_days)
                 VALUES (?, ?, ?, ?, ?, ?)",
                (
                    task_id,
                    today,
                    new_frequency_type,
                    new_weekday_mask,
                    new_monthday,
                    new_interval_days,
                ),
            ).await?;

            conn.execute(
                "UPDATE task_stats SET current_streak = 0, last_completed_day = NULL WHERE task_id = ?",
                [task_id],
            ).await?;
        }
    }

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
                let weekday = d.weekday().num_days_from_monday() as i64;
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
                }
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

async fn update_task_stats(conn: &Connection, task_id: i64) -> Result<()> {
    let mut rows = conn
        .query(
            "SELECT * FROM task_schedule WHERE task_id = ? AND effective_to IS NULL",
            [task_id],
        )
        .await?;

    let schedule = if let Some(row) = rows.next().await? {
        TaskSchedule {
            id: row.get(0)?,
            task_id: row.get(1)?,
            effective_from: row.get(2)?,
            effective_to: row.get(3)?,
            type_: row.get(4)?,
            weekday_mask: row.get(5)?,
            monthday: row.get(6)?,
            interval_days: row.get(7)?,
            params_json: row.get(8)?,
        }
    } else {
        return Ok(());
    };

    let mut rows_comp = conn
        .query(
            "SELECT day FROM task_completion WHERE task_id = ? ORDER BY day DESC",
            [task_id],
        )
        .await?;

    let mut completions = std::collections::HashSet::new();
    while let Some(row) = rows_comp.next().await? {
        completions.insert(row.get::<i64>(0)?);
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
        (current_streak, current_streak, task_id),
    ).await?;

    Ok(())
}

async fn check_week_perfect(conn: &Connection, monday_day_index: i64) -> Result<bool> {
    let start_day = monday_day_index;
    let end_day = monday_day_index + 6;

    let mut rows = conn.query(
        "SELECT s.task_id, s.effective_from, s.effective_to, s.type, s.weekday_mask, s.monthday, s.interval_days
            FROM task_schedule s
            WHERE s.effective_from <= ? AND (s.effective_to IS NULL OR s.effective_to >= ?)",
        (end_day, start_day),
    ).await?;

    let mut scheds = Vec::new();
    while let Some(row) = rows.next().await? {
        scheds.push(TaskSchedule {
            id: 0,
            task_id: row.get(0)?,
            effective_from: row.get(1)?,
            effective_to: row.get(2)?,
            type_: row.get(3)?,
            weekday_mask: row.get(4)?,
            monthday: row.get(5)?,
            interval_days: row.get(6)?,
            params_json: None,
        });
    }

    let mut rows_comp = conn
        .query(
            "SELECT task_id, day FROM task_completion WHERE day BETWEEN ? AND ?",
            (start_day, end_day),
        )
        .await?;
    let mut completions = std::collections::HashSet::new();
    while let Some(row) = rows_comp.next().await? {
        let tid: i64 = row.get(0)?;
        let d: i64 = row.get(1)?;
        completions.insert((tid, d));
    }

    let mut due_count = 0;

    for day in start_day..=end_day {
        for s in &scheds {
            if is_task_due(s, day) {
                due_count += 1;
                if !completions.contains(&(s.task_id, day)) {
                    return Ok(false);
                }
            }
        }
    }

    Ok(due_count > 0)
}

pub async fn get_weekly_streak(conn: &Connection) -> Result<i64> {
    let today = get_day_index();
    let d = DateTime::from_timestamp(today * 86400, 0)
        .unwrap()
        .date_naive();
    let weekday_offset = d.weekday().num_days_from_monday() as i64;
    let this_monday = today - weekday_offset;

    let mut streak = 0;

    if check_week_perfect(conn, this_monday).await? {
        streak += 1;
    }

    let mut check_monday = this_monday - 7;
    for _ in 0..260 {
        if check_monday < 0 {
            break;
        }

        let perfect = check_week_perfect(conn, check_monday).await?;
        if perfect {
            streak += 1;
            check_monday -= 7;
        } else {
            break;
        }
    }

    Ok(streak)
}
