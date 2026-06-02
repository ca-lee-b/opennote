use serde::{Deserialize, Serialize};
use sqlx::{Connection, SqliteConnection};
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecordingWithSegmentsRequest {
    audio_path: Option<String>,
    created_at: String,
    duration: f64,
    id: String,
    is_partial: bool,
    language: Option<String>,
    model_id: String,
    segments: Vec<TranscriptSegment>,
    title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    end_time_secs: f64,
    start_time_secs: f64,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecordingWithSegmentsResult {
    full_text: String,
}

#[tauri::command]
pub async fn create_recording_with_segments(
    app: AppHandle,
    request: CreateRecordingWithSegmentsRequest,
) -> Result<CreateRecordingWithSegmentsResult, String> {
    let db_path = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to resolve app config dir: {error}"))?
        .join("opennote.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let mut connection = SqliteConnection::connect(&database_url)
        .await
        .map_err(|error| format!("Failed to connect to recordings database: {error}"))?;
    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to start recordings transaction: {error}"))?;
    let full_text = request
        .segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    sqlx::query(
        "INSERT INTO recordings (
            id, title, created_at, duration, model_id, is_partial, audio_path, language, full_text
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&request.id)
    .bind(&request.title)
    .bind(&request.created_at)
    .bind(request.duration)
    .bind(&request.model_id)
    .bind(request.is_partial)
    .bind(&request.audio_path)
    .bind(&request.language)
    .bind(&full_text)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to save recording: {error}"))?;

    for (index, segment) in request.segments.iter().enumerate() {
        if segment.text.trim().is_empty() {
            continue;
        }
        sqlx::query(
            "INSERT INTO transcript_lines (
                id, recording_id, line_id, text, start_time, start_time_secs, end_time_secs,
                duration, sort_order, is_final
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&request.id)
        .bind(index as i64 + 1)
        .bind(&segment.text)
        .bind(format_timestamp(segment.start_time_secs))
        .bind(segment.start_time_secs)
        .bind(segment.end_time_secs)
        .bind((segment.end_time_secs - segment.start_time_secs).max(0.0))
        .bind(index as i64)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to save transcript segment: {error}"))?;
    }

    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit recording: {error}"))?;
    Ok(CreateRecordingWithSegmentsResult { full_text })
}

fn format_timestamp(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::format_timestamp;

    #[test]
    fn formats_absolute_segment_offset() {
        assert_eq!(format_timestamp(0.0), "0:00");
        assert_eq!(format_timestamp(65.9), "1:05");
    }

    #[test]
    fn clamps_negative_segment_offset() {
        assert_eq!(format_timestamp(-1.0), "0:00");
    }
}
