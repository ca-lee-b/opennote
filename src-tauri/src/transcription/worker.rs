use crate::transcription::audio::AudioCapture;
use crate::transcription::whisper::WhisperModel;
use crate::transcription::AudioSource;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tauri::{AppHandle, Emitter};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const PREVIEW_SAMPLE_RATE: usize = 16_000;
const PREVIEW_INITIAL_SECS: usize = 6;
const PREVIEW_INTERVAL_SECS: usize = 3;
const PREVIEW_WINDOW_SECS: usize = 24;

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
pub struct TranscriptionPreviewEvent {
    pub end_time_secs: f64,
    pub is_final: bool,
    pub sequence: u64,
    pub start_time_secs: f64,
    pub text: String,
}

enum WorkerCommand {
    LoadModel {
        model_path: PathBuf,
        model_id: String,
        reply: Sender<Result<(), String>>,
    },
    StartRecording {
        app: AppHandle,
        live_preview_enabled: bool,
        wav_path: PathBuf,
        source: AudioSource,
        reply: Sender<Result<(), String>>,
    },
    StopRecording {
        reply: Sender<Result<RecordingData, String>>,
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
        live_preview_enabled: bool,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.cmd_tx
            .send(WorkerCommand::StartRecording {
                app,
                live_preview_enabled,
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
}

fn worker_loop(cmd_rx: Receiver<WorkerCommand>) {
    let mut model: Option<WhisperModel> = None;
    let mut loaded_model_id: Option<String> = None;
    let mut loaded_model_path: Option<PathBuf> = None;
    let mut audio_capture: Option<AudioCapture> = None;
    let mut preview: Option<StreamingPreviewHandle> = None;
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
                    loaded_model_path = Some(model_path);
                });
                let _ = reply.send(result);
            }
            WorkerCommand::StartRecording {
                app,
                live_preview_enabled,
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

                let preview_input = if live_preview_enabled {
                    let Some(model_path) = loaded_model_path.clone() else {
                        let _ = reply.send(Err("No model path loaded".to_string()));
                        continue;
                    };
                    let (sample_tx, sample_rx) = std::sync::mpsc::channel();
                    Some((model_path, sample_tx, sample_rx))
                } else {
                    None
                };

                match crate::transcription::audio::start_audio_capture(
                    app.clone(),
                    wav_path.clone(),
                    source,
                    preview_input
                        .as_ref()
                        .map(|(_, sample_tx, _)| sample_tx.clone()),
                ) {
                    Ok(capture) => {
                        if let Some((model_path, _, sample_rx)) = preview_input {
                            preview =
                                Some(StreamingPreviewHandle::start(app, model_path, sample_rx));
                        }
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
                    if let Some(preview_handle) = preview.take() {
                        preview_handle.stop();
                    }
                    let _ = reply.send(Err(error));
                    continue;
                }
                audio_capture = None;
                if let Some(preview_handle) = preview.take() {
                    preview_handle.stop();
                }

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
        }
    }
}

