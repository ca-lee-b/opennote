use std::path::Path;
use transcribe_rs::onnx::moonshine::StreamingModel as MoonshineStreamingModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::transcriber::{
    EnergyAdaptiveChunked, EnergyAdaptiveConfig, Transcriber,
};
use transcribe_rs::SpeechModel;
use transcribe_rs::TranscribeOptions;

pub struct StreamingModel {
    inner: MoonshineStreamingModel,
}

impl StreamingModel {
    pub fn load(model_dir: &Path) -> Result<Self, String> {
        let thread_count = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        let inner = MoonshineStreamingModel::load(model_dir, thread_count, &Quantization::FP32)
            .map_err(|error| error.to_string())?;

        Ok(Self { inner })
    }

    pub fn as_speech_model(&mut self) -> &mut dyn SpeechModel {
        &mut self.inner
    }

    pub fn transcribe_file(&mut self, wav_path: &Path) -> Result<String, String> {
        let config = EnergyAdaptiveConfig {
            target_chunk_secs: 15.0,
            search_window_secs: 2.0,
            ..Default::default()
        };
        let mut transcriber = EnergyAdaptiveChunked::new(config, TranscribeOptions::default());
        let result = transcriber
            .transcribe_file(&mut self.inner, wav_path)
            .map_err(|error| error.to_string())?;
        let text = result.text.trim().to_string();

        if text.is_empty() {
            return Err("Moonshine did not detect any speech in the recording".to_string());
        }

        Ok(text)
    }
}
