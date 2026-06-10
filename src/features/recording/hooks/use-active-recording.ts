import { useCallback, useEffect, useRef, useState } from "react";
import {
  getSystemAudioPermission,
  openSystemAudioSettings,
} from "@/features/transcription/api/transcription-service";
import type { AudioSource } from "@/features/transcription/types";
import { getAppPreferences } from "@/lib/app-preferences";
import type { StopRecordingResult } from "../api/capture-session";
import { CaptureSession } from "../api/capture-session";
import { INITIAL_RECORDING_STATE, type RecordingDialogState } from "../types";

export type { StopRecordingResult } from "../api/capture-session";

function getInitialRecordingState(): RecordingDialogState {
  return {
    ...INITIAL_RECORDING_STATE,
    livePreviewEnabled: getAppPreferences().liveTranscriptionPreviewEnabled,
  };
}

export function useActiveRecording() {
  const [state, setState] = useState<RecordingDialogState>(
    getInitialRecordingState
  );
  const [audioLevel, setAudioLevel] = useState(0);
  const [systemAudioPermission, setSystemAudioPermission] = useState<
    boolean | null
  >(null);
  const sessionRef = useRef<CaptureSession | null>(null);
  const startRequestIdRef = useRef(0);

  if (!sessionRef.current) {
    const preferences = getAppPreferences();
    sessionRef.current = new CaptureSession({
      audioSource: INITIAL_RECORDING_STATE.audioSource,
      livePreviewEnabled: preferences.liveTranscriptionPreviewEnabled,
      saveAudio: INITIAL_RECORDING_STATE.saveAudio,
      onAudioLevel: setAudioLevel,
      onDurationChange: (duration) => {
        setState((s) => ({ ...s, duration }));
      },
      onTranscriptionPreview: (preview) => {
        setState((s) => ({
          ...s,
          previewSegments: [...s.previewSegments, preview],
        }));
      },
    });
  }

  const getSession = useCallback(() => {
    if (!sessionRef.current) {
      throw new Error("Capture session is unavailable");
    }
    return sessionRef.current;
  }, []);

  const cleanup = useCallback(() => {
    getSession().cleanup();
  }, [getSession]);

  const startRecording = useCallback(async () => {
    const requestId = startRequestIdRef.current + 1;
    startRequestIdRef.current = requestId;
    const livePreviewEnabled =
      getAppPreferences().liveTranscriptionPreviewEnabled;
    getSession().setLivePreviewEnabled(livePreviewEnabled);
    setState((s) => ({
      ...s,
      livePreviewEnabled,
      phase: "loading-model",
      previewSegments: [],
    }));
    try {
      await getSession().start({
        shouldCancel: () => startRequestIdRef.current !== requestId,
      });
      if (startRequestIdRef.current !== requestId) {
        return;
      }
      setState((s) => ({ ...s, phase: "recording" }));
    } catch (err) {
      cleanup();
      if (startRequestIdRef.current !== requestId) {
        return;
      }
      setState((s) => ({ ...s, phase: "error", error: String(err) }));
    }
  }, [cleanup, getSession]);

  const cancelLoading = useCallback(() => {
    startRequestIdRef.current += 1;
    getSession().cleanup();
    setState(getInitialRecordingState());
    setAudioLevel(0);
    setSystemAudioPermission(null);
  }, [getSession]);

  const stopRecording =
    useCallback(async (): Promise<StopRecordingResult | null> => {
      setState((s) => ({ ...s, phase: "transcribing" }));

      try {
        return await getSession().stop();
      } catch (err) {
        console.error("Failed to stop recording:", err);
        setState((s) => ({ ...s, phase: "error", error: String(err) }));
        return null;
      }
    }, [getSession]);

  const reset = useCallback(async () => {
    startRequestIdRef.current += 1;
    await getSession().resetRecording();
    getSession().setAudioSource(INITIAL_RECORDING_STATE.audioSource);
    getSession().setLivePreviewEnabled(
      getAppPreferences().liveTranscriptionPreviewEnabled
    );
    getSession().setSaveAudio(INITIAL_RECORDING_STATE.saveAudio);
    setState(getInitialRecordingState());
    setAudioLevel(0);
    setSystemAudioPermission(null);
  }, [getSession]);

  const toggleSaveAudio = useCallback(() => {
    setState((s) => {
      const newValue = !s.saveAudio;
      getSession().setSaveAudio(newValue);
      return { ...s, saveAudio: newValue };
    });
  }, [getSession]);

  const selectAudioSource = useCallback(
    (audioSource: AudioSource) => {
      getSession().setAudioSource(audioSource);
      setState((s) => ({ ...s, audioSource }));
      if (audioSource === "computer_audio") {
        getSystemAudioPermission()
          .then(setSystemAudioPermission)
          .catch(() => setSystemAudioPermission(false));
      }
    },
    [getSession]
  );

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
    cancelLoading,
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
