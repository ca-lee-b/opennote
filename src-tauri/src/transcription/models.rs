use crate::transcription::ModelArch;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;

#[derive(Default)]
pub struct DownloadState {
    pub cancellation_tokens: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub progress: Mutex<HashMap<String, f64>>,
}

struct ModelDescriptor {
    id: &'static str,
    arch: ModelArch,
    dir_name: &'static str,
    display_name: &'static str,
    size_bytes: u64,
    wer: &'static str,
    blurb: &'static str,
    parameter_count: &'static str,
    download_base_url: &'static str,
    /// Files that must exist locally for the model to be considered downloaded.
    required_files: &'static [&'static str],
    /// Mapping from URL filename → local filename. If empty, `required_files`
    /// filenames are used for both the download URL and the local path.
    download_files: &'static [(&'static str, &'static str)],
}

impl ModelDescriptor {
    /// Returns the (url_filename, local_filename) pairs for downloading.
    /// Falls back to using required_files as both url and local names when download_files is empty.
    fn download_pairs(&self) -> Vec<(&str, &str)> {
        if self.download_files.is_empty() {
            self.required_files
                .iter()
                .map(|f| (*f, *f))
                .collect()
        } else {
            self.download_files
                .iter()
                .map(|(url, local)| (*url, *local))
                .collect()
        }
    }
}

const MODEL_REGISTRY: &[ModelDescriptor] = &[
    ModelDescriptor {
        id: "small_streaming",
        arch: ModelArch::Small,
        dir_name: "moonshine-small",
        display_name: "Small Streaming",
        size_bytes: 128_974_848,
        wer: "7.84% WER",
        blurb: "Balanced accuracy and speed. Perfect for everyday conversations and meetings.",
        parameter_count: "123M",
        download_base_url: "https://download.moonshine.ai/model/small-streaming-en/quantized",
        required_files: &[
            "adapter.ort",
            "cross_kv.ort",
            "decoder_kv.ort",
            "encoder.ort",
            "frontend.ort",
            "streaming_config.json",
            "tokenizer.bin",
        ],
        download_files: &[],
    },
    ModelDescriptor {
        id: "medium_streaming",
        arch: ModelArch::Medium,
        dir_name: "moonshine-medium",
        display_name: "Medium Streaming",
        size_bytes: 256_901_120,
        wer: "6.65% WER",
        blurb: "Best accuracy for long lectures and technical vocabulary. Requires more storage.",
        parameter_count: "200M",
        download_base_url: "https://download.moonshine.ai/model/medium-streaming-en/quantized",
        required_files: &[
            "adapter.ort",
            "cross_kv.ort",
            "decoder_kv.ort",
            "encoder.ort",
            "frontend.ort",
            "streaming_config.json",
            "tokenizer.bin",
        ],
        download_files: &[],
    },
    ModelDescriptor {
        id: "parakeet_tdt_0.6b_v3",
        arch: ModelArch::ParakeetTdt,
        dir_name: "parakeet-tdt-0.6b-v3",
        display_name: "Parakeet TDT 0.6B v3",
        size_bytes: 933_000_000,
        wer: "~5% WER",
        blurb: "NVIDIA's state-of-the-art multilingual model. 25 languages with automatic detection. Best accuracy.",
        parameter_count: "600M",
        download_base_url: "https://huggingface.co/nasedkinpv/parakeet-tdt-0.6b-v3-onnx-int8/resolve/main",
        required_files: &[
            "encoder-int8.onnx",
            "encoder-int8.onnx.data",
            "decoder_joint-model.int8.onnx",
            "vocab.txt",
        ],
        download_files: &[
            ("encoder-int8.onnx", "encoder-int8.onnx"),
            ("encoder-int8.onnx.data", "encoder-int8.onnx.data"),
            ("decoder_joint-int8.onnx", "decoder_joint-model.int8.onnx"),
            ("vocab.txt", "vocab.txt"),
        ],
    },
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadInfo {
    pub id: String,
    pub arch: ModelArch,
    pub display_name: String,
    pub size_bytes: u64,
    pub wer: String,
    pub blurb: String,
    pub parameter_count: String,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub download_progress: f64,
    pub path: String,
}

#[tauri::command]
pub fn get_downloaded_models(
    app: AppHandle,
    download_state: State<'_, DownloadState>,
) -> Result<Vec<ModelDownloadInfo>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    let models_dir = data_dir.join("models");

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
        .map(|desc| {
            let model_path = models_dir.join(desc.dir_name);
            let is_downloading = cancellation_tokens.contains_key(desc.id);
            let download_progress = progress.get(desc.id).copied().unwrap_or(0.0);
            ModelDownloadInfo {
                id: desc.id.to_string(),
                arch: desc.arch,
                display_name: desc.display_name.to_string(),
                size_bytes: desc.size_bytes,
                wer: desc.wer.to_string(),
                blurb: desc.blurb.to_string(),
                parameter_count: desc.parameter_count.to_string(),
                is_downloaded: is_model_downloaded(&model_path, desc.required_files),
                is_downloading,
                download_progress,
                path: model_path.display().to_string(),
            }
        })
        .collect())
}

