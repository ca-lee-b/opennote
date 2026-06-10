mod chunking;
mod cleanup;
mod model;
mod recording_write;
mod repository;
mod workers;

use self::model::{
    EnqueueRecordingTranscriptionRequest, EnqueueRecordingTranscriptionResult,
    ImportAudioTranscriptionRequest, RecordingProcessingStatus,
};
use self::workers::PipelineRuntime;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, State};

#[derive(Default)]
pub struct PipelineState {
    runtime: Mutex<Option<PipelineRuntime>>,
    startup_checked: AtomicBool,
}

#[tauri::command]
pub async fn enqueue_recording_transcription(
    app: AppHandle,
    request: EnqueueRecordingTranscriptionRequest,
    state: State<'_, PipelineState>,
) -> Result<EnqueueRecordingTranscriptionResult, String> {
    ensure_pipeline(&app, &state).await?;
    let result = repository::enqueue_recording(&app, request).await?;
    wake_pipeline(&state)?;
    Ok(result)
}

#[tauri::command]
pub async fn import_audio_for_transcription(
    app: AppHandle,
    request: ImportAudioTranscriptionRequest,
    state: State<'_, PipelineState>,
) -> Result<EnqueueRecordingTranscriptionResult, String> {
    ensure_pipeline(&app, &state).await?;
    let audio_dir = super::import_audio::audio_dir(&app)?;
    let prepared = super::import_audio::prepare_imported_audio(
        std::path::Path::new(&request.source_audio_path),
        &audio_dir,
    )?;
    let result = repository::enqueue_recording(
        &app,
        EnqueueRecordingTranscriptionRequest {
            audio_path: prepared.wav_path.display().to_string(),
            duration: prepared.duration,
            model_id: request.model_id,
            save_audio: true,
            started_at: None,
            title: request.title,
        },
    )
    .await?;
    wake_pipeline(&state)?;
    Ok(result)
}

#[tauri::command]
pub async fn list_recording_processing_statuses(
    app: AppHandle,
    state: State<'_, PipelineState>,
) -> Result<Vec<RecordingProcessingStatus>, String> {
    ensure_startup_checked(&app, &state).await?;
    repository::list_processing_statuses(&app).await
}

#[tauri::command]
pub async fn resume_recording_processing(
    app: AppHandle,
    recording_id: String,
    state: State<'_, PipelineState>,
) -> Result<(), String> {
    ensure_pipeline(&app, &state).await?;
    repository::resume_recording(&app, &recording_id).await?;
    wake_pipeline(&state)?;
    Ok(())
}

#[tauri::command]
pub async fn delete_recording(app: AppHandle, recording_id: String) -> Result<(), String> {
    repository::delete_recording(&app, &recording_id).await
}

async fn ensure_pipeline(app: &AppHandle, state: &PipelineState) -> Result<(), String> {
    ensure_startup_checked(app, state).await?;
    let mut runtime = state
        .runtime
        .lock()
        .map_err(|_| "Recording pipeline state is unavailable".to_string())?;
    if runtime.is_none() {
        *runtime = Some(PipelineRuntime::start(app.clone()));
    }
    Ok(())
}

async fn ensure_startup_checked(app: &AppHandle, state: &PipelineState) -> Result<(), String> {
    if !state.startup_checked.swap(true, Ordering::SeqCst) {
        repository::interrupt_stale_jobs(app).await?;
    }
    Ok(())
}

fn wake_pipeline(state: &PipelineState) -> Result<(), String> {
    let runtime = state
        .runtime
        .lock()
        .map_err(|_| "Recording pipeline state is unavailable".to_string())?;
    if let Some(runtime) = runtime.as_ref() {
        runtime.wake();
    }
    Ok(())
}
