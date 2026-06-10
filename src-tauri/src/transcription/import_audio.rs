use std::fs::File;
use std::path::{Path, PathBuf};
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tauri::{AppHandle, Manager};

pub(super) struct PreparedImport {
    pub(super) duration: f64,
    pub(super) wav_path: PathBuf,
}

struct DecodedAudio {
    samples: Vec<f32>,
    sample_rate: u32,
}

pub(super) fn prepare_imported_audio(
    source_path: &Path,
    audio_dir: &Path,
) -> Result<PreparedImport, String> {
    prepare_imported_audio_with_decoder(source_path, audio_dir, decode_mp3)
}

pub(super) fn audio_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data dir: {error}"))?
        .join("audio"))
}

fn prepare_imported_audio_with_decoder(
    source_path: &Path,
    audio_dir: &Path,
    decode_mp3: impl Fn(&Path) -> Result<DecodedAudio, String>,
) -> Result<PreparedImport, String> {
    let extension = source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    std::fs::create_dir_all(audio_dir)
        .map_err(|error| format!("Failed to create audio import dir: {error}"))?;
    let wav_path = audio_dir.join(format!("{}.wav", uuid::Uuid::new_v4()));
    match extension.as_str() {
        "wav" => {
            std::fs::copy(source_path, &wav_path)
                .map_err(|error| format!("Failed to copy imported audio file: {error}"))?;
        }
        "mp3" => {
            let decoded = decode_mp3(source_path)?;
            write_wav(&wav_path, &decoded.samples, decoded.sample_rate)?;
        }
        _ => return Err("Imported audio must be a WAV or MP3 file.".to_string()),
    }

    Ok(PreparedImport {
        duration: wav_duration_secs(&wav_path)?,
        wav_path,
    })
}

fn decode_mp3(path: &Path) -> Result<DecodedAudio, String> {
    let file = File::open(path).map_err(|error| format!("Failed to open imported MP3: {error}"))?;
    let media_source = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            media_source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| format!("Failed to inspect imported MP3: {error}"))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| "Imported MP3 has no audio track.".to_string())?;
    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| "Imported MP3 has no sample rate.".to_string())?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|error| format!("Failed to decode imported MP3: {error}"))?;
    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(error) => return Err(format!("Failed to read imported MP3 packet: {error}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(error) => return Err(format!("Failed to decode imported MP3 packet: {error}")),
        };
        append_mono_samples(decoded, &mut samples);
    }

    if samples.is_empty() {
        return Err("Imported MP3 did not contain decodable audio.".to_string());
    }

    Ok(DecodedAudio {
        samples,
        sample_rate,
    })
}

fn wav_duration_secs(path: &Path) -> Result<f64, String> {
    let reader = hound::WavReader::open(path)
        .map_err(|error| format!("Failed to read imported WAV file: {error}"))?;
    let spec = reader.spec();
    if spec.sample_rate == 0 {
        return Err("Imported WAV file has an invalid sample rate.".to_string());
    }
    let channels = u32::from(spec.channels.max(1));
    Ok(reader.duration() as f64 / channels as f64 / spec.sample_rate as f64)
}

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    if sample_rate == 0 {
        return Err("Imported audio has an invalid sample rate.".to_string());
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|error| format!("Failed to create imported WAV file: {error}"))?;
    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        writer
            .write_sample((clamped * i16::MAX as f32) as i16)
            .map_err(|error| format!("Failed to write imported WAV file: {error}"))?;
    }
    writer
        .finalize()
        .map_err(|error| format!("Failed to finalize imported WAV file: {error}"))?;
    Ok(())
}

fn append_mono_samples(decoded: AudioBufferRef<'_>, samples: &mut Vec<f32>) {
    let channels = decoded.spec().channels.count();
    let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
    sample_buffer.copy_interleaved_ref(decoded);
    append_interleaved_mono(sample_buffer.samples(), channels, samples);
}

fn append_interleaved_mono(input: &[f32], channels: usize, samples: &mut Vec<f32>) {
    let channels = channels.max(1);
    for frame in input.chunks(channels) {
        samples.push(frame.iter().sum::<f32>() / frame.len() as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::{prepare_imported_audio, prepare_imported_audio_with_decoder, DecodedAudio};
    use hound::{SampleFormat, WavSpec, WavWriter};
    use std::path::Path;
    use tempfile::tempdir;

    fn write_test_wav(path: &Path) {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).expect("create wav");
        for _ in 0..16_000 {
            writer.write_sample::<i16>(0).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    #[test]
    fn prepares_wav_import_as_canonical_saved_audio() {
        let temp = tempdir().expect("tempdir");
        let source_path = temp.path().join("Interview With Sam.wav");
        let audio_dir = temp.path().join("audio");
        write_test_wav(&source_path);

        let prepared =
            prepare_imported_audio(&source_path, &audio_dir).expect("prepare imported wav");

        assert!(prepared.wav_path.starts_with(&audio_dir));
        assert_eq!(
            prepared.wav_path.extension().and_then(|ext| ext.to_str()),
            Some("wav")
        );
        assert!(prepared.wav_path.exists());
        assert!((prepared.duration - 1.0).abs() < 0.001);
    }

    #[test]
    fn prepares_mp3_import_as_decoded_canonical_wav() {
        let temp = tempdir().expect("tempdir");
        let source_path = temp.path().join("Interview With Sam.mp3");
        let audio_dir = temp.path().join("audio");
        std::fs::write(&source_path, b"fake mp3").expect("write fake mp3");

        let prepared = prepare_imported_audio_with_decoder(&source_path, &audio_dir, |_| {
            Ok(DecodedAudio {
                samples: vec![0.0; 16_000],
                sample_rate: 16_000,
            })
        })
        .expect("prepare imported mp3");

        assert!(prepared.wav_path.starts_with(&audio_dir));
        assert_eq!(
            prepared.wav_path.extension().and_then(|ext| ext.to_str()),
            Some("wav")
        );
        hound::WavReader::open(&prepared.wav_path).expect("read canonical wav");
        assert!((prepared.duration - 1.0).abs() < 0.001);
    }
}
