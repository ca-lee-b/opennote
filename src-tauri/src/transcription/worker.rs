use crate::transcription::audio::AudioCapture;
use crate::transcription::whisper::WhisperModel;
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
pub struct TranscriptionSegment {
    pub text: String,
    pub start_time_secs: f64,
    pub end_time_secs: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub model_id: String,
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
}

enum WorkerCommand {
    LoadModel {
        model_path: PathBuf,
        model_id: String,
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

    pub fn load_model(&self, model_path: PathBuf, model_id: String) -> Result<(), String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::LoadModel {
                model_path,
                model_id,
                reply: reply_tx,
            })
            .map_err(|error| format!("Failed to send load command: {error}"))?;
        reply_rx
            .recv()
            .map_err(|error| format!("Worker did not respond: {error}"))?
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
            .map_err(|error| format!("Failed to send start command: {error}"))?;
        reply_rx
            .recv()
            .map_err(|error| format!("Worker did not respond: {error}"))?
    }

    pub fn stop_recording(&self) -> Result<RecordingData, String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::StopRecording { reply: reply_tx })
            .map_err(|error| format!("Failed to send stop command: {error}"))?;
        reply_rx
            .recv()
            .map_err(|error| format!("Worker did not respond: {error}"))?
    }

    pub fn transcribe_file(&self, wav_path: PathBuf) -> Result<TranscriptionResult, String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::TranscribeFile {
                wav_path,
                reply: reply_tx,
            })
            .map_err(|error| format!("Failed to send transcribe command: {error}"))?;
        reply_rx
            .recv()
            .map_err(|error| format!("Worker did not respond: {error}"))?
    }
}

fn worker_loop(cmd_rx: Receiver<WorkerCommand>) {
    let mut model: Option<WhisperModel> = None;
    let mut loaded_model_id: Option<String> = None;
    let mut audio_capture: Option<AudioCapture> = None;
    let mut recording_start: Option<Instant> = None;
    let mut recording_wall_start: Option<SystemTime> = None;
    let mut saved_wav_path: Option<PathBuf> = None;

    for command in cmd_rx {
        match command {
            WorkerCommand::LoadModel {
                model_path,
                model_id,
                reply,
            } => {
                let result = WhisperModel::load(&model_path).map(|loaded_model| {
                    log::info!("Loaded Whisper model from {}", model_path.display());
                    model = Some(loaded_model);
                    loaded_model_id = Some(model_id);
                });
                let _ = reply.send(result);
            }
            WorkerCommand::StartRecording {
                app,
                wav_path,
                source,
                reply,
            } => {
                if model.is_none() {
                    let _ = reply.send(Err("No model loaded".to_string()));
                    continue;
                }
                if audio_capture.is_some() {
                    let _ = reply.send(Err("Already recording".to_string()));
                    continue;
                }

                match crate::transcription::audio::start_audio_capture(
                    app,
                    wav_path.clone(),
                    source,
                    None,
                ) {
                    Ok(capture) => {
                        saved_wav_path = Some(wav_path);
                        audio_capture = Some(capture);
                        recording_start = Some(Instant::now());
                        recording_wall_start = Some(SystemTime::now());
                        let _ = reply.send(Ok(()));
                    }
                    Err(error) => {
                        let _ = reply.send(Err(format!("Failed to start audio capture: {error}")));
                    }
                }
            }
            WorkerCommand::StopRecording { reply } => {
                let Some(capture) = &audio_capture else {
                    let _ = reply.send(Err("Not recording".to_string()));
                    continue;
                };

                if let Err(error) = capture.finalize_wav() {
                    let _ = reply.send(Err(error));
                    continue;
                }
                audio_capture = None;

                let duration = recording_start
                    .take()
                    .map(|start| start.elapsed().as_secs_f64())
                    .unwrap_or_default();
                let started_at = recording_wall_start
                    .take()
                    .and_then(|start| OffsetDateTime::from(start).format(&Rfc3339).ok());
                let audio_path = saved_wav_path.take().map(|path| path.display().to_string());

                let _ = reply.send(Ok(RecordingData {
                    duration,
                    model_id: loaded_model_id.clone().unwrap_or_default(),
                    started_at,
                    audio_path,
                }));
            }
            WorkerCommand::TranscribeFile { wav_path, reply } => {
                let result = match &mut model {
                    Some(model) => model.transcribe_file(&wav_path).map(|output| {
                        log::info!("Transcription complete: {} chars", output.text.len());
                        TranscriptionResult {
                            model_id: loaded_model_id.clone().unwrap_or_default(),
                            text: output.text,
                            segments: output.segments,
                        }
                    }),
                    None => Err("No model loaded".to_string()),
                };
                if let Err(error) = &result {
                    log::error!("Transcription failed: {error}");
                }
                let _ = reply.send(result);
            }
        }
    }
}
