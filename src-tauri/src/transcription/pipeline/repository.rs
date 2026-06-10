use super::chunking::{plan_chunk_windows, write_chunk_wav};
use super::cleanup::{
    chunk_dir, cleanup_completed_job_files, cleanup_paths, collect_recording_audio_paths,
};
use super::model::{
    now_rfc3339, ChunkWork, EnqueueRecordingTranscriptionRequest,
    EnqueueRecordingTranscriptionResult, JobRow, RecordingProcessingStatus, CHUNK_CANCELLED,
    CHUNK_COMPLETED, CHUNK_FAILED, CHUNK_QUEUED, CHUNK_TRANSCRIBING, JOB_CANCELLED, JOB_CHUNKING,
    JOB_COMPLETE, JOB_FAILED, JOB_INTERRUPTED, JOB_PARTIAL, JOB_QUEUED, JOB_TRANSCRIBING,
};
use super::recording_write::{EnqueueRecordingWrite, RecordingWrite};
use crate::transcription::models;
use crate::transcription::whisper::{read_wav_samples, WHISPER_SAMPLE_RATE};
use crate::transcription::worker::TranscriptionSegment;
use sqlx::{Connection, Row, SqliteConnection};
use std::path::PathBuf;
use tauri::AppHandle;

pub(super) async fn list_processing_statuses(
    app: &AppHandle,
) -> Result<Vec<RecordingProcessingStatus>, String> {
    let mut connection = crate::db::connect(app).await?;
    let rows = sqlx::query(
        "SELECT id, recording_id, status, total_chunks, completed_chunks, failed_chunks, error, updated_at
         FROM recording_processing_jobs
         WHERE status != ?
         ORDER BY updated_at DESC",
    )
    .bind(JOB_CANCELLED)
    .fetch_all(&mut connection)
    .await
    .map_err(|error| format!("Failed to load recording processing statuses: {error}"))?;

    Ok(rows
        .into_iter()
        .map(|row| RecordingProcessingStatus {
            completed_chunks: row.get("completed_chunks"),
            error: row.get("error"),
            failed_chunks: row.get("failed_chunks"),
            job_id: row.get("id"),
            recording_id: row.get("recording_id"),
            status: row.get("status"),
            total_chunks: row.get("total_chunks"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub(super) async fn interrupt_stale_jobs(app: &AppHandle) -> Result<(), String> {
    let mut connection = crate::db::connect(app).await?;
    let now = now_rfc3339();
    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to start stale job transaction: {error}"))?;

    sqlx::query(
        "UPDATE recording_processing_jobs
         SET status = ?, error = COALESCE(error, 'Processing was interrupted when OpenNote closed.'), updated_at = ?
         WHERE status IN (?, ?, ?)",
    )
    .bind(JOB_INTERRUPTED)
    .bind(&now)
    .bind(JOB_QUEUED)
    .bind(JOB_CHUNKING)
    .bind(JOB_TRANSCRIBING)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to interrupt stale jobs: {error}"))?;

    sqlx::query(
        "UPDATE recording_processing_chunks
         SET status = ?, error = COALESCE(error, 'Processing was interrupted when OpenNote closed.'), updated_at = ?
         WHERE status IN (?, ?)",
    )
    .bind(super::model::CHUNK_INTERRUPTED)
    .bind(&now)
    .bind(CHUNK_QUEUED)
    .bind(CHUNK_TRANSCRIBING)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to interrupt stale chunks: {error}"))?;

    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit stale job interruption: {error}"))?;
    Ok(())
}

pub(super) async fn enqueue_recording(
    app: &AppHandle,
    request: EnqueueRecordingTranscriptionRequest,
) -> Result<EnqueueRecordingTranscriptionResult, String> {
    if request.audio_path.trim().is_empty() {
        return Err("No audio file was saved. Please try again.".to_string());
    }
    if request.model_id.trim().is_empty() {
        return Err("No transcription model was selected.".to_string());
    }
    models::resolve_downloaded_model(app, &request.model_id)?;

    let now = now_rfc3339();
    let created_at = request.started_at.unwrap_or_else(|| now.clone());
    let mut connection = crate::db::connect(app).await?;
    let result = RecordingWrite::new(&mut connection)
        .enqueue_recording(EnqueueRecordingWrite {
            audio_path: request.audio_path,
            created_at,
            duration: request.duration,
            model_id: request.model_id,
            save_audio: request.save_audio,
            title: request.title,
        })
        .await?;

    Ok(EnqueueRecordingTranscriptionResult {
        job_id: result.job_id,
        recording_id: result.recording_id,
    })
}

pub(super) async fn resume_recording(app: &AppHandle, recording_id: &str) -> Result<(), String> {
    let mut connection = crate::db::connect(app).await?;
    let job = sqlx::query(
        "SELECT id, model_id, status
         FROM recording_processing_jobs
         WHERE recording_id = ?
         LIMIT 1",
    )
    .bind(recording_id)
    .fetch_optional(&mut connection)
    .await
    .map_err(|error| format!("Failed to load recording processing job: {error}"))?
    .ok_or_else(|| "No processing job exists for this recording.".to_string())?;
    let job_id: String = job.get("id");
    let model_id: String = job.get("model_id");
    let status: String = job.get("status");
    if status == JOB_COMPLETE || status == JOB_CANCELLED {
        return Ok(());
    }
    models::resolve_downloaded_model(app, &model_id)?;

    let now = now_rfc3339();
    let has_chunks =
        sqlx::query("SELECT COUNT(*) AS count FROM recording_processing_chunks WHERE job_id = ?")
            .bind(&job_id)
            .fetch_one(&mut connection)
            .await
            .map_err(|error| format!("Failed to inspect recording chunks: {error}"))?
            .get::<i64, _>("count")
            > 0;

    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to start resume transaction: {error}"))?;
    sqlx::query(
        "UPDATE recording_processing_chunks
         SET status = ?, error = NULL, updated_at = ?
         WHERE job_id = ? AND status IN (?, ?, ?, ?)",
    )
    .bind(CHUNK_QUEUED)
    .bind(&now)
    .bind(&job_id)
    .bind(CHUNK_FAILED)
    .bind(super::model::CHUNK_INTERRUPTED)
    .bind(CHUNK_TRANSCRIBING)
    .bind(CHUNK_QUEUED)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to reset failed chunks: {error}"))?;
    sqlx::query(
        "UPDATE recording_processing_jobs
         SET status = ?, error = NULL, failed_chunks = 0, updated_at = ?
         WHERE id = ?",
    )
    .bind(if has_chunks {
        JOB_TRANSCRIBING
    } else {
        JOB_QUEUED
    })
    .bind(&now)
    .bind(&job_id)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to resume recording processing: {error}"))?;
    sqlx::query("UPDATE recordings SET is_partial = 1 WHERE id = ?")
        .bind(recording_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to mark recording incomplete: {error}"))?;

    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit recording resume: {error}"))?;
    Ok(())
}

pub(super) async fn delete_recording(app: &AppHandle, recording_id: &str) -> Result<(), String> {
    let mut connection = crate::db::connect(app).await?;
    let paths = collect_recording_audio_paths(app, &mut connection, recording_id).await?;
    let now = now_rfc3339();
    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to start delete transaction: {error}"))?;

    sqlx::query(
        "UPDATE recording_processing_jobs
         SET status = ?, updated_at = ?, completed_at = COALESCE(completed_at, ?)
         WHERE recording_id = ? AND status NOT IN (?, ?)",
    )
    .bind(JOB_CANCELLED)
    .bind(&now)
    .bind(&now)
    .bind(recording_id)
    .bind(JOB_COMPLETE)
    .bind(JOB_CANCELLED)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to cancel recording jobs: {error}"))?;

    sqlx::query(
        "UPDATE recording_processing_chunks
         SET status = ?, updated_at = ?
         WHERE recording_id = ? AND status NOT IN (?, ?, ?)",
    )
    .bind(CHUNK_CANCELLED)
    .bind(&now)
    .bind(recording_id)
    .bind(CHUNK_COMPLETED)
    .bind(CHUNK_FAILED)
    .bind(CHUNK_CANCELLED)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to cancel recording chunks: {error}"))?;

    sqlx::query("DELETE FROM recordings WHERE id = ?")
        .bind(recording_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to delete recording: {error}"))?;

    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit recording delete: {error}"))?;

    cleanup_paths(paths);
    Ok(())
}

pub(super) async fn process_next_chunking_job(app: &AppHandle) -> Result<bool, String> {
    let mut connection = crate::db::connect(app).await?;
    let Some(job) = claim_next_chunking_job(&mut connection).await? else {
        return Ok(false);
    };

    match create_chunks_for_job(app, &job).await {
        Ok(total_chunks) => {
            let now = now_rfc3339();
            sqlx::query(
                "UPDATE recording_processing_jobs
                 SET status = ?, total_chunks = ?, updated_at = ?
                 WHERE id = ? AND status != ?",
            )
            .bind(JOB_TRANSCRIBING)
            .bind(total_chunks)
            .bind(now)
            .bind(job.id)
            .bind(JOB_CANCELLED)
            .execute(&mut connection)
            .await
            .map_err(|error| format!("Failed to mark job ready for transcription: {error}"))?;
            Ok(true)
        }
        Err(error) => {
            mark_job_failed(&mut connection, &job.recording_id, &job.id, &error).await?;
            Err(error)
        }
    }
}

pub(super) async fn claim_next_transcription_chunk(
    app: &AppHandle,
) -> Result<Option<ChunkWork>, String> {
    let mut connection = crate::db::connect(app).await?;
    let Some(row) = sqlx::query(
        "SELECT
            c.id, c.job_id, c.recording_id, c.chunk_index, c.logical_start_secs,
            c.logical_end_secs, c.source_start_secs, c.chunk_path, j.model_id
         FROM recording_processing_chunks c
         INNER JOIN recording_processing_jobs j ON j.id = c.job_id
         WHERE c.status = ? AND j.status = ?
         ORDER BY j.created_at ASC, c.chunk_index ASC
         LIMIT 1",
    )
    .bind(CHUNK_QUEUED)
    .bind(JOB_TRANSCRIBING)
    .fetch_optional(&mut connection)
    .await
    .map_err(|error| format!("Failed to load next transcription chunk: {error}"))?
    else {
        return Ok(None);
    };

    let work = ChunkWork {
        chunk_id: row.get("id"),
        chunk_index: row.get("chunk_index"),
        chunk_path: row.get("chunk_path"),
        job_id: row.get("job_id"),
        logical_end_secs: row.get("logical_end_secs"),
        logical_start_secs: row.get("logical_start_secs"),
        model_id: row.get("model_id"),
        recording_id: row.get("recording_id"),
        source_start_secs: row.get("source_start_secs"),
    };

    let now = now_rfc3339();
    sqlx::query(
        "UPDATE recording_processing_chunks
         SET status = ?, attempt_count = attempt_count + 1, updated_at = ?
         WHERE id = ? AND status = ?",
    )
    .bind(CHUNK_TRANSCRIBING)
    .bind(now)
    .bind(&work.chunk_id)
    .bind(CHUNK_QUEUED)
    .execute(&mut connection)
    .await
    .map_err(|error| format!("Failed to claim transcription chunk: {error}"))?;
    Ok(Some(work))
}

pub(super) async fn complete_chunk(
    app: &AppHandle,
    work: &ChunkWork,
    segments: &[TranscriptionSegment],
) -> Result<(), String> {
    let mut connection = crate::db::connect(app).await?;
    RecordingWrite::new(&mut connection)
        .complete_chunk(work, segments)
        .await?;

    refresh_job_counts_and_maybe_finish(app, &work.job_id, &work.recording_id).await
}

pub(super) async fn fail_chunk(
    app: &AppHandle,
    work: &ChunkWork,
    error: String,
) -> Result<(), String> {
    let mut connection = crate::db::connect(app).await?;
    let now = now_rfc3339();
    sqlx::query(
        "UPDATE recording_processing_chunks
         SET status = ?, error = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(CHUNK_FAILED)
    .bind(&error)
    .bind(now)
    .bind(&work.chunk_id)
    .execute(&mut connection)
    .await
    .map_err(|db_error| format!("Failed to mark chunk failed: {db_error}"))?;
    refresh_job_counts_and_maybe_finish(app, &work.job_id, &work.recording_id).await
}

async fn claim_next_chunking_job(
    connection: &mut SqliteConnection,
) -> Result<Option<JobRow>, String> {
    let Some(row) = sqlx::query(
        "SELECT id, recording_id, source_audio_path
         FROM recording_processing_jobs
         WHERE status = ?
         ORDER BY created_at ASC
         LIMIT 1",
    )
    .bind(JOB_QUEUED)
    .fetch_optional(&mut *connection)
    .await
    .map_err(|error| format!("Failed to claim chunking job: {error}"))?
    else {
        return Ok(None);
    };
    let job = JobRow {
        id: row.get("id"),
        recording_id: row.get("recording_id"),
        source_audio_path: row.get("source_audio_path"),
    };
    let now = now_rfc3339();
    sqlx::query(
        "UPDATE recording_processing_jobs
         SET status = ?, updated_at = ?
         WHERE id = ? AND status = ?",
    )
    .bind(JOB_CHUNKING)
    .bind(now)
    .bind(&job.id)
    .bind(JOB_QUEUED)
    .execute(connection)
    .await
    .map_err(|error| format!("Failed to mark job as chunking: {error}"))?;
    Ok(Some(job))
}

async fn create_chunks_for_job(app: &AppHandle, job: &JobRow) -> Result<i64, String> {
    let source_path = PathBuf::from(&job.source_audio_path);
    let samples = read_wav_samples(&source_path)?;
    if samples.is_empty() {
        return Err("The recording audio file is empty.".to_string());
    }
    let duration_secs = samples.len() as f64 / WHISPER_SAMPLE_RATE;
    let windows = plan_chunk_windows(duration_secs);
    let chunk_dir = chunk_dir(app, &job.id)?;
    std::fs::create_dir_all(&chunk_dir)
        .map_err(|error| format!("Failed to create chunk directory: {error}"))?;

    let mut connection = crate::db::connect(app).await?;
    let now = now_rfc3339();
    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to start chunk transaction: {error}"))?;

    for window in &windows {
        let chunk_path = chunk_dir.join(format!("chunk-{:04}.wav", window.index));
        write_chunk_wav(
            &samples,
            window.source_start_secs,
            window.source_end_secs,
            &chunk_path,
        )?;
        sqlx::query(
            "INSERT INTO recording_processing_chunks (
                id, job_id, recording_id, chunk_index, logical_start_secs, logical_end_secs,
                source_start_secs, source_end_secs, chunk_path, status, transcript_json,
                error, attempt_count, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '[]', NULL, 0, ?, ?)
            ON CONFLICT(job_id, chunk_index) DO UPDATE SET
                logical_start_secs = excluded.logical_start_secs,
                logical_end_secs = excluded.logical_end_secs,
                source_start_secs = excluded.source_start_secs,
                source_end_secs = excluded.source_end_secs,
                chunk_path = excluded.chunk_path,
                status = excluded.status,
                error = NULL,
                updated_at = excluded.updated_at",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&job.id)
        .bind(&job.recording_id)
        .bind(window.index)
        .bind(window.logical_start_secs)
        .bind(window.logical_end_secs)
        .bind(window.source_start_secs)
        .bind(window.source_end_secs)
        .bind(chunk_path.display().to_string())
        .bind(CHUNK_QUEUED)
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to create chunk row: {error}"))?;
    }

    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit chunks: {error}"))?;
    Ok(windows.len() as i64)
}

async fn refresh_job_counts_and_maybe_finish(
    app: &AppHandle,
    job_id: &str,
    recording_id: &str,
) -> Result<(), String> {
    let mut connection = crate::db::connect(app).await?;
    let counts = sqlx::query(
        "SELECT
            SUM(CASE WHEN status = ? THEN 1 ELSE 0 END) AS completed,
            SUM(CASE WHEN status = ? THEN 1 ELSE 0 END) AS failed,
            SUM(CASE WHEN status IN (?, ?) THEN 1 ELSE 0 END) AS active,
            COUNT(*) AS total
         FROM recording_processing_chunks
         WHERE job_id = ?",
    )
    .bind(CHUNK_COMPLETED)
    .bind(CHUNK_FAILED)
    .bind(CHUNK_QUEUED)
    .bind(CHUNK_TRANSCRIBING)
    .bind(job_id)
    .fetch_one(&mut connection)
    .await
    .map_err(|error| format!("Failed to refresh job counts: {error}"))?;
    let completed = counts.get::<i64, _>("completed");
    let failed = counts.get::<i64, _>("failed");
    let active = counts.get::<i64, _>("active");
    let total = counts.get::<i64, _>("total");
    let now = now_rfc3339();

    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to start job refresh transaction: {error}"))?;

    let final_status = if total > 0 && active == 0 && failed == 0 {
        Some(JOB_COMPLETE)
    } else if total > 0 && active == 0 && failed > 0 {
        Some(JOB_PARTIAL)
    } else {
        None
    };

    if let Some(status) = final_status {
        let error = if status == JOB_PARTIAL {
            Some("One or more audio chunks failed to transcribe.".to_string())
        } else {
            None
        };
        sqlx::query(
            "UPDATE recording_processing_jobs
             SET status = ?, completed_chunks = ?, failed_chunks = ?, error = ?, updated_at = ?, completed_at = ?
             WHERE id = ? AND status != ?",
        )
        .bind(status)
        .bind(completed)
        .bind(failed)
        .bind(error)
        .bind(&now)
        .bind(&now)
        .bind(job_id)
        .bind(JOB_CANCELLED)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to finalize processing job: {error}"))?;
        sqlx::query("UPDATE recordings SET is_partial = ? WHERE id = ?")
            .bind(status != JOB_COMPLETE)
            .bind(recording_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| format!("Failed to update recording completion status: {error}"))?;
    } else {
        sqlx::query(
            "UPDATE recording_processing_jobs
             SET completed_chunks = ?, failed_chunks = ?, updated_at = ?
             WHERE id = ? AND status != ?",
        )
        .bind(completed)
        .bind(failed)
        .bind(&now)
        .bind(job_id)
        .bind(JOB_CANCELLED)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to update processing job counts: {error}"))?;
    }

    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit processing job counts: {error}"))?;

    if final_status == Some(JOB_COMPLETE) {
        cleanup_completed_job_files(app, job_id).await?;
    }
    Ok(())
}

async fn mark_job_failed(
    connection: &mut SqliteConnection,
    recording_id: &str,
    job_id: &str,
    error: &str,
) -> Result<(), String> {
    let now = now_rfc3339();
    let mut transaction = connection
        .begin()
        .await
        .map_err(|error| format!("Failed to start job failure transaction: {error}"))?;
    sqlx::query(
        "UPDATE recording_processing_jobs
         SET status = ?, error = ?, updated_at = ?, completed_at = ?
         WHERE id = ?",
    )
    .bind(JOB_FAILED)
    .bind(error)
    .bind(&now)
    .bind(&now)
    .bind(job_id)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to mark job failed: {error}"))?;
    sqlx::query("UPDATE recordings SET is_partial = 1 WHERE id = ?")
        .bind(recording_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("Failed to mark recording partial: {error}"))?;
    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to commit job failure: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{mark_job_failed, JOB_FAILED};
    use sqlx::{Connection, Row, SqliteConnection};

    async fn memory_connection() -> SqliteConnection {
        SqliteConnection::connect("sqlite::memory:")
            .await
            .expect("memory sqlite connection")
    }

    #[test]
    fn mark_job_failed_sets_job_and_recording_status() {
        tauri::async_runtime::block_on(async {
            let mut connection = memory_connection().await;
            sqlx::query(
                "CREATE TABLE recordings (
                    id TEXT PRIMARY KEY,
                    is_partial INTEGER NOT NULL DEFAULT 0
                )",
            )
            .execute(&mut connection)
            .await
            .expect("create recordings");
            sqlx::query(
                "CREATE TABLE recording_processing_jobs (
                    id TEXT PRIMARY KEY,
                    recording_id TEXT NOT NULL,
                    status TEXT NOT NULL,
                    error TEXT,
                    updated_at TEXT,
                    completed_at TEXT
                )",
            )
            .execute(&mut connection)
            .await
            .expect("create jobs");
            sqlx::query("INSERT INTO recordings (id, is_partial) VALUES ('recording-1', 0)")
                .execute(&mut connection)
                .await
                .expect("insert recording");
            sqlx::query(
                "INSERT INTO recording_processing_jobs (id, recording_id, status)
                 VALUES ('job-1', 'recording-1', 'chunking')",
            )
            .execute(&mut connection)
            .await
            .expect("insert job");

            mark_job_failed(&mut connection, "recording-1", "job-1", "chunking failed")
                .await
                .expect("mark failed");

            let job = sqlx::query(
                "SELECT status, error FROM recording_processing_jobs WHERE id = 'job-1'",
            )
            .fetch_one(&mut connection)
            .await
            .expect("select job");
            let recording =
                sqlx::query("SELECT is_partial FROM recordings WHERE id = 'recording-1'")
                    .fetch_one(&mut connection)
                    .await
                    .expect("select recording");

            assert_eq!(job.get::<String, _>("status"), JOB_FAILED);
            assert_eq!(job.get::<String, _>("error"), "chunking failed");
            assert_eq!(recording.get::<i64, _>("is_partial"), 1);
        });
    }
}