fn is_model_downloaded(model_dir: &Path, required_files: &[&str]) -> bool {
    if !model_dir.is_dir() {
        return false;
    }
    for file_name in required_files {
        let file_path = model_dir.join(file_name);
        match std::fs::metadata(&file_path) {
            Ok(metadata) if metadata.len() > 0 => {}
            _ => return false,
        }
    }
    true
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

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    request: DownloadModelRequest,
    download_state: State<'_, DownloadState>,
) -> Result<(), String> {
    let model_id = request.model_id;

    let descriptor = MODEL_REGISTRY
        .iter()
        .find(|d| d.id == model_id)
        .ok_or_else(|| format!("Unknown model: {model_id}"))?;

    {
        let tokens = download_state
            .cancellation_tokens
            .lock()
            .map_err(|_| "Download state is unavailable".to_string())?;
        if tokens.contains_key(&model_id) {
            return Err(format!(
                "Download already in progress for model: {model_id}"
            ));
        }
    }

    let cancel_token = Arc::new(AtomicBool::new(false));
    download_state
        .cancellation_tokens
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .insert(model_id.clone(), cancel_token.clone());
    download_state
        .progress
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .insert(model_id.clone(), 0.0);

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let model_dir = data_dir.join("models").join(descriptor.dir_name);

    if model_dir.exists() {
        std::fs::remove_dir_all(&model_dir)
            .map_err(|e| format!("Failed to clean up model directory: {e}"))?;
    }
    std::fs::create_dir_all(&model_dir)
        .map_err(|e| format!("Failed to create model directory: {e}"))?;

    let client = reqwest::Client::new();
    let download_pairs = descriptor.download_pairs();
    let total_files = download_pairs.len();
    let result: Result<(), String> = async {
        for (file_index, (url_name, local_name)) in download_pairs.iter().enumerate() {
            if cancel_token.load(Ordering::Relaxed) {
                return Err("Download cancelled".to_string());
            }

            let url = format!("{}/{}", descriptor.download_base_url, url_name);
            let dest_path = model_dir.join(local_name);

            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to download {local_name}: {e}"))?;

            let status = response.status();
            if !status.is_success() {
                return Err(format!(
                    "Failed to download {local_name}: HTTP {}",
                    status.as_u16()
                ));
            }

            let total_size = response
                .content_length()
                .unwrap_or_else(|| descriptor.size_bytes / total_files as u64);

            let mut file = tokio::fs::File::create(&dest_path)
                .await
                .map_err(|e| format!("Failed to create file {local_name}: {e}"))?;

            let mut downloaded: u64 = 0;
            let mut stream = response.bytes_stream();
            let mut last_reported_progress = -1.0_f64;

            while let Some(chunk_result) = stream.next().await {
                if cancel_token.load(Ordering::Relaxed) {
                    return Err("Download cancelled".to_string());
                }

                let chunk = chunk_result
                    .map_err(|e| format!("Error reading response for {local_name}: {e}"))?;
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Error writing file {local_name}: {e}"))?;

                downloaded += chunk.len() as u64;

                let file_progress = (downloaded as f64) / (total_size as f64).max(1.0);
                let overall_progress = (file_index as f64 + file_progress) / total_files as f64;

                if overall_progress - last_reported_progress >= 0.005 || downloaded >= total_size {
                    last_reported_progress = overall_progress;
                    download_state
                        .progress
                        .lock()
                        .map_err(|_| "Download state is unavailable".to_string())?
                        .insert(model_id.clone(), overall_progress);
                    let _ = app.emit(
                        "model-download-progress",
                        ModelDownloadProgressEvent {
                            model_id: model_id.clone(),
                            progress: overall_progress,
                            status: DownloadStatus::Downloading,
                        },
                    );
                }
            }

            file.flush()
                .await
                .map_err(|e| format!("Error flushing file {local_name}: {e}"))?;
        }

        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = std::fs::remove_dir_all(&model_dir);
    }

    download_state
        .cancellation_tokens
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .remove(&model_id);
    download_state
        .progress
        .lock()
        .map_err(|_| "Download state is unavailable".to_string())?
        .remove(&model_id);

    let (final_progress, final_status) = match &result {
        Ok(()) => (1.0, DownloadStatus::Completed),
        Err(msg) if msg == "Download cancelled" => (0.0, DownloadStatus::Cancelled),
        Err(_) => (0.0, DownloadStatus::Failed),
    };

    let _ = app.emit(
        "model-download-progress",
        ModelDownloadProgressEvent {
            model_id: model_id.clone(),
            progress: final_progress,
            status: final_status,
        },
    );

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

    {
        let model_info = transcription_state
            .model_info
            .lock()
            .map_err(|_| "Transcription state is unavailable".to_string())?;
        if let Some(ref loaded_id) = model_info.loaded_model_id {
            if loaded_id == &model_id {
                return Err(
                    "Cannot delete model that is currently in use. Please select a different model first."
                        .to_string(),
                );
            }
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

    let descriptor = MODEL_REGISTRY
        .iter()
        .find(|d| d.id == model_id)
        .ok_or_else(|| format!("Unknown model: {model_id}"))?;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let model_dir = data_dir.join("models").join(descriptor.dir_name);

    if model_dir.exists() {
        std::fs::remove_dir_all(&model_dir).map_err(|e| format!("Failed to delete model: {e}"))?;
    }

    Ok(())
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
