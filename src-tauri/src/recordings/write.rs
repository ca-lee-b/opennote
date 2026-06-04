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

struct RecordingWrite {
    request: CreateRecordingWithSegmentsRequest,
    full_text: String,
    lines: Vec<TranscriptLineWrite>,
}

struct TranscriptLineWrite {
    duration: f64,
    end_time_secs: f64,
    line_id: i64,
    sort_order: i64,
    start_time: String,
    start_time_secs: f64,
    text: String,
}

impl RecordingWrite {
    fn from_request(request: CreateRecordingWithSegmentsRequest) -> Self {
        let lines = request
            .segments
            .iter()
            .enumerate()
            .filter_map(|(index, segment)| TranscriptLineWrite::from_segment(index, segment))
            .collect::<Vec<_>>();
        let full_text = lines
            .iter()
            .map(|line| line.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        Self {
            request,
            full_text,
            lines,
        }
    }

    async fn commit(self, connection: &mut SqliteConnection) -> Result<String, String> {
        let mut transaction = connection
            .begin()
            .await
            .map_err(|error| format!("Failed to start recordings transaction: {error}"))?;

        sqlx::query(
            "INSERT INTO recordings (
                id, title, created_at, duration, model_id, is_partial, audio_path, language, full_text
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&self.request.id)
        .bind(&self.request.title)
        .bind(&self.request.created_at)
        .bind(self.request.duration)
        .bind(&self.request.model_id)
        .bind(self.request.is_partial)
        .bind(&self.request.audio_path)
        .bind(&self.request.language)
        .bind(&self.full_text)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to save recording: {error}"))?;

        for line in &self.lines {
            sqlx::query(
                "INSERT INTO transcript_lines (
                    id, recording_id, line_id, text, start_time, start_time_secs, end_time_secs,
                    duration, sort_order, is_final
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&self.request.id)
            .bind(line.line_id)
            .bind(&line.text)
            .bind(&line.start_time)
            .bind(line.start_time_secs)
            .bind(line.end_time_secs)
            .bind(line.duration)
            .bind(line.sort_order)
            .execute(&mut *transaction)
            .await
            .map_err(|error| format!("Failed to save transcript segment: {error}"))?;
        }

        transaction
            .commit()
            .await
            .map_err(|error| format!("Failed to commit recording: {error}"))?;

        Ok(self.full_text)
    }
}

impl TranscriptLineWrite {
    fn from_segment(index: usize, segment: &TranscriptSegment) -> Option<Self> {
        if segment.text.trim().is_empty() {
            return None;
        }

        Some(Self {
            duration: (segment.end_time_secs - segment.start_time_secs).max(0.0),
            end_time_secs: segment.end_time_secs,
            line_id: index as i64 + 1,
            sort_order: index as i64,
            start_time: format_timestamp(segment.start_time_secs),
            start_time_secs: segment.start_time_secs,
            text: segment.text.clone(),
        })
    }
}

pub(super) async fn create_recording_with_segments(
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
    let full_text = RecordingWrite::from_request(request)
        .commit(&mut connection)
        .await?;

    Ok(CreateRecordingWithSegmentsResult { full_text })
}

fn format_timestamp(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::{format_timestamp, RecordingWrite, TranscriptSegment};

    fn request_with_segments(
        segments: Vec<TranscriptSegment>,
    ) -> super::CreateRecordingWithSegmentsRequest {
        super::CreateRecordingWithSegmentsRequest {
            audio_path: Some("/tmp/audio.wav".to_string()),
            created_at: "2026-06-04T16:00:00Z".to_string(),
            duration: 10.0,
            id: "recording-1".to_string(),
            is_partial: false,
            language: None,
            model_id: "whisper_small_en_q5_1".to_string(),
            segments,
            title: "Recording".to_string(),
        }
    }

    #[test]
    fn formats_absolute_segment_offset() {
        assert_eq!(format_timestamp(0.0), "0:00");
        assert_eq!(format_timestamp(65.9), "1:05");
    }

    #[test]
    fn clamps_negative_segment_offset() {
        assert_eq!(format_timestamp(-1.0), "0:00");
    }

    #[test]
    fn normalizes_transcript_lines_and_full_text() {
        let write = RecordingWrite::from_request(request_with_segments(vec![
            TranscriptSegment {
                end_time_secs: 1.5,
                start_time_secs: 0.0,
                text: " First line ".to_string(),
            },
            TranscriptSegment {
                end_time_secs: 2.0,
                start_time_secs: 1.5,
                text: "   ".to_string(),
            },
            TranscriptSegment {
                end_time_secs: 4.0,
                start_time_secs: 2.2,
                text: "Second line".to_string(),
            },
        ]));

        assert_eq!(write.full_text, "First line\n\nSecond line");
        assert_eq!(write.lines.len(), 2);
        assert_eq!(write.lines[0].line_id, 1);
        assert_eq!(write.lines[0].sort_order, 0);
        assert_eq!(write.lines[0].start_time, "0:00");
        assert_eq!(write.lines[1].line_id, 3);
        assert_eq!(write.lines[1].sort_order, 2);
        assert_eq!(write.lines[1].start_time, "0:02");
    }

    #[test]
    fn clamps_negative_segment_duration() {
        let write = RecordingWrite::from_request(request_with_segments(vec![TranscriptSegment {
            end_time_secs: 2.0,
            start_time_secs: 3.0,
            text: "Backwards clock".to_string(),
        }]));

        assert_eq!(write.lines[0].duration, 0.0);
    }
}
