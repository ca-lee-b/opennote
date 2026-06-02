use crate::transcription::worker::TranscriptionSegment;
use std::path::Path;
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};

pub struct WhisperTranscription {
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
}

pub struct WhisperModel {
    engine: WhisperEngine,
}

impl WhisperModel {
    pub fn load(model_path: &Path) -> Result<Self, String> {
        let engine = WhisperEngine::load(model_path)
            .map_err(|error| format!("Failed to load Whisper model: {error}"))?;
        Ok(Self { engine })
    }

    pub fn transcribe_file(&mut self, wav_path: &Path) -> Result<WhisperTranscription, String> {
        let samples = transcribe_rs::audio::read_wav_samples(wav_path)
            .map_err(|error| format!("Failed to read WAV file: {error}"))?;
        let output = self
            .engine
            .transcribe_with(&samples, &WhisperInferenceParams::default())
            .map_err(|error| format!("Whisper transcription failed: {error}"))?;

        let text = output.text.trim().to_string();
        let segments = output
            .segments
            .unwrap_or_default()
            .into_iter()
            .filter_map(|segment| {
                let text = segment.text.trim().to_string();
                (!text.is_empty()).then_some(TranscriptionSegment {
                    text,
                    start_time_secs: f64::from(segment.start),
                    end_time_secs: f64::from(segment.end),
                })
            })
            .collect::<Vec<_>>();

        if text.is_empty() || segments.is_empty() {
            return Err("Whisper did not detect any speech in the recording".to_string());
        }

        Ok(WhisperTranscription { text, segments })
    }
}
