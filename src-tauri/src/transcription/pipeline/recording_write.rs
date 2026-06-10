use super::model::{
    chunk_line_base, format_timestamp, now_rfc3339, ChunkWork, CHUNK_COMPLETED, JOB_QUEUED,
};
use crate::transcription::worker::TranscriptionSegment;
use sqlx::{Connection, Row, SqliteConnection};

pub(super) struct EnqueueRecordingWrite {
    pub(super) audio_path: String,
    pub(super) created_at: String,
    pub(super) duration: f64,
    pub(super) model_id: String,
    pub(super) save_audio: bool,
    pub(super) title: String,
}

pub(super) struct EnqueueRecordingWriteResult {
    pub(super) job_id: String,
    pub(super) recording_id: String,
}

pub(super) struct RecordingWrite<'a> {
    connection: &'a mut SqliteConnection,
}

impl<'a> RecordingWrite<'a> {
    pub(super) fn new(connection: &'a mut SqliteConnection) -> Self {
        Self { connection }
    }

    pub(super) async fn enqueue_recording(
        &mut self,
        input: EnqueueRecordingWrite,
    ) -> Result<EnqueueRecordingWriteResult, String> {
        let recording_id = uuid::Uuid::new_v4().to_string();
        let job_id = uuid::Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let mut transaction =
            self.connection.begin().await.map_err(|error| {
                format!("Failed to start recording enqueue transaction: {error}")
            })?;

        sqlx::query(
            "INSERT INTO recordings (
                id, title, created_at, duration, model_id, is_partial, audio_path, language, full_text
            ) VALUES (?, ?, ?, ?, ?, 1, ?, NULL, '')",
        )
        .bind(&recording_id)
        .bind(input.title)
        .bind(input.created_at)
        .bind(input.duration.max(0.0))
        .bind(&input.model_id)
        .bind(if input.save_audio {
            Some(input.audio_path.clone())
        } else {
            None
        })
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to create processing recording: {error}"))?;

        sqlx::query(
            "INSERT INTO recording_processing_jobs (
                id, recording_id, model_id, source_audio_path, save_audio, status,
                total_chunks, completed_chunks, failed_chunks, error, created_at, updated_at, completed_at
            ) VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0, NULL, ?, ?, NULL)",
        )
        .bind(&job_id)
        .bind(&recording_id)
        .bind(&input.model_id)
        .bind(input.audio_path)
        .bind(input.save_audio)
        .bind(JOB_QUEUED)
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to create recording processing job: {error}"))?;

        transaction
            .commit()
            .await
            .map_err(|error| format!("Failed to commit recording enqueue: {error}"))?;

        Ok(EnqueueRecordingWriteResult {
            job_id,
            recording_id,
        })
    }

    pub(super) async fn complete_chunk(
        &mut self,
        work: &ChunkWork,
        segments: &[TranscriptionSegment],
    ) -> Result<(), String> {
        let now = now_rfc3339();
        let transcript_json = serde_json::to_string(segments)
            .map_err(|error| format!("Failed to serialize chunk transcript: {error}"))?;
        let mut transaction =
            self.connection.begin().await.map_err(|error| {
                format!("Failed to start chunk completion transaction: {error}")
            })?;

        sqlx::query(
            "DELETE FROM transcript_lines
             WHERE recording_id = ? AND line_id >= ? AND line_id < ?",
        )
        .bind(&work.recording_id)
        .bind(chunk_line_base(work.chunk_index))
        .bind(chunk_line_base(work.chunk_index + 1))
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to replace chunk transcript lines: {error}"))?;

        for (index, segment) in segments.iter().enumerate() {
            if segment.text.trim().is_empty() {
                continue;
            }
            let line_id = chunk_line_base(work.chunk_index) + index as i64 + 1;
            let duration = (segment.end_time_secs - segment.start_time_secs).max(0.0);
            sqlx::query(
                "INSERT INTO transcript_lines (
                    id, recording_id, line_id, text, start_time, start_time_secs, end_time_secs,
                    duration, sort_order, is_final
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&work.recording_id)
            .bind(line_id)
            .bind(segment.text.trim())
            .bind(format_timestamp(segment.start_time_secs))
            .bind(segment.start_time_secs)
            .bind(segment.end_time_secs)
            .bind(duration)
            .bind(line_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| format!("Failed to insert chunk transcript line: {error}"))?;
        }

        sqlx::query(
            "UPDATE recording_processing_chunks
             SET status = ?, transcript_json = ?, error = NULL, updated_at = ?
             WHERE id = ?",
        )
        .bind(CHUNK_COMPLETED)
        .bind(transcript_json)
        .bind(&now)
        .bind(&work.chunk_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to mark chunk completed: {error}"))?;

        rebuild_full_text_in_transaction(&mut transaction, &work.recording_id).await?;
        transaction
            .commit()
            .await
            .map_err(|error| format!("Failed to commit chunk completion: {error}"))?;
        Ok(())
    }
}

