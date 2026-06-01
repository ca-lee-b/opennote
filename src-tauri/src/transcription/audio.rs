use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use hound::{WavSpec, WavWriter};
use serde::Serialize;
#[cfg(target_os = "macos")]
use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

use crate::transcription::AudioSource;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevelEvent {
    pub level: f32,
}

pub struct AudioCapture {
    _stream: Option<Stream>,
    #[cfg(target_os = "macos")]
    system_audio: Option<SystemAudioCapture>,
    writer: Option<WavWriterHandle>,
}

impl AudioCapture {
    pub fn finalize_wav(&self) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        if let Some(ref capture) = self.system_audio {
            capture.stop()?;
        }
        if let Some(ref w) = self.writer {
            finalize_wav(w)?;
        }
        Ok(())
    }
}

type WavWriterHandle = Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>;

pub fn start_audio_capture(
    app: AppHandle,
    wav_path: PathBuf,
    source: AudioSource,
    sample_tx: Option<std::sync::mpsc::Sender<Vec<f32>>>,
) -> Result<AudioCapture, String> {
    match source {
        AudioSource::Microphone => start_microphone_capture(app, wav_path, sample_tx),
        AudioSource::ComputerAudio => start_computer_audio_capture(app, wav_path, sample_tx),
    }
}

fn start_microphone_capture(
    app: AppHandle,
    wav_path: PathBuf,
    sample_tx: Option<std::sync::mpsc::Sender<Vec<f32>>>,
) -> Result<AudioCapture, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No audio input device available".to_string())?;

    let supported_config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get default input config: {e}"))?;

    let native_sample_rate = supported_config.sample_rate().0;
    let channels = supported_config.channels();
    let sample_format = supported_config.sample_format();
    let stream_config: StreamConfig = supported_config.into();

    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let w = WavWriter::create(&wav_path, spec)
        .map_err(|e| format!("Failed to create WAV file: {e}"))?;
    let writer: Option<WavWriterHandle> = Some(Arc::new(Mutex::new(Some(w))));
    let writer_clone = writer.clone();
    let resample_ratio = native_sample_rate as f64 / 16000.0;
    let app_emit = app.clone();
    let sample_tx_mic = sample_tx.clone();

    let capture_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mono = downmix(data, channels);
        let resampled = resample_if_needed(&mono, resample_ratio);

        if !resampled.is_empty() {
            let rms = compute_rms(&resampled);
            let level = (rms * 5.0).min(1.0);
            let _ = app_emit.emit("audio-level", AudioLevelEvent { level });
        }

        if let Some(ref tx) = sample_tx_mic {
            let _ = tx.send(resampled.clone());
        }

        if let Some(ref w) = writer_clone {
            if let Ok(mut guard) = w.try_lock() {
                if let Some(ref mut writer) = *guard {
                    for &sample in &resampled {
                        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        let _ = writer.write_sample(s);
                    }
                }
            }
        }
    };

    let stream = match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                &stream_config,
                capture_fn,
                |err| {
                    log::error!("Audio input error: {err}");
                },
                None,
            )
            .map_err(|e| format!("Failed to build audio stream: {e}"))?,
        SampleFormat::I16 => {
            let app_emit2 = app.clone();
            let writer_clone2 = writer.clone();
            let sample_tx_i16 = sample_tx;
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        let mono = downmix(&f32_data, channels);
                        let resampled = resample_if_needed(&mono, resample_ratio);

                        if !resampled.is_empty() {
                            let rms = compute_rms(&resampled);
                            let level = (rms * 5.0).min(1.0);
                            let _ = app_emit2.emit("audio-level", AudioLevelEvent { level });
                        }

                        if let Some(ref tx) = sample_tx_i16 {
                            let _ = tx.send(resampled.clone());
                        }

                        if let Some(ref w) = writer_clone2 {
                            if let Ok(mut guard) = w.try_lock() {
                                if let Some(ref mut writer) = *guard {
                                    for &sample in &resampled {
                                        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                        let _ = writer.write_sample(s);
                                    }
                                }
                            }
                        }
                    },
                    |err| log::error!("Audio input error: {err}"),
                    None,
                )
                .map_err(|e| format!("Failed to build audio stream: {e}"))?
        }
        _ => return Err("Unsupported audio sample format".to_string()),
    };

    stream
        .play()
        .map_err(|e| format!("Failed to start audio stream: {e}"))?;

    log::info!(
        "Audio capture started: {}Hz, {}ch, {:?} (resample ratio: {:.2})",
        native_sample_rate,
        channels,
        sample_format,
        resample_ratio,
    );

    Ok(AudioCapture {
        _stream: Some(stream),
        #[cfg(target_os = "macos")]
        system_audio: None,
        writer,
    })
}

#[cfg(target_os = "macos")]
struct SystemAudioCallbackContext {
    app: AppHandle,
    writer: WavWriterHandle,
    sample_tx: Option<std::sync::mpsc::Sender<Vec<f32>>>,
}

#[cfg(target_os = "macos")]
struct SystemAudioCapture {
    callback_context: *mut SystemAudioCallbackContext,
    handle: Mutex<Option<*mut c_void>>,
}

#[cfg(target_os = "macos")]
unsafe impl Send for SystemAudioCapture {}

#[cfg(target_os = "macos")]
unsafe impl Sync for SystemAudioCapture {}

