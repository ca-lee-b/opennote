import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  getDownloadedModels,
  listenToAudioLevels,
  listenToTranscriptionPreview,
  loadTranscriptionModel,
  startTranscriptionRecording,
  stopTranscriptionRecording,
} from "@/features/transcription/api/transcription-service";
import type {
  AudioLevelEvent,
  AudioSource,
  TranscriptionPreviewEvent,
} from "@/features/transcription/types";
import { getAppPreferences } from "@/lib/app-preferences";

interface CaptureSessionOptions {
  audioSource: AudioSource;
  livePreviewEnabled: boolean;
  onAudioLevel: (level: number) => void;
  onDurationChange: (duration: number) => void;
  onTranscriptionPreview: (event: TranscriptionPreviewEvent) => void;
  saveAudio: boolean;
}

interface StartOptions {
  shouldCancel?: () => boolean;
}

export interface StopRecordingResult {
  audioPath: string;
  duration: number;
  modelId: string;
  saveAudio: boolean;
  startedAt: string | null;
}

export class CaptureSession {
  private audioSource: AudioSource;
  private durationTimer: ReturnType<typeof setInterval> | null = null;
  private readonly onAudioLevel: (level: number) => void;
  private readonly onDurationChange: (duration: number) => void;
  private readonly onTranscriptionPreview: (
    event: TranscriptionPreviewEvent
  ) => void;
  private livePreviewEnabled: boolean;
  private saveAudio: boolean;
  private unlistenAudioLevel: UnlistenFn | null = null;
  private unlistenTranscriptionPreview: UnlistenFn | null = null;

  constructor(options: CaptureSessionOptions) {
    this.audioSource = options.audioSource;
    this.livePreviewEnabled = options.livePreviewEnabled;
    this.onAudioLevel = options.onAudioLevel;
    this.onDurationChange = options.onDurationChange;
    this.onTranscriptionPreview = options.onTranscriptionPreview;
    this.saveAudio = options.saveAudio;
  }

  setAudioSource(audioSource: AudioSource): void {
    this.audioSource = audioSource;
  }

  setLivePreviewEnabled(livePreviewEnabled: boolean): void {
    this.livePreviewEnabled = livePreviewEnabled;
  }

  setSaveAudio(saveAudio: boolean): void {
    this.saveAudio = saveAudio;
  }

  async start(options: StartOptions = {}): Promise<void> {
    let recordingStarted = false;
    try {
      const throwIfCancelled = () => {
        if (options.shouldCancel?.()) {
          throw new Error("Recording start was cancelled.");
        }
      };

      const selectedModelId = getAppPreferences().selectedModelId;
      if (!selectedModelId) {
        throw new Error(
          "Select and download a transcription model in Settings before recording."
        );
      }

      const models = await getDownloadedModels();
      throwIfCancelled();

      const model = models.find(
        (candidate) => candidate.id === selectedModelId
      );
      if (!model?.isDownloaded) {
        throw new Error(
          "The selected transcription model is not downloaded. Choose a downloaded model in Settings."
        );
      }

      await loadTranscriptionModel({ id: model.id });
      throwIfCancelled();

      this.unlistenAudioLevel = await listenToAudioLevels(
        (event: AudioLevelEvent) => {
          this.onAudioLevel(event.level);
        }
      );
      if (this.livePreviewEnabled) {
        this.unlistenTranscriptionPreview = await listenToTranscriptionPreview(
          this.onTranscriptionPreview
        );
      }
      throwIfCancelled();

      await startTranscriptionRecording({
        audioSource: this.audioSource,
        livePreviewEnabled: this.livePreviewEnabled,
        saveAudio: this.saveAudio,
      });
      recordingStarted = true;
      throwIfCancelled();

      this.startDurationTimer();
    } catch (error) {
      this.cleanup();
      if (recordingStarted) {
        await stopTranscriptionRecording().catch(() => {
          // recording may have already stopped, ignore errors
        });
      }
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

    return {
      ...recordingData,
      audioPath: recordingData.audioPath,
      saveAudio: this.saveAudio,
    };
  }

  cleanup(): void {
    this.unlistenAudioLevel?.();
    this.unlistenAudioLevel = null;
    this.unlistenTranscriptionPreview?.();
    this.unlistenTranscriptionPreview = null;

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

  private startDurationTimer(): void {
    const startTime = Date.now();
    this.durationTimer = setInterval(() => {
      this.onDurationChange((Date.now() - startTime) / 1000);
    }, 100);
  }
}
