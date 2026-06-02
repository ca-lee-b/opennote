use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;

const DOWNLOAD_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const LEGACY_MODEL_DIRS: &[&str] = &[
    "moonshine-small",
    "moonshine-medium",
    "parakeet-tdt-0.6b-v3",
];

#[derive(Default)]
pub struct DownloadState {
    pub cancellation_tokens: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub progress: Mutex<HashMap<String, f64>>,
}

struct Artifact {
    file_name: &'static str,
    sha256: &'static str,
    size_bytes: u64,
}

struct ModelDescriptor {
    id: &'static str,
    dir_name: &'static str,
    display_name: &'static str,
    wer: &'static str,
    blurb: &'static str,
    parameter_count: &'static str,
    model: Artifact,
    coreml_encoder: Artifact,
    coreml_dir_name: &'static str,
}

impl ModelDescriptor {
    fn download_size_bytes(&self) -> u64 {
        self.model.size_bytes + self.coreml_encoder.size_bytes
    }

    fn model_path(&self, models_dir: &Path) -> PathBuf {
        models_dir.join(self.dir_name).join(self.model.file_name)
    }

    fn artifacts(&self) -> [&Artifact; 2] {
        [&self.model, &self.coreml_encoder]
    }
}

const MODEL_REGISTRY: &[ModelDescriptor] = &[
    ModelDescriptor {
        id: "whisper_small_en_q5_1",
        dir_name: "whisper-small-en-q5_1",
        display_name: "Whisper Small English",
        wer: "Quantized Q5_1",
        blurb: "Fast English transcription for everyday recordings.",
        parameter_count: "466M",
        model: Artifact {
            file_name: "ggml-small.en-q5_1.bin",
            sha256: "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30",
            size_bytes: 190_098_681,
        },
        coreml_encoder: Artifact {
            file_name: "ggml-small.en-encoder.mlmodelc.zip",
            sha256: "b2ef1c506378b825b4b4341979a93e1656b5d6c129f17114cfb8fb78aabc2f89",
            size_bytes: 162_952_446,
        },
        coreml_dir_name: "ggml-small.en-encoder.mlmodelc",
    },
    ModelDescriptor {
        id: "whisper_medium_en_q5_0",
        dir_name: "whisper-medium-en-q5_0",
        display_name: "Whisper Medium English",
        wer: "Quantized Q5_0",
        blurb: "More accurate English transcription for longer or technical recordings.",
        parameter_count: "769M",
        model: Artifact {
            file_name: "ggml-medium.en-q5_0.bin",
            sha256: "76733e26ad8fe1c7a5bf7531a9d41917b2adc0f20f2e4f5531688a8c6cd88eb0",
            size_bytes: 539_225_533,
        },
        coreml_encoder: Artifact {
            file_name: "ggml-medium.en-encoder.mlmodelc.zip",
            sha256: "cdc44fee3c62b5743913e3147ed75f4e8ecfb52dd7a0f0f7387094b406ff0ee6",
            size_bytes: 566_993_085,
        },
        coreml_dir_name: "ggml-medium.en-encoder.mlmodelc",
    },
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadInfo {
    pub id: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub wer: String,
    pub blurb: String,
    pub parameter_count: String,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub download_progress: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadStatus {
    Downloading,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgressEvent {
    model_id: String,
    progress: f64,
    status: DownloadStatus,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadModelRequest {
    pub model_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteModelRequest {
    pub model_id: String,
}

pub fn cleanup_legacy_models(app: &AppHandle) -> Result<(), String> {
    let models_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data dir: {error}"))?
        .join("models");

    for dir_name in LEGACY_MODEL_DIRS {
        let path = models_dir.join(dir_name);
        if path.exists() {
            std::fs::remove_dir_all(&path).map_err(|error| {
                format!("Failed to remove legacy model {}: {error}", path.display())
            })?;
        }
    }

    Ok(())
}

pub fn resolve_downloaded_model(app: &AppHandle, model_id: &str) -> Result<PathBuf, String> {
    let descriptor = descriptor_for(model_id)?;
    let models_dir = models_dir(app)?;
    let model_dir = models_dir.join(descriptor.dir_name);
    if !is_model_downloaded(&model_dir, descriptor) {
        return Err(format!(
            "Selected model is not downloaded. Open Settings and download {}.",
            descriptor.display_name
        ));
    }
    Ok(descriptor.model_path(&models_dir))
}

#[tauri::command]
pub fn get_downloaded_models(
    app: AppHandle,
    download_state: State<'_, DownloadState>,
) -> Result<Vec<ModelDownloadInfo>, String> {
    cleanup_legacy_models(&app)?;
    let models_dir = models_dir(&app)?;
    let cancellation_tokens = download_state
        .cancellation_tokens
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?;
    let progress = download_state
        .progress
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?;

    Ok(MODEL_REGISTRY
        .iter()
        .map(|descriptor| {
            let model_dir = models_dir.join(descriptor.dir_name);
            ModelDownloadInfo {
                id: descriptor.id.to_string(),
                display_name: descriptor.display_name.to_string(),
                size_bytes: descriptor.download_size_bytes(),
                wer: descriptor.wer.to_string(),
                blurb: descriptor.blurb.to_string(),
                parameter_count: descriptor.parameter_count.to_string(),
                is_downloaded: is_model_downloaded(&model_dir, descriptor),
                is_downloading: cancellation_tokens.contains_key(descriptor.id),
                download_progress: progress.get(descriptor.id).copied().unwrap_or(0.0),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    request: DownloadModelRequest,
    download_state: State<'_, DownloadState>,
) -> Result<(), String> {
    let model_id = request.model_id;
    let descriptor = descriptor_for(&model_id)?;
    let models_dir = models_dir(&app)?;
    std::fs::create_dir_all(&models_dir)
        .map_err(|error| format!("Failed to create models directory: {error}"))?;
    let staging_dir = models_dir.join(format!(".{}.download", descriptor.dir_name));
    let model_dir = models_dir.join(descriptor.dir_name);
    remove_dir_if_exists(&staging_dir)?;
    std::fs::create_dir_all(&staging_dir)
        .map_err(|error| format!("Failed to create staging directory: {error}"))?;
    let cancel_token = register_download(&download_state, &model_id)?;

    let result = download_and_activate(
        &app,
        &download_state,
        descriptor,
        &model_id,
        &cancel_token,
        &staging_dir,
        &model_dir,
    )
    .await;

    if result.is_err() {
        let _ = remove_dir_if_exists(&staging_dir);
    }
    clear_download(&download_state, &model_id)?;
    emit_final_download_status(&app, &model_id, &result);
    result
}

#[tauri::command]
pub fn delete_model(
    app: AppHandle,
    request: DeleteModelRequest,
    transcription_state: State<'_, crate::transcription::TranscriptionState>,
    download_state: State<'_, DownloadState>,
) -> Result<(), String> {
    let model_id = request.model_id;
    let descriptor = descriptor_for(&model_id)?;
    {
        let model_info = transcription_state
            .model_info
            .lock()
            .map_err(|_| "Transcription state is unavailable".to_string())?;
        if model_info.loaded_model_id.as_deref() == Some(model_id.as_str()) {
            return Err(
                "Cannot delete model that is currently in use. Please select a different model first."
                    .to_string(),
            );
        }
    }
    {
        let tokens = download_state
            .cancellation_tokens
            .lock()
            .map_err(|_| "Download state is unavailable".to_string())?;
        if tokens.contains_key(&model_id) {
            return Err("Cannot delete model while it is being downloaded.".to_string());
        }
    }
    remove_dir_if_exists(&models_dir(&app)?.join(descriptor.dir_name))
}

#[tauri::command]
pub fn cancel_download(
    model_id: String,
    download_state: State<'_, DownloadState>,
) -> Result<(), String> {
    let tokens = download_state
        .cancellation_tokens
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?;
    if let Some(cancel_token) = tokens.get(&model_id) {
        cancel_token.store(true, Ordering::Relaxed);
        Ok(())
    } else {
        Err(format!("No active download for model: {model_id}"))
    }
}

async fn download_and_activate(
    app: &AppHandle,
    download_state: &State<'_, DownloadState>,
    descriptor: &ModelDescriptor,
    model_id: &str,
    cancel_token: &AtomicBool,
    staging_dir: &Path,
    model_dir: &Path,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let total_size = descriptor.download_size_bytes();
    let mut completed_bytes = 0_u64;

    for artifact in descriptor.artifacts() {
        ensure_not_cancelled(cancel_token)?;
        let path = staging_dir.join(artifact.file_name);
        download_artifact(
            app,
            download_state,
            &client,
            model_id,
            artifact,
            &path,
            completed_bytes,
            total_size,
            cancel_token,
        )
        .await?;
        completed_bytes += artifact.size_bytes;
    }

    ensure_not_cancelled(cancel_token)?;
    extract_coreml_encoder(
        &staging_dir.join(descriptor.coreml_encoder.file_name),
        staging_dir,
        cancel_token,
    )?;
    std::fs::remove_file(staging_dir.join(descriptor.coreml_encoder.file_name))
        .map_err(|error| format!("Failed to remove Core ML archive: {error}"))?;
    validate_staged_model(staging_dir, descriptor)?;
    remove_dir_if_exists(model_dir)?;
    std::fs::rename(staging_dir, model_dir)
        .map_err(|error| format!("Failed to activate downloaded model: {error}"))?;
    Ok(())
}

async fn download_artifact(
    app: &AppHandle,
    download_state: &State<'_, DownloadState>,
    client: &reqwest::Client,
    model_id: &str,
    artifact: &Artifact,
    destination: &Path,
    completed_bytes: u64,
    total_size: u64,
    cancel_token: &AtomicBool,
) -> Result<(), String> {
    let url = format!("{DOWNLOAD_BASE_URL}/{}", artifact.file_name);
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Failed to download {}: {error}", artifact.file_name))?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {}: HTTP {}",
            artifact.file_name,
            response.status()
        ));
    }

    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|error| format!("Failed to create {}: {error}", artifact.file_name))?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    let mut stream = response.bytes_stream();
    let mut last_reported = -1.0_f64;

    while let Some(chunk) = stream.next().await {
        ensure_not_cancelled(cancel_token)?;
        let chunk =
            chunk.map_err(|error| format!("Failed to read {}: {error}", artifact.file_name))?;
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("Failed to write {}: {error}", artifact.file_name))?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        let progress = (completed_bytes + downloaded) as f64 / total_size as f64;
        if progress - last_reported >= 0.005 || downloaded >= artifact.size_bytes {
            last_reported = progress;
            update_progress(app, download_state, model_id, progress)?;
        }
    }
    file.flush()
        .await
        .map_err(|error| format!("Failed to flush {}: {error}", artifact.file_name))?;

    if downloaded != artifact.size_bytes {
        return Err(format!(
            "Downloaded {} bytes for {}, expected {}",
            downloaded, artifact.file_name, artifact.size_bytes
        ));
    }
    let digest = format!("{:x}", hasher.finalize());
    if digest != artifact.sha256 {
        return Err(format!(
            "Checksum verification failed for {}",
            artifact.file_name
        ));
    }
    Ok(())
}

fn extract_coreml_encoder(
    archive_path: &Path,
    destination: &Path,
    cancel_token: &AtomicBool,
) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("Failed to open Core ML archive: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("Invalid Core ML archive: {error}"))?;
    for index in 0..archive.len() {
        ensure_not_cancelled(cancel_token)?;
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to read Core ML archive: {error}"))?;
        let relative_path = entry
            .enclosed_name()
            .ok_or_else(|| "Core ML archive contains an unsafe path".to_string())?;
        let output_path = destination.join(relative_path);
        if entry.is_dir() {
            std::fs::create_dir_all(&output_path)
                .map_err(|error| format!("Failed to create Core ML directory: {error}"))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create Core ML directory: {error}"))?;
        }
        let mut output = File::create(&output_path)
            .map_err(|error| format!("Failed to extract Core ML file: {error}"))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("Failed to extract Core ML file: {error}"))?;
    }
    Ok(())
}

fn validate_staged_model(model_dir: &Path, descriptor: &ModelDescriptor) -> Result<(), String> {
    let model_path = model_dir.join(descriptor.model.file_name);
    if !is_non_empty_file(&model_path) {
        return Err(format!(
            "Downloaded model is missing {}",
            descriptor.model.file_name
        ));
    }
    let coreml_dir = model_dir.join(descriptor.coreml_dir_name);
    if !coreml_dir.is_dir() || !directory_contains_file(&coreml_dir) {
        return Err(format!(
            "Downloaded model is missing {}",
            descriptor.coreml_dir_name
        ));
    }
    Ok(())
}

fn directory_contains_file(path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        path.is_file() || (path.is_dir() && directory_contains_file(&path))
    })
}

fn is_model_downloaded(model_dir: &Path, descriptor: &ModelDescriptor) -> bool {
    validate_staged_model(model_dir, descriptor).is_ok()
}

fn is_non_empty_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn descriptor_for(model_id: &str) -> Result<&'static ModelDescriptor, String> {
    MODEL_REGISTRY
        .iter()
        .find(|descriptor| descriptor.id == model_id)
        .ok_or_else(|| format!("Unknown model: {model_id}"))
}

fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("models"))
        .map_err(|error| format!("Failed to resolve app data dir: {error}"))
}

fn remove_dir_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .map_err(|error| format!("Failed to remove {}: {error}", path.display()))?;
    }
    Ok(())
}

fn ensure_not_cancelled(cancel_token: &AtomicBool) -> Result<(), String> {
    if cancel_token.load(Ordering::Relaxed) {
        Err("Download cancelled".to_string())
    } else {
        Ok(())
    }
}

fn register_download(
    download_state: &State<'_, DownloadState>,
    model_id: &str,
) -> Result<Arc<AtomicBool>, String> {
    let mut tokens = download_state
        .cancellation_tokens
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?;
    if tokens.contains_key(model_id) {
        return Err(format!(
            "Download already in progress for model: {model_id}"
        ));
    }
    let cancel_token = Arc::new(AtomicBool::new(false));
    tokens.insert(model_id.to_string(), Arc::clone(&cancel_token));
    download_state
        .progress
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .insert(model_id.to_string(), 0.0);
    Ok(cancel_token)
}

fn clear_download(download_state: &State<'_, DownloadState>, model_id: &str) -> Result<(), String> {
    download_state
        .cancellation_tokens
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .remove(model_id);
    download_state
        .progress
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .remove(model_id);
    Ok(())
}

