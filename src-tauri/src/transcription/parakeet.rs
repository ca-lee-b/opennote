use std::path::Path;

use parakeet_rs::{ParakeetTDT, Transcriber, TimestampMode};

pub struct ParakeetModel {
    inner: ParakeetTDT,
}

impl ParakeetModel {
    pub fn load(model_dir: &Path) -> Result<Self, String> {
        let inner = ParakeetTDT::from_pretrained(model_dir, None)
            .map_err(|error| format!("Failed to load Parakeet model: {error}"))?;

        Ok(Self { inner })
    }

    pub fn transcribe_file(&mut self, wav_path: &Path) -> Result<String, String> {
        let result = self
            .inner
            .transcribe_file(wav_path, Some(TimestampMode::Sentences))
            .map_err(|error| format!("Parakeet transcription failed: {error}"))?;

        let text = result.text.trim().to_string();

        if text.is_empty() {
            return Err("Parakeet did not detect any speech in the recording".to_string());
        }

        Ok(text)
    }
}