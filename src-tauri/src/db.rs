use sqlx::{Connection, SqliteConnection};
use tauri::{AppHandle, Manager};

pub fn database_url(app: &AppHandle) -> Result<String, String> {
    let db_path = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve app config dir: {error}"))?
        .join("opennote.db");
    Ok(format!("sqlite:{}", db_path.display()))
}

pub async fn connect(app: &AppHandle) -> Result<SqliteConnection, String> {
    SqliteConnection::connect(&database_url(app)?)
        .await
        .map_err(|error| format!("Failed to connect to OpenNote database: {error}"))
}
