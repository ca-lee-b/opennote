import type {
  AudioSource,
  TranscriptionPreviewEvent,
} from "@/features/transcription/types";

export type RecordingDialogPhase =
  | "idle"
  | "loading-model"
  | "recording"
  | "transcribing"
  | "error";

export interface RecordingDialogState {
  audioSource: AudioSource;
  duration: number;
  error: string | null;
  livePreviewEnabled: boolean;
  phase: RecordingDialogPhase;
  previewSegments: TranscriptionPreviewEvent[];
  saveAudio: boolean;
}

export const INITIAL_RECORDING_STATE: RecordingDialogState = {
  audioSource: "microphone",
  phase: "idle",
  duration: 0,
  livePreviewEnabled: true,
  previewSegments: [],
  saveAudio: true,
  error: null,
};
