use crate::transcription::worker::TranscriptionSegment;
use hound::SampleFormat;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub(super) const WHISPER_SAMPLE_RATE: f64 = 16_000.0;

pub struct WhisperTranscription {
    pub segments: Vec<TranscriptionSegment>,
}

pub struct WhisperModel {
    context: WhisperContext,
}

impl WhisperModel {
    pub fn load(model_path: &Path) -> Result<Self, String> {
        let mut context_params = WhisperContextParameters::default();
        context_params.flash_attn = true;

        let context = WhisperContext::new_with_params(model_path, context_params)
            .map_err(|error| format!("Failed to load Whisper model: {error}"))?;
        Ok(Self { context })
    }

    pub fn transcribe_file_with_offset(
        &self,
        wav_path: &Path,
        offset_secs: f64,
    ) -> Result<WhisperTranscription, String> {
        let samples = read_wav_samples(wav_path)?;
        self.transcribe_samples(&samples, offset_secs)
    }

    pub fn transcribe_samples(
        &self,
        samples: &[f32],
        offset_secs: f64,
    ) -> Result<WhisperTranscription, String> {
        if samples.is_empty() {
            return Ok(WhisperTranscription {
                segments: Vec::new(),
            });
        }

        let mut state = self
            .context
            .create_state()
            .map_err(|error| format!("Failed to create Whisper state: {error}"))?;

        state
            .full(default_full_params(), samples)
            .map_err(|error| format!("Whisper transcription failed: {error}"))?;

        let mut segments = Vec::new();

        for segment in state.as_iter() {
            let segment_text = segment.to_string().trim().to_string();
            if segment_text.is_empty() {
                continue;
            }

            let start_time_secs = segment.start_timestamp() as f64 / 100.0 + offset_secs;
            let end_time_secs = segment.end_timestamp() as f64 / 100.0 + offset_secs;

            segments.push(TranscriptionSegment {
                text: segment_text,
                start_time_secs,
                end_time_secs,
            });
        }

        Ok(WhisperTranscription { segments })
    }
}

fn default_full_params() -> FullParams<'static, 'static> {
    let mut params = FullParams::new(SamplingStrategy::BeamSearch {
        beam_size: 3,
        patience: -1.0,
    });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_suppress_nst(true);
    params.set_no_speech_thold(0.2);
    params
}

pub(super) fn read_wav_samples(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|error| format!("Failed to read WAV file: {error}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>(),
        (SampleFormat::Int, 24 | 32) => reader
            .samples::<i32>()
            .map(|sample| sample.map(|value| value as f32 / i32::MAX as f32))
            .collect::<Result<Vec<_>, _>>(),
        (SampleFormat::Float, 32) => reader.samples::<f32>().collect::<Result<Vec<_>, _>>(),
        _ => {
            return Err(format!(
                "Unsupported WAV format: {:?} {} bits",
                spec.sample_format, spec.bits_per_sample
            ));
        }
    }
    .map_err(|error| format!("Failed to decode WAV samples: {error}"))?;

    let mono = downmix_to_mono(&samples, channels);
    if spec.sample_rate == WHISPER_SAMPLE_RATE as u32 {
        return Ok(mono);
    }

    Ok(linear_resample(
        &mono,
        spec.sample_rate as f64 / WHISPER_SAMPLE_RATE,
    ))
}

fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks(channels)
        .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
        .collect()
}

fn linear_resample(samples: &[f32], ratio: f64) -> Vec<f32> {
    if ratio <= 0.0 || samples.is_empty() {
        return samples.to_vec();
    }

    let output_len = ((samples.len() as f64) / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for index in 0..output_len {
        let src_idx = index as f64 * ratio;
        let idx0 = src_idx.floor() as usize;
        let frac = src_idx - idx0 as f64;
        let s0 = samples.get(idx0).copied().unwrap_or(0.0);
        let s1 = samples.get(idx0 + 1).copied().unwrap_or(s0);
        output.push(s0 + (s1 - s0) * frac as f32);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{downmix_to_mono, linear_resample};

    #[test]
    fn downmixes_stereo_samples() {
        assert_eq!(
            downmix_to_mono(&[1.0, -1.0, 0.5, 0.25], 2),
            vec![0.0, 0.375]
        );
    }

    #[test]
    fn resamples_when_source_rate_differs() {
        let samples = vec![0.0, 1.0, 0.0, -1.0];
        let resampled = linear_resample(&samples, 2.0);

        assert_eq!(resampled.len(), 2);
        assert_eq!(resampled, vec![0.0, 0.0]);
    }
}
