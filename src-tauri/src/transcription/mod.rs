pub mod audio;
pub mod models;
mod whisper;
mod worker;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Instant, SystemTime};
use tauri::{AppHandle, Manager, State};

pub use models::DownloadState;
use worker::{RecordingData, TranscriptionResult, TranscriptionWorker};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSource {
    Microphone,
    ComputerAudio,
}

pub struct ModelInfoState {
    pub loaded_model_id: Option<String>,
    pub loaded_model_path: Option<String>,
    pub is_recording: bool,
    pub started_at: Option<Instant>,
    pub started_wall_time: Option<SystemTime>,
}

pub struct TranscriptionState {
    pub model_info: Mutex<ModelInfoState>,
    pub worker: Mutex<Option<TranscriptionWorker>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionStateSnapshot {
    is_model_loaded: bool,
    is_recording: bool,
    loaded_model_id: Option<String>,
    loaded_model_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadModelRequest {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingRequest {
    audio_source: AudioSource,
    save_audio: bool,
}

#[cfg(target_os = "macos")]
mod macos_permissions {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    unsafe extern "C" {
        fn opennote_check_screen_capture_permission() -> bool;
        fn opennote_open_screen_capture_settings() -> bool;
        fn opennote_is_macos_13_or_newer() -> bool;
        fn opennote_screen_capture_permission_error() -> *const c_char;
    }

    pub fn check_screen_capture_permission() -> bool {
        // SAFETY: bridge function has no side effects beyond OS permission check.
        unsafe { opennote_check_screen_capture_permission() }
    }

    pub fn open_screen_capture_settings() -> bool {
        // SAFETY: bridge function requests macOS to open system settings URL.
        unsafe { opennote_open_screen_capture_settings() }
    }

    pub fn is_macos_13_or_newer() -> bool {
        // SAFETY: bridge function reads OS availability only.
        unsafe { opennote_is_macos_13_or_newer() }
    }

    pub fn permission_error_message() -> String {
        // SAFETY: bridge returns a static null-terminated C string.
        let ptr = unsafe { opennote_screen_capture_permission_error() };
        if ptr.is_null() {
            return "Screen Recording permission is required to record computer audio. Enable it in System Settings > Privacy & Security > Screen Recording, then restart OpenNote.".to_string();
        }
        // SAFETY: pointer is expected to reference static UTF-8 bytes.
        unsafe { CStr::from_ptr(ptr) }.to_string_lossy().to_string()
    }
}

#[cfg(not(target_os = "macos"))]
mod macos_permissions {
    pub fn check_screen_capture_permission() -> bool {
        false
    }

    pub fn open_screen_capture_settings() -> bool {
        false
    }

    pub fn is_macos_13_or_newer() -> bool {
        false
    }

    pub fn permission_error_message() -> String {
        "Computer audio recording is only available on macOS 13 or newer.".to_string()
    }
}

#[tauri::command]
pub fn get_transcription_state(
    state: State<'_, TranscriptionState>,
) -> Result<TranscriptionStateSnapshot, String> {
    transcription_state_snapshot(&state)
}

fn transcription_state_snapshot(
    state: &TranscriptionState,
) -> Result<TranscriptionStateSnapshot, String> {
    let info = state
        .model_info
        .lock()
        .map_err(|_| "Transcription state is unavailable".to_string())?;

    Ok(TranscriptionStateSnapshot {
        is_model_loaded: info.loaded_model_id.is_some(),
        is_recording: info.is_recording,
        loaded_model_id: info.loaded_model_id.clone(),
        loaded_model_path: info.loaded_model_path.clone(),
    })
}

#[tauri::command]
pub async fn load_transcription_model(
    app: AppHandle,
    request: LoadModelRequest,
    state: State<'_, TranscriptionState>,
) -> Result<TranscriptionStateSnapshot, String> {
    let model_path = models::resolve_downloaded_model(&app, &request.id)?;

    {
        let info = state
            .model_info
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        if info.loaded_model_id.as_deref() == Some(request.id.as_str()) {
            return transcription_state_snapshot(&state);
        }
    }

    ensure_worker(&state)?;

    {
        let worker_guard = state
            .worker
            .lock()
            .map_err(|_| "Worker state unavailable".to_string())?;
        if let Some(ref worker) = *worker_guard {
            worker.load_model(model_path.clone(), request.id.clone())?;
        }
    }

    {
        let mut info = state
            .model_info
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        info.loaded_model_id = Some(request.id);
        info.loaded_model_path = Some(model_path.display().to_string());
    }

    transcription_state_snapshot(&state)
}

#[tauri::command]
pub async fn start_transcription_recording(
    app: AppHandle,
    request: StartRecordingRequest,
    state: State<'_, TranscriptionState>,
) -> Result<TranscriptionStateSnapshot, String> {
    validate_audio_source(request.audio_source)?;

    {
        let info = state
            .model_info
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        if info.loaded_model_id.is_none() {
            return Err("No model is loaded".to_string());
        }
        if info.is_recording {
            return Err("Recording is already in progress".to_string());
        }
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    let wav_path = if request.save_audio {
        let audio_dir = data_dir.join("audio");
        std::fs::create_dir_all(&audio_dir)
            .map_err(|e| format!("Failed to create audio dir: {e}"))?;
        let id = uuid::Uuid::new_v4().to_string();
        audio_dir.join(format!("{id}.wav"))
    } else {
        let temp_dir = std::env::temp_dir();
        let id = uuid::Uuid::new_v4().to_string();
        temp_dir.join(format!("opennote-{id}.wav"))
    };

    ensure_worker(&state)?;

    {
        let worker_guard = state
            .worker
            .lock()
            .map_err(|_| "Worker state unavailable".to_string())?;
        if let Some(ref worker) = *worker_guard {
            worker.start_recording(app.clone(), wav_path, request.audio_source)?;
        }
    }

    {
        let mut info = state
            .model_info
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        info.is_recording = true;
        info.started_at = Some(Instant::now());
        info.started_wall_time = Some(SystemTime::now());
    }

    transcription_state_snapshot(&state)
}

#[tauri::command]
pub async fn stop_transcription_recording(
    _app: AppHandle,
    state: State<'_, TranscriptionState>,
) -> Result<RecordingData, String> {
    {
        let info = state
            .model_info
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        if !info.is_recording {
            return Err("Recording is not in progress".to_string());
        }
    }

    let recording_data = {
        let worker_guard = state
            .worker
            .lock()
            .map_err(|_| "Worker state unavailable".to_string())?;
        if let Some(ref worker) = *worker_guard {
            worker.stop_recording()?
        } else {
            return Err("No worker available".to_string());
        }
    };

    {
        let mut info = state
            .model_info
            .lock()
            .map_err(|_| "State unavailable".to_string())?;
        info.is_recording = false;
        info.started_at = None;
        info.started_wall_time = None;
    }

    Ok(recording_data)
}

#[tauri::command]
pub async fn transcribe_recording(
    state: State<'_, TranscriptionState>,
    wav_path: String,
) -> Result<TranscriptionResult, String> {
    let worker_guard = state
        .worker
        .lock()
        .map_err(|_| "Worker state unavailable".to_string())?;
    if let Some(ref worker) = *worker_guard {
        worker.transcribe_file(std::path::PathBuf::from(wav_path))
    } else {
        Err("No worker available".to_string())
    }
}

#[tauri::command]
pub fn delete_audio_file(path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&path);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete audio file: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn clear_all_audio_files(app: AppHandle) -> Result<usize, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let audio_dir = data_dir.join("audio");
    if !audio_dir.exists() {
        return Ok(0);
    }
    let mut count = 0usize;
    let entries = std::fs::read_dir(&audio_dir)
        .map_err(|e| format!("Failed to read audio directory: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_file() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete {}: {e}", path.display()))?;
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn get_system_audio_permission() -> Result<bool, String> {
    if !macos_permissions::is_macos_13_or_newer() {
        return Ok(false);
    }
    Ok(macos_permissions::check_screen_capture_permission())
}

#[tauri::command]
pub fn open_system_audio_settings() -> Result<(), String> {
    if !macos_permissions::open_screen_capture_settings() {
        return Err(
            "Unable to open Screen Recording settings. Open System Settings > Privacy & Security > Screen Recording manually."
                .to_string(),
        );
    }
    Ok(())
}

pub fn validate_audio_source(source: AudioSource) -> Result<(), String> {
    match source {
        AudioSource::Microphone => Ok(()),
        AudioSource::ComputerAudio => {
            if !macos_permissions::is_macos_13_or_newer() {
                return Err(
                    "Computer audio recording is only available on macOS 13 or newer.".to_string(),
                );
            }
            if !macos_permissions::check_screen_capture_permission() {
                return Err(macos_permissions::permission_error_message());
            }
            Ok(())
        }
    }
}

fn ensure_worker(state: &State<'_, TranscriptionState>) -> Result<(), String> {
    let mut worker_guard = state
        .worker
        .lock()
        .map_err(|_| "Worker state unavailable".to_string())?;
    if worker_guard.is_none() {
        *worker_guard = Some(TranscriptionWorker::start());
    }
    Ok(())
}
