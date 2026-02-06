mod db;

use std::sync::Mutex;
use tauri::{Manager, State};

#[tauri::command]
fn get_month_view(
    state: State<db::AppState>,
    year: i32,
    month: u32,
) -> Result<Vec<db::MonthViewDay>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "Failed to lock db".to_string())?;
    db::get_month_view(&conn, year, month).map_err(|e| e.to_string())
}

#[tauri::command]
fn edit_task(
    state: State<db::AppState>,
    task_id: i64,
    new_title: String,
    new_frequency_type: String,
    new_weekday_mask: Option<i64>,
    new_monthday: Option<i64>,
    new_interval_days: Option<i64>,
) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "Failed to lock db".to_string())?;
    db::edit_task(
        &conn,
        task_id,
        new_title,
        new_frequency_type,
        new_weekday_mask,
        new_monthday,
        new_interval_days,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_task(
    state: State<db::AppState>,
    title: String,
    frequency_type: String,
    weekday_mask: Option<i64>,
    monthday: Option<i64>,
    interval_days: Option<i64>,
) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "Failed to lock db".to_string())?;
    db::add_task(
        &conn,
        title,
        frequency_type,
        weekday_mask,
        monthday,
        interval_days,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tasks(
    state: State<db::AppState>,
    day: Option<i64>,
) -> Result<Vec<db::TaskWithStats>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "Failed to lock db".to_string())?;
    db::list_tasks(&conn, day).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(state: State<db::AppState>, task_id: i64) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "Failed to lock db".to_string())?;
    db::delete_task(&conn, task_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_completion(state: State<db::AppState>, task_id: i64, day: i64) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "Failed to lock db".to_string())?;
    db::toggle_completion(&conn, task_id, day).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_all_tasks(state: State<db::AppState>) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "Failed to lock db".to_string())?;
    db::delete_all_tasks(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_weekly_streak(state: State<db::AppState>) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_weekly_streak(&conn).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let conn = db::init_db(app.handle()).expect("failed to init db");
            app.manage(db::AppState {
                db: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_task,
            list_tasks,
            delete_task,
            delete_all_tasks,
            toggle_completion,
            get_month_view,
            edit_task,
            get_weekly_streak
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
