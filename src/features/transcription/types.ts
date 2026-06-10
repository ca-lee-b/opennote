export type AudioSource = "microphone" | "computer_audio";

export interface LoadTranscriptionModelRequest {
  id: string;
}

export interface StartTranscriptionRecordingRequest {
  audioSource: AudioSource;
  livePreviewEnabled: boolean;
  saveAudio: boolean;
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

export interface EnqueueRecordingTranscriptionRequest {
  audioPath: string;
  duration: number;
  modelId: string;
  saveAudio: boolean;
  startedAt: string | null;
  title: string;
}

export interface EnqueueRecordingTranscriptionResult {
  jobId: string;
  recordingId: string;
}

export interface ImportAudioTranscriptionRequest {
  modelId: string;
  sourceAudioPath: string;
  title: string;
}

export type RecordingProcessingState =
  | "queued"
  | "chunking"
  | "transcribing"
  | "complete"
  | "partial"
  | "failed"
  | "interrupted"
  | "cancelled";

export interface RecordingProcessingStatus {
  completedChunks: number;
  error: string | null;
  failedChunks: number;
  jobId: string;
  recordingId: string;
  status: RecordingProcessingState;
  totalChunks: number;
  updatedAt: string;
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

export interface TranscriptionPreviewEvent extends TranscriptionSegment {
  isFinal: boolean;
  sequence: number;
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