fn update_progress(
    app: &AppHandle,
    download_state: &State<'_, DownloadState>,
    model_id: &str,
    progress: f64,
) -> Result<(), String> {
    download_state
        .progress
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .insert(model_id.to_string(), progress);
    let _ = app.emit(
        "model-download-progress",
        ModelDownloadProgressEvent {
            model_id: model_id.to_string(),
            progress,
            status: DownloadStatus::Downloading,
        },
    );
    Ok(())
}

fn emit_final_download_status(app: &AppHandle, model_id: &str, result: &Result<(), String>) {
    let (progress, status) = match result {
        Ok(()) => (1.0, DownloadStatus::Completed),
        Err(message) if message == "Download cancelled" => (0.0, DownloadStatus::Cancelled),
        Err(_) => (0.0, DownloadStatus::Failed),
    };
    let _ = app.emit(
        "model-download-progress",
        ModelDownloadProgressEvent {
            model_id: model_id.to_string(),
            progress,
            status,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::{descriptor_for, MODEL_REGISTRY};

    #[test]
    fn registry_contains_supported_whisper_models() {
        assert_eq!(MODEL_REGISTRY.len(), 2);
        assert!(descriptor_for("whisper_small_en_q5_1").is_ok());
        assert!(descriptor_for("whisper_medium_en_q5_0").is_ok());
    }

    #[test]
    fn registry_rejects_legacy_model_id() {
        assert!(descriptor_for("small_streaming").is_err());
    }

    #[test]
    fn artifacts_have_sha256_checksums_and_sizes() {
        for descriptor in MODEL_REGISTRY {
            for artifact in descriptor.artifacts() {
                assert_eq!(artifact.sha256.len(), 64);
                assert!(artifact.size_bytes > 0);
            }
        }
    }
}
