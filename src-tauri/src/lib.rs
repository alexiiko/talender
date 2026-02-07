mod db;

use tauri::{Manager, State};

// TODO: Replace with your actual Turso database URL and auth token
const TURSO_DATABASE_URL: &str = "libsql://talender-alexiko.aws-eu-west-1.turso.io";
const TURSO_AUTH_TOKEN: &str = "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJhIjoicnciLCJpYXQiOjE3NzA0NTY4NzEsImlkIjoiM2Y0MWY3NWItNzZjNC00MzFhLThlNzQtNTdkM2MxODE1NDE5IiwicmlkIjoiZmEyNWI5Y2YtMTU2OC00ZGIyLTkyOTUtZDhiYzNmMzljZTJlIn0.stm6IjJaZE1O0PoH-dOc6WX-4JfkS24FIiMtWMjKruKBzMp7Bc6SvMjPuUUiWjVpImKMuhRBPiJr7gnSSgIeDA";

#[tauri::command]
async fn get_month_view(
    state: State<'_, db::AppState>,
    year: i32,
    month: u32,
) -> Result<Vec<db::MonthViewDay>, String> {
    let conn = state.db.lock().await;
    db::get_month_view(&conn, year, month)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn edit_task(
    state: State<'_, db::AppState>,
    task_id: i64,
    new_title: String,
    new_frequency_type: String,
    new_weekday_mask: Option<i64>,
    new_monthday: Option<i64>,
    new_interval_days: Option<i64>,
) -> Result<(), String> {
    let conn = state.db.lock().await;
    db::edit_task(
        &conn,
        task_id,
        new_title,
        new_frequency_type,
        new_weekday_mask,
        new_monthday,
        new_interval_days,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_task(
    state: State<'_, db::AppState>,
    title: String,
    frequency_type: String,
    weekday_mask: Option<i64>,
    monthday: Option<i64>,
    interval_days: Option<i64>,
) -> Result<(), String> {
    let conn = state.db.lock().await;
    db::add_task(
        &conn,
        title,
        frequency_type,
        weekday_mask,
        monthday,
        interval_days,
    )
    .await
    .map_err(|e| {
        eprintln!("add_task error: {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
async fn list_tasks(
    state: State<'_, db::AppState>,
    day: Option<i64>,
) -> Result<Vec<db::TaskWithStats>, String> {
    let conn = state.db.lock().await;
    db::list_tasks(&conn, day).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_task(state: State<'_, db::AppState>, task_id: i64) -> Result<(), String> {
    let conn = state.db.lock().await;
    db::delete_task(&conn, task_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_completion(
    state: State<'_, db::AppState>,
    task_id: i64,
    day: i64,
) -> Result<(), String> {
    let conn = state.db.lock().await;
    db::toggle_completion(&conn, task_id, day)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_all_tasks(state: State<'_, db::AppState>) -> Result<(), String> {
    let conn = state.db.lock().await;
    db::delete_all_tasks(&conn).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_weekly_streak(state: State<'_, db::AppState>) -> Result<i64, String> {
    let conn = state.db.lock().await;
    db::get_weekly_streak(&conn)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                let handle = tauri::async_runtime::spawn(async move {
                    db::init_db(TURSO_DATABASE_URL, TURSO_AUTH_TOKEN).await
                });
                let conn = handle
                    .await
                    .expect("task failed")
                    .expect("failed to init db");
                app.manage(db::AppState {
                    db: tokio::sync::Mutex::new(conn),
                });
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
