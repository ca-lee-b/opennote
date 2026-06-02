export type AudioSource = "microphone" | "computer_audio";

export interface LoadTranscriptionModelRequest {
  id: string;
}

export interface TranscriptionStateSnapshot {
  isModelLoaded: boolean;
  isRecording: boolean;
  loadedModelId: string | null;
}

export interface RecordingData {
  audioPath: string | null;
  duration: number;
  modelId: string;
  startedAt: string | null;
}

export interface TranscriptionResult {
  modelId: string;
  segments: TranscriptionSegment[];
  text: string;
}

export interface ModelDownloadInfo {
  blurb: string;
  displayName: string;
  downloadProgress: number;
  id: string;
  isDownloaded: boolean;
  isDownloading: boolean;
  parameterCount: string;
  sizeBytes: number;
  wer: string;
}

export interface TranscriptionSegment {
  endTimeSecs: number;
  startTimeSecs: number;
  text: string;
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
