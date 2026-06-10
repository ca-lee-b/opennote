use sqlx::{Row, SqliteConnection};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub(super) async fn cleanup_completed_job_files(
    app: &AppHandle,
    job_id: &str,
) -> Result<(), String> {
    let mut connection = crate::db::connect(app).await?;
    let row = sqlx::query(
        "SELECT source_audio_path, save_audio
         FROM recording_processing_jobs
         WHERE id = ?",
    )
    .bind(job_id)
    .fetch_optional(&mut connection)
    .await
    .map_err(|error| format!("Failed to load completed job cleanup metadata: {error}"))?;

    let Some(row) = row else {
        return Ok(());
    };
    let source_audio_path: String = row.get("source_audio_path");
    let save_audio = row.get::<i64, _>("save_audio") != 0;
    let chunk_dir = chunk_dir(app, job_id)?;
    let mut paths = vec![chunk_dir];
    if !save_audio {
        paths.push(PathBuf::from(source_audio_path));
    }
    cleanup_paths(paths);
    Ok(())
}

pub(super) async fn collect_recording_audio_paths(
    app: &AppHandle,
    connection: &mut SqliteConnection,
    recording_id: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    if let Some(row) = sqlx::query("SELECT audio_path FROM recordings WHERE id = ? LIMIT 1")
        .bind(recording_id)
        .fetch_optional(&mut *connection)
        .await
        .map_err(|error| format!("Failed to load recording audio path: {error}"))?
    {
        if let Some(path) = row.get::<Option<String>, _>("audio_path") {
            paths.push(PathBuf::from(path));
        }
    }
    let rows = sqlx::query(
        "SELECT id, source_audio_path FROM recording_processing_jobs WHERE recording_id = ?",
    )
    .bind(recording_id)
    .fetch_all(&mut *connection)
    .await
    .map_err(|error| format!("Failed to load recording processing paths: {error}"))?;
    for row in rows {
        let job_id: String = row.get("id");
        let source_audio_path: String = row.get("source_audio_path");
        paths.push(PathBuf::from(source_audio_path));
        paths.push(chunk_dir(app, &job_id)?);
    }
    Ok(paths)
}

pub(super) fn cleanup_paths(paths: Vec<PathBuf>) {
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        if path.as_os_str().is_empty() || !seen.insert(path.display().to_string()) {
            continue;
        }
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
}

pub(super) fn chunk_dir(app: &AppHandle, job_id: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data dir: {error}"))?
        .join("audio")
        .join("chunks")
        .join(job_id))
}
