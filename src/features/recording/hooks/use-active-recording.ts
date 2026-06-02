import type { UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  deleteAudioFile,
  getDownloadedModels,
  getSystemAudioPermission,
  listenToAudioLevels,
  loadTranscriptionModel,
  openSystemAudioSettings,
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
import { INITIAL_RECORDING_STATE, type RecordingDialogState } from "../types";

export interface StopRecordingResult {
  audioPath: string | null;
  duration: number;
  modelId: string;
  segments: TranscriptionResult["segments"];
  startedAt: string | null;
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

      const unlisten = await listenToAudioLevels((e: AudioLevelEvent) =>
        setAudioLevel(e.level)
      );
      unlistenRefs.current = [unlisten];
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
      cleanup();
      setState((s) => ({ ...s, phase: "error", error: String(err) }));
    }
  }, [cleanup]);

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
              segments: transcriptionResult.segments,
            };
          }

          return {
            ...data,
            audioPath,
            segments: transcriptionResult.segments,
          };
        } catch (err) {
          console.error("Transcription failed:", err);
          if (!saveAudioRef.current) {
            try {
              await deleteAudioFile(data.audioPath);
            } catch {
              // deletion best-effort
            }
          }
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

  const reportError = useCallback((error: unknown) => {
    setState((s) => ({ ...s, phase: "error", error: String(error) }));
  }, []);

  useEffect(() => {
    return cleanup;
  }, [cleanup]);

  return {
    audioLevel,
    reset,
    openComputerAudioSettings,
    reportError,
    selectAudioSource,
    startRecording,
    state,
    stopRecording,
    systemAudioPermission,
    toggleSaveAudio,
  };
}
