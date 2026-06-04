import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  deleteAudioFile,
  getDownloadedModels,
  listenToAudioLevels,
  loadTranscriptionModel,
  startTranscriptionRecording,
  stopTranscriptionRecording,
  transcribeRecording,
} from "@/features/transcription/api/transcription-service";
import type {
  AudioLevelEvent,
  AudioSource,
  TranscriptionResult,
} from "@/features/transcription/types";
import { getAppPreferences } from "@/lib/app-preferences";

interface CaptureSessionOptions {
  audioSource: AudioSource;
  onAudioLevel: (level: number) => void;
  onDurationChange: (duration: number) => void;
  saveAudio: boolean;
}

export interface StopRecordingResult {
  audioPath: string | null;
  duration: number;
  modelId: string;
  segments: TranscriptionResult["segments"];
  startedAt: string | null;
}

export class CaptureSession {
  private audioSource: AudioSource;
  private durationTimer: ReturnType<typeof setInterval> | null = null;
  private readonly onAudioLevel: (level: number) => void;
  private readonly onDurationChange: (duration: number) => void;
  private saveAudio: boolean;
  private unlistenAudioLevel: UnlistenFn | null = null;

  constructor(options: CaptureSessionOptions) {
    this.audioSource = options.audioSource;
    this.onAudioLevel = options.onAudioLevel;
    this.onDurationChange = options.onDurationChange;
    this.saveAudio = options.saveAudio;
  }

  setAudioSource(audioSource: AudioSource): void {
    this.audioSource = audioSource;
  }

  setSaveAudio(saveAudio: boolean): void {
    this.saveAudio = saveAudio;
  }

  async start(): Promise<void> {
    try {
      const selectedModelId = getAppPreferences().selectedModelId;
      if (!selectedModelId) {
        throw new Error(
          "Select and download a transcription model in Settings before recording."
        );
      }

      const models = await getDownloadedModels();
      const model = models.find(
        (candidate) => candidate.id === selectedModelId
      );
      if (!model?.isDownloaded) {
        throw new Error(
          "The selected transcription model is not downloaded. Choose a downloaded model in Settings."
        );
      }

      await loadTranscriptionModel({ id: model.id });
      this.unlistenAudioLevel = await listenToAudioLevels(
        (event: AudioLevelEvent) => {
          this.onAudioLevel(event.level);
        }
      );
      await startTranscriptionRecording(this.audioSource, this.saveAudio);
      this.startDurationTimer();
    } catch (error) {
      this.cleanup();
      throw error;
    }
  }

  async stop(): Promise<StopRecordingResult> {
    this.cleanup();
    const recordingData = await stopTranscriptionRecording();
    console.log("[stopRecording] RecordingData:", recordingData);

    if (!recordingData.audioPath) {
      throw new Error("No audio file was saved. Please try again.");
    }

    try {
      const transcriptionResult = await transcribeRecording(
        recordingData.audioPath
      );
      const transcriptionText = transcriptionResult.text.trim();
      console.log(
        "[stopRecording] Transcription result:",
        transcriptionText.length,
        "chars"
      );

      if (!this.saveAudio) {
        await this.deleteAudioBestEffort(recordingData.audioPath);
        return {
          ...recordingData,
          audioPath: null,
          segments: transcriptionResult.segments,
        };
      }

      return {
        ...recordingData,
        audioPath: recordingData.audioPath,
        segments: transcriptionResult.segments,
      };
    } catch (error) {
      console.error("Transcription failed:", error);
      if (!this.saveAudio) {
        await this.deleteAudioBestEffort(recordingData.audioPath);
      }
      throw error;
    }
  }

  cleanup(): void {
    this.unlistenAudioLevel?.();
    this.unlistenAudioLevel = null;

    if (this.durationTimer) {
      clearInterval(this.durationTimer);
      this.durationTimer = null;
    }
  }

  async resetRecording(): Promise<void> {
    this.cleanup();
    await stopTranscriptionRecording().catch(() => {
      // recording may not be active, ignore errors
    });
  }

  private async deleteAudioBestEffort(audioPath: string): Promise<void> {
    try {
      await deleteAudioFile(audioPath);
    } catch {
      // deletion best-effort
    }
  }

  private startDurationTimer(): void {
    const startTime = Date.now();
    this.durationTimer = setInterval(() => {
      this.onDurationChange((Date.now() - startTime) / 1000);
    }, 100);
  }
}
