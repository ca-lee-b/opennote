use super::chunking::clip_segments_to_logical_window;
use super::model::POLL_INTERVAL;
use super::repository;
use crate::transcription::models;
use crate::transcription::whisper::WhisperModel;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use tauri::AppHandle;

pub(super) struct PipelineRuntime {
    chunker_tx: Sender<()>,
    transcriber_tx: Sender<()>,
    #[allow(dead_code)]
    chunker_thread: Option<std::thread::JoinHandle<()>>,
    #[allow(dead_code)]
    transcriber_thread: Option<std::thread::JoinHandle<()>>,
}

impl PipelineRuntime {
    pub(super) fn start(app: AppHandle) -> Self {
        let (chunker_tx, chunker_rx) = std::sync::mpsc::channel();
        let (transcriber_tx, transcriber_rx) = std::sync::mpsc::channel();
        let chunker_app = app.clone();
        let chunker_wake_transcriber = transcriber_tx.clone();
        let chunker_thread = std::thread::Builder::new()
            .name("recording-pipeline-chunker".to_string())
            .spawn(move || chunker_loop(chunker_app, chunker_rx, chunker_wake_transcriber))
            .ok();
        let transcriber_thread = std::thread::Builder::new()
            .name("recording-pipeline-transcriber".to_string())
            .spawn(move || transcriber_loop(app, transcriber_rx))
            .ok();

        Self {
            chunker_tx,
            transcriber_tx,
            chunker_thread,
            transcriber_thread,
        }
    }

    pub(super) fn wake(&self) {
        let _ = self.chunker_tx.send(());
        let _ = self.transcriber_tx.send(());
    }
}

fn chunker_loop(app: AppHandle, wake_rx: Receiver<()>, transcriber_tx: Sender<()>) {
    loop {
        let result = tauri::async_runtime::block_on(repository::process_next_chunking_job(&app));
        match result {
            Ok(true) => {
                let _ = transcriber_tx.send(());
                continue;
            }
            Ok(false) => {}
            Err(error) => log::error!("Recording chunker failed: {error}"),
        }
        let _ = wake_rx.recv_timeout(POLL_INTERVAL);
    }
}

fn transcriber_loop(app: AppHandle, wake_rx: Receiver<()>) {
    let mut loaded_model_id: Option<String> = None;
    let mut model: Option<WhisperModel> = None;

    loop {
        let work = tauri::async_runtime::block_on(repository::claim_next_transcription_chunk(&app));
        let work = match work {
            Ok(Some(work)) => work,
            Ok(None) => {
                let _ = wake_rx.recv_timeout(POLL_INTERVAL);
                continue;
            }
            Err(error) => {
                log::error!("Failed to claim transcription chunk: {error}");
                let _ = wake_rx.recv_timeout(POLL_INTERVAL);
                continue;
            }
        };

        if loaded_model_id.as_deref() != Some(work.model_id.as_str()) {
            match models::resolve_downloaded_model(&app, &work.model_id)
                .and_then(|path| WhisperModel::load(&path))
            {
                Ok(loaded_model) => {
                    model = Some(loaded_model);
                    loaded_model_id = Some(work.model_id.clone());
                }
                Err(error) => {
                    let _ =
                        tauri::async_runtime::block_on(repository::fail_chunk(&app, &work, error));
                    continue;
                }
            }
        }

        let Some(model) = model.as_ref() else {
            let _ = tauri::async_runtime::block_on(repository::fail_chunk(
                &app,
                &work,
                "No transcription model is loaded.".to_string(),
            ));
            continue;
        };

        let result = model
            .transcribe_file_with_offset(Path::new(&work.chunk_path), work.source_start_secs)
            .map(|output| {
                clip_segments_to_logical_window(
                    output.segments,
                    work.logical_start_secs,
                    work.logical_end_secs,
                )
            });

        match result {
            Ok(segments) => {
                if let Err(error) = tauri::async_runtime::block_on(repository::complete_chunk(
                    &app, &work, &segments,
                )) {
                    log::error!("Failed to complete transcription chunk: {error}");
                }
            }
            Err(error) => {
                let _ = tauri::async_runtime::block_on(repository::fail_chunk(&app, &work, error));
            }
        }
    }
}
