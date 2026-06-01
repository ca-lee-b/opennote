export type ModelArch = "small" | "medium" | "parakeet_tdt";
export type AudioSource = "microphone" | "computer_audio";

export interface LoadTranscriptionModelRequest {
  arch: ModelArch;
  id: string;
  path: string;
}

export interface TranscriptionStateSnapshot {
  isModelLoaded: boolean;
  isRecording: boolean;
  loadedModelArch: ModelArch | null;
  loadedModelId: string | null;
  loadedModelPath: string | null;
}

export interface RecordingData {
  audioPath: string | null;
  duration: number;
  modelId: string;
  startedAt: string | null;
}

export interface TranscriptionResult {
  modelId: string;
  text: string;
}

export interface ModelDownloadInfo {
  arch: ModelArch;
  blurb: string;
  displayName: string;
  downloadProgress: number;
  id: string;
  isDownloaded: boolean;
  isDownloading: boolean;
  parameterCount: string;
  path: string;
  sizeBytes: number;
  wer: string;
}

export type DownloadStatus =
  | "downloading"
  | "completed"
  | "failed"
  | "cancelled";

export interface ModelDownloadProgressEvent {
  modelId: string;
  progress: number;
  status: DownloadStatus;
}

export interface AudioLevelEvent {
  level: number;
}