#[cfg(target_os = "macos")]
impl SystemAudioCapture {
    fn stop(&self) -> Result<(), String> {
        let handle = self
            .handle
            .lock()
            .map_err(|e| format!("Computer audio capture lock error: {e}"))?
            .take();
        let Some(handle) = handle else {
            return Ok(());
        };

        let mut error = std::ptr::null_mut();
        // SAFETY: the handle was created by the Objective-C bridge and is consumed once.
        let stopped = unsafe { opennote_stop_system_audio_capture(handle, &mut error) };
        if stopped {
            // SAFETY: callbacks have stopped, so Rust owns the boxed callback context again.
            unsafe {
                drop(Box::from_raw(self.callback_context));
            }
            Ok(())
        } else {
            Err(take_bridge_error(error))
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for SystemAudioCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn opennote_start_system_audio_capture(
        callback: extern "C" fn(*const f32, usize, *mut c_void),
        context: *mut c_void,
        error: *mut *mut c_char,
    ) -> *mut c_void;
    fn opennote_stop_system_audio_capture(handle: *mut c_void, error: *mut *mut c_char) -> bool;
    fn opennote_free_error(error: *mut c_char);
}

#[cfg(target_os = "macos")]
extern "C" fn receive_system_audio(samples: *const f32, count: usize, context: *mut c_void) {
    if samples.is_null() || context.is_null() {
        return;
    }
    // SAFETY: the Objective-C bridge supplies valid samples only while the boxed context lives.
    let samples = unsafe { std::slice::from_raw_parts(samples, count) };
    // SAFETY: context points to SystemAudioCallbackContext until native capture stops.
    let context = unsafe { &*(context.cast::<SystemAudioCallbackContext>()) };
    write_samples(&context.writer, samples);
    emit_audio_level(&context.app, samples);
    if let Some(ref tx) = context.sample_tx {
        let _ = tx.send(samples.to_vec());
    }
}

#[cfg(target_os = "macos")]
fn take_bridge_error(error: *mut c_char) -> String {
    if error.is_null() {
        return "Computer audio capture failed".to_string();
    }
    // SAFETY: the bridge returns a null-terminated allocated UTF-8 error string.
    let message = unsafe { CStr::from_ptr(error) }
        .to_string_lossy()
        .to_string();
    // SAFETY: the error string was allocated by the Objective-C bridge.
    unsafe {
        opennote_free_error(error);
    }
    message
}

#[cfg(target_os = "macos")]
fn start_computer_audio_capture(
    app: AppHandle,
    wav_path: PathBuf,
    sample_tx: Option<std::sync::mpsc::Sender<Vec<f32>>>,
) -> Result<AudioCapture, String> {
    let writer = Arc::new(Mutex::new(Some(create_wav_writer(&wav_path)?)));
    let callback_context = Box::into_raw(Box::new(SystemAudioCallbackContext {
        app,
        writer: Arc::clone(&writer),
        sample_tx,
    }));
    let mut error = std::ptr::null_mut();
    // SAFETY: callback_context remains boxed until SystemAudioCapture stops or start fails.
    let handle = unsafe {
        opennote_start_system_audio_capture(
            receive_system_audio,
            callback_context.cast(),
            &mut error,
        )
    };
    if handle.is_null() {
        // SAFETY: native capture did not retain the context when start failed.
        unsafe {
            drop(Box::from_raw(callback_context));
        }
        return Err(take_bridge_error(error));
    }

    Ok(AudioCapture {
        _stream: None,
        system_audio: Some(SystemAudioCapture {
            callback_context,
            handle: Mutex::new(Some(handle)),
        }),
        writer: Some(writer),
    })
}

#[cfg(not(target_os = "macos"))]
fn start_computer_audio_capture(
    _app: AppHandle,
    _wav_path: PathBuf,
    _sample_tx: Option<std::sync::mpsc::Sender<Vec<f32>>>,
) -> Result<AudioCapture, String> {
    Err("Computer audio recording is only available on macOS 13 or newer.".to_string())
}

fn create_wav_writer(
    wav_path: &PathBuf,
) -> Result<WavWriter<std::io::BufWriter<std::fs::File>>, String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    WavWriter::create(wav_path, spec).map_err(|e| format!("Failed to create WAV file: {e}"))
}

fn emit_audio_level(app: &AppHandle, samples: &[f32]) {
    if !samples.is_empty() {
        let rms = compute_rms(samples);
        let level = (rms * 5.0).min(1.0);
        let _ = app.emit("audio-level", AudioLevelEvent { level });
    }
}

fn write_samples(writer: &WavWriterHandle, samples: &[f32]) {
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(ref mut writer) = *guard {
            for &sample in samples {
                let value = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                let _ = writer.write_sample(value);
            }
        }
    }
}

fn downmix(data: &[f32], channels: u16) -> Vec<f32> {
    if channels > 1 {
        data.chunks(channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        data.to_vec()
    }
}

fn resample_if_needed(samples: &[f32], ratio: f64) -> Vec<f32> {
    if (ratio - 1.0).abs() <= 0.01 {
        return samples.to_vec();
    }
    linear_resample(samples, ratio)
}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

fn linear_resample(samples: &[f32], ratio: f64) -> Vec<f32> {
    if ratio <= 0.0 || samples.is_empty() {
        return samples.to_vec();
    }
    let output_len = ((samples.len() as f64) / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx0 = src_idx.floor() as usize;
        let frac = src_idx - idx0 as f64;
        let s0 = samples.get(idx0).copied().unwrap_or(0.0);
        let s1 = samples.get(idx0 + 1).copied().unwrap_or(s0);
        output.push(s0 + (s1 - s0) * frac as f32);
    }
    output
}

pub fn finalize_wav(writer: &WavWriterHandle) -> Result<(), String> {
    let mut guard = writer
        .lock()
        .map_err(|e| format!("WAV writer lock error: {e}"))?;
    if let Some(w) = guard.take() {
        w.finalize()
            .map_err(|e| format!("WAV finalize error: {e}"))?;
    }
    Ok(())
}
