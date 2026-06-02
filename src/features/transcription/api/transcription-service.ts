import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AudioLevelEvent,
  AudioSource,
  LoadTranscriptionModelRequest,
  ModelDownloadInfo,
  ModelDownloadProgressEvent,
  RecordingData,
  TranscriptionResult,
  TranscriptionStateSnapshot,
} from "@/features/transcription/types";

export function getTranscriptionState(): Promise<TranscriptionStateSnapshot> {
  return invoke("get_transcription_state");
}

export function loadTranscriptionModel(
  request: LoadTranscriptionModelRequest
): Promise<TranscriptionStateSnapshot> {
  return invoke("load_transcription_model", { request });
}

export function startTranscriptionRecording(
  audioSource: AudioSource,
  saveAudio: boolean
): Promise<TranscriptionStateSnapshot> {
  return invoke("start_transcription_recording", {
    request: { audioSource, saveAudio },
  });
}

export function getSystemAudioPermission(): Promise<boolean> {
  return invoke("get_system_audio_permission");
}

export function openSystemAudioSettings(): Promise<void> {
  return invoke("open_system_audio_settings");
}

export function stopTranscriptionRecording(): Promise<RecordingData> {
  return invoke("stop_transcription_recording");
}

export function transcribeRecording(
  wavPath: string
): Promise<TranscriptionResult> {
  return invoke("transcribe_recording", { wavPath });
}

export function deleteAudioFile(path: string): Promise<void> {
  return invoke("delete_audio_file", { path });
}

export function clearAllAudioFiles(): Promise<number> {
  return invoke("clear_all_audio_files");
}

export function listenToAudioLevels(
  handler: (event: AudioLevelEvent) => void
): Promise<UnlistenFn> {
  return listen<AudioLevelEvent>("audio-level", (event) => {
    handler(event.payload);
  });
}

export function getDownloadedModels(): Promise<ModelDownloadInfo[]> {
  return invoke("get_downloaded_models");
}

export function downloadModel(modelId: string): Promise<void> {
  return invoke("download_model", { request: { modelId } });
}

export function deleteModel(modelId: string): Promise<void> {
  return invoke("delete_model", { request: { modelId } });
}

export function cancelDownload(modelId: string): Promise<void> {
  return invoke("cancel_download", { modelId });
}

export function listenToDownloadProgress(
  handler: (event: ModelDownloadProgressEvent) => void
): Promise<UnlistenFn> {
  return listen<ModelDownloadProgressEvent>(
    "model-download-progress",
    (event) => {
      handler(event.payload);
    }
  );
}
