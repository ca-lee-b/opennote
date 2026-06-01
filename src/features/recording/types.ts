import type { AudioSource } from "@/features/transcription/types";

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
  phase: RecordingDialogPhase;
  saveAudio: boolean;
}

export const INITIAL_RECORDING_STATE: RecordingDialogState = {
  audioSource: "microphone",
  phase: "idle",
  duration: 0,
  saveAudio: true,
  error: null,
};
