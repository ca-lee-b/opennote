use crate::transcription::audio::AudioCapture;
use crate::transcription::moonshine::StreamingModel as MoonshineModel;
use crate::transcription::parakeet::ParakeetModel;
use crate::transcription::ModelArch;
use crate::transcription::AudioSource;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Instant, SystemTime};
use tauri::AppHandle;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingData {
    pub duration: f64,
    pub model_id: String,
    pub started_at: Option<String>,
    pub audio_path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub model_id: String,
}

enum LoadedModel {
    Moonshine(MoonshineModel),
    Parakeet(ParakeetModel),
}

enum WorkerCommand {
    LoadModel {
        model_dir: PathBuf,
        arch: ModelArch,
        reply: Sender<Result<(), String>>,
    },
    StartRecording {
        app: AppHandle,
        wav_path: PathBuf,
        source: AudioSource,
        reply: Sender<Result<(), String>>,
    },
    StopRecording {
        reply: Sender<Result<RecordingData, String>>,
    },
    TranscribeFile {
        wav_path: PathBuf,
        reply: Sender<Result<TranscriptionResult, String>>,
    },
}

pub struct TranscriptionWorker {
    cmd_tx: Sender<WorkerCommand>,
    #[allow(dead_code)]
    thread: Option<std::thread::JoinHandle<()>>,
}

impl TranscriptionWorker {
    pub fn start() -> Self {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();

        let thread = std::thread::Builder::new()
            .name("transcription-worker".to_string())
            .spawn(move || worker_loop(cmd_rx))
            .expect("Failed to start transcription worker");

        TranscriptionWorker {
            cmd_tx,
            thread: Some(thread),
        }
    }

    pub fn load_model(&self, model_dir: PathBuf, arch: ModelArch) -> Result<(), String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::LoadModel {
                model_dir,
                arch,
                reply: reply_tx,
            })
            .map_err(|e| format!("Failed to send load command: {e}"))?;
        reply_rx
            .recv()
            .map_err(|e| format!("Worker did not respond: {e}"))?
    }

    pub fn start_recording(
        &self,
        app: AppHandle,
        wav_path: PathBuf,
        source: AudioSource,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::StartRecording {
                app,
                wav_path,
                source,
                reply: reply_tx,
            })
            .map_err(|e| format!("Failed to send start command: {e}"))?;
        reply_rx
            .recv()
            .map_err(|e| format!("Worker did not respond: {e}"))?
    }

    pub fn stop_recording(&self) -> Result<RecordingData, String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::StopRecording { reply: reply_tx })
            .map_err(|e| format!("Failed to send stop command: {e}"))?;
        reply_rx
            .recv()
            .map_err(|e| format!("Worker did not respond: {e}"))?
    }

    pub fn transcribe_file(&self, wav_path: PathBuf) -> Result<TranscriptionResult, String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::TranscribeFile {
                wav_path,
                reply: reply_tx,
            })
            .map_err(|e| format!("Failed to send transcribe command: {e}"))?;
        reply_rx
            .recv()
            .map_err(|e| format!("Worker did not respond: {e}"))?
    }
}

fn worker_loop(cmd_rx: Receiver<WorkerCommand>) {
    let mut model: Option<LoadedModel> = None;
    let mut is_recording = false;
    let mut audio_capture: Option<AudioCapture> = None;
    let mut recording_start: Option<Instant> = None;
    let mut recording_wall_start: Option<SystemTime> = None;
    let mut saved_wav_path: Option<PathBuf> = None;
    let mut loaded_model_id: Option<String> = String::new().into();

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            WorkerCommand::LoadModel {
                model_dir,
                arch,
                reply,
            } => {
                let result = match arch {
                    ModelArch::Small | ModelArch::Medium => {
                        match MoonshineModel::load(&model_dir) {
                            Ok(m) => {
                                loaded_model_id = Some(
                                    model_dir
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                );
                                model = Some(LoadedModel::Moonshine(m));
                                log::info!(
                                    "Loaded Moonshine streaming model from {}",
                                    model_dir.display()
                                );
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    ModelArch::ParakeetTdt => match ParakeetModel::load(&model_dir) {
                        Ok(m) => {
                            loaded_model_id = Some(
                                model_dir
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                            );
                            model = Some(LoadedModel::Parakeet(m));
                            log::info!(
                                "Loaded Parakeet TDT model from {}",
                                model_dir.display()
                            );
                            Ok(())
                        }
                        Err(e) => Err(e),
                    },
                };
                let _ = reply.send(result);
            }

            WorkerCommand::StartRecording {
                app: capture_app,
                wav_path,
                source,
                reply,
            } => {
                if model.is_none() {
                    let _ = reply.send(Err("No model loaded".to_string()));
                    continue;
                }
                if is_recording {
                    let _ = reply.send(Err("Already recording".to_string()));
                    continue;
                }

                let capture_result = crate::transcription::audio::start_audio_capture(
                    capture_app,
                    wav_path.clone(),
                    source,
                );

                match capture_result {
                    Ok(capture) => {
                        saved_wav_path = Some(wav_path);
                        audio_capture = Some(capture);
                    }
                    Err(e) => {
                        let _ = reply.send(Err(format!("Failed to start audio capture: {e}")));
                        continue;
                    }
                }

                is_recording = true;
                recording_start = Some(Instant::now());
                recording_wall_start = Some(SystemTime::now());

                let _ = reply.send(Ok(()));
            }

            WorkerCommand::StopRecording { reply } => {
                if !is_recording {
                    let _ = reply.send(Err("Not recording".to_string()));
                    continue;
                }

                if let Some(ref capture) = audio_capture {
                    if let Err(error) = capture.finalize_wav() {
                        let _ = reply.send(Err(error));
                        continue;
                    }
                }

                let duration = recording_start
                    .map(|t| t.elapsed().as_secs_f64())
                    .unwrap_or_default();
                let started_at = recording_wall_start
                    .and_then(|t| OffsetDateTime::from(t).format(&Rfc3339).ok());

                let audio_path = saved_wav_path.as_ref().map(|p| p.display().to_string());

                is_recording = false;
                audio_capture = None;

                let _ = reply.send(Ok(RecordingData {
                    duration,
                    model_id: loaded_model_id.clone().unwrap_or_default(),
                    started_at,
                    audio_path,
                }));

                recording_start = None;
                recording_wall_start = None;
            }

            WorkerCommand::TranscribeFile { wav_path, reply } => {
                let result = match &mut model {
                    Some(LoadedModel::Moonshine(m)) => match m.transcribe_file(&wav_path) {
                        Ok(output) => {
                            let trimmed = output.trim().to_string();
                            log::info!("Transcription complete: {} chars", trimmed.len());
                            Ok(TranscriptionResult {
                                text: trimmed,
                                model_id: loaded_model_id.clone().unwrap_or_default(),
                            })
                        }
                        Err(e) => {
                            log::error!("Transcription failed: {e}");
                            Err(format!("Transcription failed: {e}"))
                        }
                    },
                    Some(LoadedModel::Parakeet(m)) => match m.transcribe_file(&wav_path) {
                        Ok(output) => {
                            let trimmed = output.trim().to_string();
                            log::info!("Transcription complete: {} chars", trimmed.len());
                            Ok(TranscriptionResult {
                                text: trimmed,
                                model_id: loaded_model_id.clone().unwrap_or_default(),
                            })
                        }
                        Err(e) => {
                            log::error!("Transcription failed: {e}");
                            Err(format!("Transcription failed: {e}"))
                        }
                    },
                    None => Err("No model loaded".to_string()),
                };
                let _ = reply.send(result);
            }
        }
    }
}