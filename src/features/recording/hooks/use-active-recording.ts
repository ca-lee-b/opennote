import type { UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  deleteAudioFile,
  getDownloadedModels,
  getSystemAudioPermission,
  listenToAudioLevels,
  listenToPartialTranscription,
  loadTranscriptionModel,
  openSystemAudioSettings,
  startTranscriptionRecording,
  stopTranscriptionRecording,
  transcribeRecording,
} from "@/features/transcription/api/transcription-service";
import type {
  AudioLevelEvent,
  AudioSource,
} from "@/features/transcription/types";
import { INITIAL_RECORDING_STATE, type RecordingDialogState } from "../types";

export interface StopRecordingResult {
  audioPath: string | null;
  duration: number;
  modelId: string;
  startedAt: string | null;
  transcriptionText: string | null;
}

export function useActiveRecording() {
  const [state, setState] = useState<RecordingDialogState>(
    INITIAL_RECORDING_STATE
  );
  const [audioLevel, setAudioLevel] = useState(0);
  const [systemAudioPermission, setSystemAudioPermission] = useState<
    boolean | null
  >(null);
  const unlistenRefs = useRef<UnlistenFn[]>([]);
  const durationRef = useRef<ReturnType<typeof setInterval> | undefined>(
    undefined
  );

  const saveAudioRef = useRef(true);
  const audioSourceRef = useRef<AudioSource>("microphone");

  const cleanup = useCallback(() => {
    for (const fn of unlistenRefs.current) {
      fn();
    }
    unlistenRefs.current = [];
    if (durationRef.current !== undefined) {
      clearInterval(durationRef.current);
      durationRef.current = undefined;
    }
  }, []);

  const startRecording = useCallback(async () => {
    setState((s) => ({ ...s, phase: "loading-model" }));
    try {
      const unlisten = await listenToAudioLevels((e: AudioLevelEvent) =>
        setAudioLevel(e.level)
      );
      const unlistenPartial = await listenToPartialTranscription((event) => {
        console.log(
          `[streaming] ${event.isFinal ? "FINAL" : "partial"} @${event.startTimeSecs.toFixed(1)}s: ${event.text}`
        );
      });
      unlistenRefs.current = [unlisten, unlistenPartial];

      const models = await getDownloadedModels();
      const model = models.find((m) => m.isDownloaded);
      if (!model) {
        throw new Error(
          "No downloaded model found. Please download a model first."
        );
      }

      await loadTranscriptionModel({
        arch: model.arch,
        id: model.id,
        path: model.path,
      });
      await startTranscriptionRecording(
        audioSourceRef.current,
        saveAudioRef.current
      );
      const startTime = Date.now();
      durationRef.current = setInterval(() => {
        setState((s) => ({
          ...s,
          duration: (Date.now() - startTime) / 1000,
        }));
      }, 100);
      setState((s) => ({ ...s, phase: "recording" }));
    } catch (err) {
      setState((s) => ({ ...s, phase: "error", error: String(err) }));
    }
  }, []);

  const stopRecording =
    useCallback(async (): Promise<StopRecordingResult | null> => {
      cleanup();
      setState((s) => ({ ...s, phase: "transcribing" }));

      try {
        const data = await stopTranscriptionRecording();
        console.log("[stopRecording] RecordingData:", data);

        if (!data.audioPath) {
          setState((s) => ({
            ...s,
            phase: "error",
            error: "No audio file was saved. Please try again.",
          }));
          return null;
        }

        try {
          const transcriptionResult = await transcribeRecording(data.audioPath);
          const transcriptionText = transcriptionResult.text.trim();
          console.log(
            "[stopRecording] Transcription result:",
            transcriptionText.length,
            "chars"
          );

          const audioPath = data.audioPath;
          if (!saveAudioRef.current) {
            try {
              await deleteAudioFile(audioPath);
            } catch {
              // deletion best-effort
            }
            return {
              ...data,
              audioPath: null,
              transcriptionText,
            };
          }

          return {
            ...data,
            audioPath,
            transcriptionText,
          };
        } catch (err) {
          console.error("Transcription failed:", err);
          setState((s) => ({
            ...s,
            phase: "error",
            error: String(err),
          }));
          return null;
        }
      } catch (err) {
        console.error("Failed to stop recording:", err);
        setState((s) => ({ ...s, phase: "error", error: String(err) }));
        return null;
      }
    }, [cleanup]);

  const reset = useCallback(() => {
    cleanup();
    stopTranscriptionRecording().catch(() => {
      // recording may not be active, ignore errors
    });
    audioSourceRef.current = INITIAL_RECORDING_STATE.audioSource;
    saveAudioRef.current = INITIAL_RECORDING_STATE.saveAudio;
    setState(INITIAL_RECORDING_STATE);
    setAudioLevel(0);
    setSystemAudioPermission(null);
  }, [cleanup]);

  const toggleSaveAudio = useCallback(() => {
    setState((s) => {
      const newValue = !s.saveAudio;
      saveAudioRef.current = newValue;
      return { ...s, saveAudio: newValue };
    });
  }, []);

  const selectAudioSource = useCallback((audioSource: AudioSource) => {
    audioSourceRef.current = audioSource;
    setState((s) => ({ ...s, audioSource }));
    if (audioSource === "computer_audio") {
      getSystemAudioPermission()
        .then(setSystemAudioPermission)
        .catch(() => setSystemAudioPermission(false));
    }
  }, []);

  const openComputerAudioSettings = useCallback(async () => {
    await openSystemAudioSettings();
  }, []);

  useEffect(() => {
    return cleanup;
  }, [cleanup]);

  return {
    audioLevel,
    reset,
    openComputerAudioSettings,
    selectAudioSource,
    startRecording,
    state,
    stopRecording,
    systemAudioPermission,
    toggleSaveAudio,
  };
}
