use super::model::{ChunkWindow, CHUNK_LOGICAL_SECS, CHUNK_OVERLAP_SECS};
use crate::transcription::whisper::WHISPER_SAMPLE_RATE;
use crate::transcription::worker::TranscriptionSegment;
use std::path::Path;

pub(super) fn plan_chunk_windows(duration_secs: f64) -> Vec<ChunkWindow> {
    let duration_secs = duration_secs.max(0.0);
    if duration_secs == 0.0 {
        return Vec::new();
    }
    let mut windows = Vec::new();
    let mut logical_start_secs = 0.0;
    let mut index = 0_i64;

    while logical_start_secs < duration_secs {
        let logical_end_secs = (logical_start_secs + CHUNK_LOGICAL_SECS).min(duration_secs);
        let source_start_secs = if index == 0 {
            0.0
        } else {
            (logical_start_secs - CHUNK_OVERLAP_SECS).max(0.0)
        };
        let source_end_secs = if logical_end_secs >= duration_secs {
            duration_secs
        } else {
            (logical_end_secs + CHUNK_OVERLAP_SECS).min(duration_secs)
        };
        windows.push(ChunkWindow {
            index,
            logical_start_secs,
            logical_end_secs,
            source_start_secs,
            source_end_secs,
        });
        logical_start_secs += CHUNK_LOGICAL_SECS;
        index += 1;
    }

    windows
}

pub(super) fn write_chunk_wav(
    samples: &[f32],
    start_secs: f64,
    end_secs: f64,
    path: &Path,
) -> Result<(), String> {
    let start = seconds_to_sample_index(start_secs, samples.len());
    let end = seconds_to_sample_index(end_secs, samples.len()).max(start);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: WHISPER_SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|error| format!("Failed to create chunk WAV: {error}"))?;
    for sample in &samples[start..end] {
        let value = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(value)
            .map_err(|error| format!("Failed to write chunk WAV sample: {error}"))?;
    }
    writer
        .finalize()
        .map_err(|error| format!("Failed to finalize chunk WAV: {error}"))?;
    Ok(())
}

pub(super) fn clip_segments_to_logical_window(
    segments: Vec<TranscriptionSegment>,
    logical_start_secs: f64,
    logical_end_secs: f64,
) -> Vec<TranscriptionSegment> {
    segments
        .into_iter()
        .filter(|segment| {
            segment.start_time_secs >= logical_start_secs
                && segment.start_time_secs < logical_end_secs
                && !segment.text.trim().is_empty()
        })
        .collect()
}

fn seconds_to_sample_index(seconds: f64, sample_count: usize) -> usize {
    ((seconds.max(0.0) * WHISPER_SAMPLE_RATE).round() as usize).min(sample_count)
}

#[cfg(test)]
mod tests {
    use super::{clip_segments_to_logical_window, plan_chunk_windows};
    use crate::transcription::pipeline::model::ChunkWindow;
    use crate::transcription::worker::TranscriptionSegment;

    #[test]
    fn plans_short_recording_as_one_chunk_without_overlap() {
        assert_eq!(
            plan_chunk_windows(120.0),
            vec![ChunkWindow {
                index: 0,
                logical_start_secs: 0.0,
                logical_end_secs: 120.0,
                source_start_secs: 0.0,
                source_end_secs: 120.0,
            }]
        );
    }

    #[test]
    fn plans_long_recording_with_boundary_overlap() {
        assert_eq!(
            plan_chunk_windows(620.0),
            vec![
                ChunkWindow {
                    index: 0,
                    logical_start_secs: 0.0,
                    logical_end_secs: 300.0,
                    source_start_secs: 0.0,
                    source_end_secs: 315.0,
                },
                ChunkWindow {
                    index: 1,
                    logical_start_secs: 300.0,
                    logical_end_secs: 600.0,
                    source_start_secs: 285.0,
                    source_end_secs: 615.0,
                },
                ChunkWindow {
                    index: 2,
                    logical_start_secs: 600.0,
                    logical_end_secs: 620.0,
                    source_start_secs: 585.0,
                    source_end_secs: 620.0,
                },
            ]
        );
    }

    #[test]
    fn clips_overlap_segments_by_logical_start() {
        let segments = clip_segments_to_logical_window(
            vec![
                TranscriptionSegment {
                    text: "previous".to_string(),
                    start_time_secs: 298.0,
                    end_time_secs: 301.0,
                },
                TranscriptionSegment {
                    text: "current".to_string(),
                    start_time_secs: 300.0,
                    end_time_secs: 302.0,
                },
                TranscriptionSegment {
                    text: "next".to_string(),
                    start_time_secs: 600.0,
                    end_time_secs: 602.0,
                },
            ],
            300.0,
            600.0,
        );

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "current");
    }
}