async fn rebuild_full_text_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    recording_id: &str,
) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT text FROM transcript_lines WHERE recording_id = ? ORDER BY sort_order ASC",
    )
    .bind(recording_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(|error| format!("Failed to load transcript lines for full text: {error}"))?;
    let full_text = rows
        .into_iter()
        .filter_map(|row| {
            let text: String = row.get("text");
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    sqlx::query("UPDATE recordings SET full_text = ? WHERE id = ?")
        .bind(full_text)
        .bind(recording_id)
        .execute(&mut **transaction)
        .await
        .map_err(|error| format!("Failed to rebuild recording full text: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::RecordingWrite;
    use crate::transcription::pipeline::model::{ChunkWork, CHUNK_COMPLETED, JOB_QUEUED};
    use crate::transcription::worker::TranscriptionSegment;
    use sqlx::{Connection, Row, SqliteConnection};

    async fn memory_connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:")
            .await
            .expect("memory sqlite connection")
    }

    async fn create_recording_write_tables(connection: &mut SqliteConnection) {
        sqlx::query(
            "CREATE TABLE recordings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                duration REAL NOT NULL DEFAULT 0,
                model_id TEXT NOT NULL DEFAULT '',
                is_partial INTEGER NOT NULL DEFAULT 0,
                audio_path TEXT,
                language TEXT,
                full_text TEXT NOT NULL DEFAULT ''
            )",
        )
        .execute(&mut *connection)
        .await
        .expect("create recordings");
        sqlx::query(
            "CREATE TABLE recording_processing_jobs (
                id TEXT PRIMARY KEY,
                recording_id TEXT NOT NULL UNIQUE,
                model_id TEXT NOT NULL,
                source_audio_path TEXT NOT NULL,
                save_audio INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL,
                total_chunks INTEGER NOT NULL DEFAULT 0,
                completed_chunks INTEGER NOT NULL DEFAULT 0,
                failed_chunks INTEGER NOT NULL DEFAULT 0,
                error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            )",
        )
        .execute(&mut *connection)
        .await
        .expect("create processing jobs");
        sqlx::query(
            "CREATE TABLE transcript_lines (
                id TEXT PRIMARY KEY,
                recording_id TEXT NOT NULL,
                line_id INTEGER NOT NULL,
                text TEXT NOT NULL DEFAULT '',
                start_time TEXT NOT NULL,
                start_time_secs REAL NOT NULL DEFAULT 0,
                end_time_secs REAL NOT NULL DEFAULT 0,
                duration REAL NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL,
                is_final INTEGER NOT NULL DEFAULT 1,
                UNIQUE(recording_id, line_id)
            )",
        )
        .execute(&mut *connection)
        .await
        .expect("create transcript lines");
        sqlx::query(
            "CREATE TABLE recording_processing_chunks (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                recording_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                logical_start_secs REAL NOT NULL,
                logical_end_secs REAL NOT NULL,
                source_start_secs REAL NOT NULL,
                source_end_secs REAL NOT NULL,
                chunk_path TEXT NOT NULL,
                status TEXT NOT NULL,
                transcript_json TEXT NOT NULL DEFAULT '[]',
                error TEXT,
                attempt_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(job_id, chunk_index)
            )",
        )
        .execute(&mut *connection)
        .await
        .expect("create processing chunks");
    }

    #[test]
    fn enqueue_creates_partial_recording_and_queued_job() {
        tauri::async_runtime::block_on(async {
            let mut connection = memory_connection().await;
            create_recording_write_tables(&mut connection).await;

            let result = RecordingWrite::new(&mut connection)
                .enqueue_recording(super::EnqueueRecordingWrite {
                    audio_path: "/tmp/recording.wav".to_string(),
                    created_at: "2026-06-07T20:00:00Z".to_string(),
                    duration: 42.5,
                    model_id: "whisper-base-en".to_string(),
                    save_audio: false,
                    title: "Recording - Jun 7".to_string(),
                })
                .await
                .expect("enqueue recording");

            let recording = sqlx::query(
                "SELECT title, created_at, duration, model_id, is_partial, audio_path, full_text
                 FROM recordings
                 WHERE id = ?",
            )
            .bind(&result.recording_id)
            .fetch_one(&mut connection)
            .await
            .expect("select recording");
            let job = sqlx::query(
                "SELECT recording_id, model_id, source_audio_path, save_audio, status, total_chunks, completed_chunks, failed_chunks, error
                 FROM recording_processing_jobs
                 WHERE id = ?",
            )
            .bind(&result.job_id)
            .fetch_one(&mut connection)
            .await
            .expect("select processing job");

            assert_eq!(recording.get::<String, _>("title"), "Recording - Jun 7");
            assert_eq!(
                recording.get::<String, _>("created_at"),
                "2026-06-07T20:00:00Z"
            );
            assert_eq!(recording.get::<f64, _>("duration"), 42.5);
            assert_eq!(recording.get::<String, _>("model_id"), "whisper-base-en");
            assert_eq!(recording.get::<i64, _>("is_partial"), 1);
            assert_eq!(recording.get::<Option<String>, _>("audio_path"), None);
            assert_eq!(recording.get::<String, _>("full_text"), "");

            assert_eq!(job.get::<String, _>("recording_id"), result.recording_id);
            assert_eq!(job.get::<String, _>("model_id"), "whisper-base-en");
            assert_eq!(
                job.get::<String, _>("source_audio_path"),
                "/tmp/recording.wav"
            );
            assert_eq!(job.get::<i64, _>("save_audio"), 0);
            assert_eq!(job.get::<String, _>("status"), JOB_QUEUED);
            assert_eq!(job.get::<i64, _>("total_chunks"), 0);
            assert_eq!(job.get::<i64, _>("completed_chunks"), 0);
            assert_eq!(job.get::<i64, _>("failed_chunks"), 0);
            assert_eq!(job.get::<Option<String>, _>("error"), None);
        });
    }

    #[test]
    fn complete_chunk_replaces_chunk_lines_and_rebuilds_full_text() {
        tauri::async_runtime::block_on(async {
            let mut connection = memory_connection().await;
            create_recording_write_tables(&mut connection).await;
            sqlx::query(
                "INSERT INTO recordings (id, title, created_at, duration, model_id, full_text)
                 VALUES ('recording-1', 'Recording', '2026-06-07T20:00:00Z', 20, 'model', 'old text')",
            )
            .execute(&mut connection)
            .await
            .expect("insert recording");
            sqlx::query(
                "INSERT INTO recording_processing_chunks (
                    id, job_id, recording_id, chunk_index, logical_start_secs, logical_end_secs,
                    source_start_secs, source_end_secs, chunk_path, status, created_at, updated_at
                 ) VALUES ('chunk-1', 'job-1', 'recording-1', 1, 300, 600, 285, 615, '/tmp/chunk.wav', 'transcribing', 'now', 'now')",
            )
            .execute(&mut connection)
            .await
            .expect("insert chunk");
            sqlx::query(
                "INSERT INTO transcript_lines (
                    id, recording_id, line_id, text, start_time, start_time_secs, end_time_secs, duration, sort_order, is_final
                 ) VALUES
                    ('line-older', 'recording-1', 1, 'Before', '0:01', 1, 2, 1, 1, 1),
                    ('line-old-chunk', 'recording-1', 10001, 'Old chunk text', '5:01', 301, 302, 1, 10001, 1),
                    ('line-later', 'recording-1', 20001, 'After', '10:01', 601, 602, 1, 20001, 1)",
            )
            .execute(&mut connection)
            .await
            .expect("insert existing lines");

            let work = ChunkWork {
                chunk_id: "chunk-1".to_string(),
                chunk_index: 1,
                chunk_path: "/tmp/chunk.wav".to_string(),
                job_id: "job-1".to_string(),
                logical_end_secs: 600.0,
                logical_start_secs: 300.0,
                model_id: "model".to_string(),
                recording_id: "recording-1".to_string(),
                source_start_secs: 285.0,
            };
            RecordingWrite::new(&mut connection)
                .complete_chunk(
                    &work,
                    &[
                        TranscriptionSegment {
                            text: " First replacement ".to_string(),
                            start_time_secs: 301.0,
                            end_time_secs: 303.5,
                        },
                        TranscriptionSegment {
                            text: " ".to_string(),
                            start_time_secs: 304.0,
                            end_time_secs: 305.0,
                        },
                        TranscriptionSegment {
                            text: "Second replacement".to_string(),
                            start_time_secs: 306.0,
                            end_time_secs: 308.0,
                        },
                    ],
                )
                .await
                .expect("complete chunk");

            let lines = sqlx::query(
                "SELECT line_id, text, start_time, start_time_secs, end_time_secs, duration, sort_order
                 FROM transcript_lines
                 WHERE recording_id = 'recording-1'
                 ORDER BY sort_order",
            )
            .fetch_all(&mut connection)
            .await
            .expect("select lines");
            let chunk = sqlx::query(
                "SELECT status, error, transcript_json FROM recording_processing_chunks WHERE id = 'chunk-1'",
            )
            .fetch_one(&mut connection)
            .await
            .expect("select chunk");
            let full_text =
                sqlx::query("SELECT full_text FROM recordings WHERE id = 'recording-1'")
                    .fetch_one(&mut connection)
                    .await
                    .expect("select recording")
                    .get::<String, _>("full_text");

            assert_eq!(lines.len(), 4);
            assert_eq!(lines[0].get::<i64, _>("line_id"), 1);
            assert_eq!(lines[0].get::<String, _>("text"), "Before");
            assert_eq!(lines[1].get::<i64, _>("line_id"), 10001);
            assert_eq!(lines[1].get::<String, _>("text"), "First replacement");
            assert_eq!(lines[1].get::<String, _>("start_time"), "5:01");
            assert_eq!(lines[1].get::<f64, _>("start_time_secs"), 301.0);
            assert_eq!(lines[1].get::<f64, _>("end_time_secs"), 303.5);
            assert_eq!(lines[1].get::<f64, _>("duration"), 2.5);
            assert_eq!(lines[1].get::<i64, _>("sort_order"), 10001);
            assert_eq!(lines[2].get::<i64, _>("line_id"), 10003);
            assert_eq!(lines[2].get::<String, _>("text"), "Second replacement");
            assert_eq!(lines[3].get::<i64, _>("line_id"), 20001);
            assert_eq!(lines[3].get::<String, _>("text"), "After");
            assert_eq!(chunk.get::<String, _>("status"), CHUNK_COMPLETED);
            assert_eq!(chunk.get::<Option<String>, _>("error"), None);
            assert!(chunk
                .get::<String, _>("transcript_json")
                .contains("Second replacement"));
            assert_eq!(
                full_text,
                "Before\n\nFirst replacement\n\nSecond replacement\n\nAfter"
            );
        });
    }
}