struct StreamingPreviewHandle {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl StreamingPreviewHandle {
    fn start(app: AppHandle, model_path: PathBuf, sample_rx: Receiver<Vec<f32>>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("transcription-preview-worker".to_string())
            .spawn(move || run_streaming_preview(app, model_path, sample_rx, stop_thread))
            .ok();

        Self { stop, thread }
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_streaming_preview(
    app: AppHandle,
    model_path: PathBuf,
    sample_rx: Receiver<Vec<f32>>,
    stop: Arc<AtomicBool>,
) {
    let model = match WhisperModel::load(&model_path) {
        Ok(model) => model,
        Err(error) => {
            log::error!("Failed to load preview Whisper model: {error}");
            return;
        }
    };

    let mut buffer = Vec::<f32>::new();
    let mut next_inference_at = preview_initial_samples();
    let mut last_emitted_text = String::new();
    let mut sequence = 0_u64;

    while !stop.load(Ordering::Relaxed) {
        match sample_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(samples) => buffer.extend(samples),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        while let Ok(samples) = sample_rx.try_recv() {
            buffer.extend(samples);
        }

        if buffer.len() < next_inference_at {
            continue;
        }

        let (window_start, offset_secs) = preview_window_start(buffer.len());
        let window = buffer[window_start..].to_vec();
        next_inference_at = buffer.len() + preview_interval_samples();

        let output = match model.transcribe_samples(&window, offset_secs) {
            Ok(output) => output,
            Err(error) => {
                log::debug!("Preview transcription skipped: {error}");
                continue;
            }
        };

        let Some(preview) = preview_event_from_segments(&output.segments, sequence + 1) else {
            continue;
        };

        if preview.text == last_emitted_text {
            continue;
        }

        sequence = preview.sequence;
        last_emitted_text = preview.text.clone();
        let _ = app.emit("transcription-preview", preview);
    }
}

fn preview_initial_samples() -> usize {
    PREVIEW_SAMPLE_RATE * PREVIEW_INITIAL_SECS
}

fn preview_interval_samples() -> usize {
    PREVIEW_SAMPLE_RATE * PREVIEW_INTERVAL_SECS
}

fn preview_window_start(buffer_len: usize) -> (usize, f64) {
    let max_samples = PREVIEW_SAMPLE_RATE * PREVIEW_WINDOW_SECS;
    let start = buffer_len.saturating_sub(max_samples);
    (start, start as f64 / PREVIEW_SAMPLE_RATE as f64)
}

fn preview_event_from_segments(
    segments: &[TranscriptionSegment],
    sequence: u64,
) -> Option<TranscriptionPreviewEvent> {
    let non_empty_segments = segments
        .iter()
        .filter(|segment| !segment.text.trim().is_empty())
        .collect::<Vec<_>>();

    let first = non_empty_segments.first()?;
    let last = non_empty_segments.last()?;
    let text = non_empty_segments
        .iter()
        .map(|segment| segment.text.trim())
        .collect::<Vec<_>>()
        .join(" ");

    Some(TranscriptionPreviewEvent {
        end_time_secs: last.end_time_secs,
        is_final: false,
        sequence,
        start_time_secs: first.start_time_secs,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        preview_event_from_segments, preview_initial_samples, preview_interval_samples,
        preview_window_start, TranscriptionSegment, PREVIEW_SAMPLE_RATE,
    };

    #[test]
    fn preview_thresholds_are_in_sample_units() {
        assert_eq!(preview_initial_samples(), PREVIEW_SAMPLE_RATE * 6);
        assert_eq!(preview_interval_samples(), PREVIEW_SAMPLE_RATE * 3);
    }

    #[test]
    fn preview_window_timestamp_offset_is_absolute() {
        let buffer_len = PREVIEW_SAMPLE_RATE * 30;
        let (start, offset_secs) = preview_window_start(buffer_len);

        assert_eq!(start, PREVIEW_SAMPLE_RATE * 6);
        assert_eq!(offset_secs, 6.0);
    }

    #[test]
    fn preview_window_keeps_short_buffers_at_zero_offset() {
        let (start, offset_secs) = preview_window_start(PREVIEW_SAMPLE_RATE * 10);

        assert_eq!(start, 0);
        assert_eq!(offset_secs, 0.0);
    }

    #[test]
    fn preview_event_combines_window_segments() {
        let event = preview_event_from_segments(
            &[
                TranscriptionSegment {
                    text: "First phrase".to_string(),
                    start_time_secs: 3.0,
                    end_time_secs: 5.0,
                },
                TranscriptionSegment {
                    text: " second phrase ".to_string(),
                    start_time_secs: 5.2,
                    end_time_secs: 8.0,
                },
            ],
            7,
        )
        .expect("preview event");

        assert_eq!(event.sequence, 7);
        assert_eq!(event.start_time_secs, 3.0);
        assert_eq!(event.end_time_secs, 8.0);
        assert_eq!(event.text, "First phrase second phrase");
    }

    #[test]
    fn empty_preview_segments_are_suppressed() {
        assert!(preview_event_from_segments(
            &[TranscriptionSegment {
                text: " ".to_string(),
                start_time_secs: 1.0,
                end_time_secs: 8.0,
            }],
            1
        )
        .is_none());
    }
}
