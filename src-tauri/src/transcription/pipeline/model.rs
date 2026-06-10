use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub(super) const CHUNK_LOGICAL_SECS: f64 = 300.0;
pub(super) const CHUNK_OVERLAP_SECS: f64 = 15.0;
pub(super) const CHUNK_LINE_STRIDE: i64 = 10_000;
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(750);

pub(super) const JOB_QUEUED: &str = "queued";
pub(super) const JOB_CHUNKING: &str = "chunking";
pub(super) const JOB_TRANSCRIBING: &str = "transcribing";
pub(super) const JOB_COMPLETE: &str = "complete";
pub(super) const JOB_PARTIAL: &str = "partial";
pub(super) const JOB_FAILED: &str = "failed";
pub(super) const JOB_INTERRUPTED: &str = "interrupted";
pub(super) const JOB_CANCELLED: &str = "cancelled";

pub(super) const CHUNK_QUEUED: &str = "queued";
pub(super) const CHUNK_TRANSCRIBING: &str = "transcribing";
pub(super) const CHUNK_COMPLETED: &str = "completed";
pub(super) const CHUNK_FAILED: &str = "failed";
pub(super) const CHUNK_INTERRUPTED: &str = "interrupted";
pub(super) const CHUNK_CANCELLED: &str = "cancelled";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueRecordingTranscriptionRequest {
    pub(super) audio_path: String,
    pub(super) duration: f64,
    pub(super) model_id: String,
    pub(super) save_audio: bool,
    pub(super) started_at: Option<String>,
    pub(super) title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueRecordingTranscriptionResult {
    pub job_id: String,
    pub recording_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAudioTranscriptionRequest {
    pub(super) model_id: String,
    pub(super) source_audio_path: String,
    pub(super) title: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingStatus {
    pub completed_chunks: i64,
    pub error: Option<String>,
    pub failed_chunks: i64,
    pub job_id: String,
    pub recording_id: String,
    pub status: String,
    pub total_chunks: i64,
    pub updated_at: String,
}

pub(super) struct JobRow {
    pub id: String,
    pub recording_id: String,
    pub source_audio_path: String,
}

pub(super) struct ChunkWork {
    pub chunk_id: String,
    pub chunk_index: i64,
    pub chunk_path: String,
    pub job_id: String,
    pub logical_end_secs: f64,
    pub logical_start_secs: f64,
    pub model_id: String,
    pub recording_id: String,
    pub source_start_secs: f64,
}

#[derive(Debug, PartialEq)]
pub(super) struct ChunkWindow {
    pub index: i64,
    pub logical_start_secs: f64,
    pub logical_end_secs: f64,
    pub source_start_secs: f64,
    pub source_end_secs: f64,
}

pub(super) fn chunk_line_base(chunk_index: i64) -> i64 {
    chunk_index * CHUNK_LINE_STRIDE
}

pub(super) fn format_timestamp(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

pub(super) fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
