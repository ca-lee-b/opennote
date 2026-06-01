import { Mic01Icon, StopCircleIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useCallback, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Separator } from "@/components/ui/separator";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import type { StopRecordingResult } from "@/features/recording/hooks/use-active-recording";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import { formatDuration } from "@/types/recording";
import { useActiveRecording } from "../hooks/use-active-recording";
import { WaveformView } from "./waveform-view";

interface ActiveRecordingDialogProps {
  onOpenChange: (open: boolean) => void;
  open: boolean;
}

function getDescriptionText(phase: string): string {
  if (phase === "recording") {
    return "Speak naturally. Your words will be transcribed after you stop.";
  }
  if (phase === "loading-model") {
    return "Loading transcription model...";
  }
  if (phase === "transcribing") {
    return "Transcribing your recording...";
  }
  return "Start a new voice recording with live transcription.";
}

function getTitleText(phase: string): string {
  if (phase === "recording") {
    return "Recording";
  }
  if (phase === "loading-model") {
    return "Preparing...";
  }
  if (phase === "transcribing") {
    return "Transcribing...";
  }
  return "New Recording";
}

export function ActiveRecordingDialog({
  onOpenChange,
  open,
}: ActiveRecordingDialogProps) {
  const {
    audioLevel,
    openComputerAudioSettings,
    reset,
    selectAudioSource,
    startRecording,
    state,
    stopRecording,
    systemAudioPermission,
    toggleSaveAudio,
  } = useActiveRecording();
  const createRecording = useRecordingsStore((s) => s.createRecording);
  const insertLine = useRecordingsStore((s) => s.insertLine);
  const [isSaving, setIsSaving] = useState(false);

  const handleOpenChange = useCallback(
    (nextOpen: boolean) => {
      if (
        !nextOpen &&
        (state.phase === "recording" || state.phase === "transcribing")
      ) {
        return;
      }
      if (!nextOpen) {
        reset();
      }
      onOpenChange(nextOpen);
    },
    [state.phase, reset, onOpenChange]
  );

  const handleStart = useCallback(async () => {
    await startRecording();
  }, [startRecording]);

  const handleStop = useCallback(async () => {
    setIsSaving(true);
    const result: StopRecordingResult | null = await stopRecording();
    console.log("[handleStop] StopRecordingResult:", result);

    if (!result) {
      setIsSaving(false);
      return;
    }

    try {
      const recording = await createRecording({
        audioPath: result.audioPath,
        createdAt: result.startedAt ?? undefined,
        duration: result.duration,
        fullText: result.transcriptionText ?? "",
        isPartial: false,
        modelId: result.modelId,
      });
      console.log("[handleStop] Created recording:", recording.id);

      if (result.transcriptionText?.trim()) {
        await insertLine({
          duration: result.duration,
          isFinal: true,
          lineId: 1,
          recordingId: recording.id,
          sortOrder: 0,
          startTime: "0:00",
          text: result.transcriptionText,
        });
      } else {
        console.log("[handleStop] No transcription text to insert");
      }
    } catch (err) {
      console.error("Failed to save recording:", err);
    }

    setIsSaving(false);
    reset();
    onOpenChange(false);
  }, [stopRecording, createRecording, insertLine, reset, onOpenChange]);

  const isRecording = state.phase === "recording";
  const isLoading = state.phase === "loading-model";
  const isTranscribing = state.phase === "transcribing";

  return (
    <Dialog onOpenChange={handleOpenChange} open={open}>
      <DialogContent
        className="sm:max-w-xl"
        showCloseButton={isRecording || isTranscribing ? false : undefined}
      >
        <DialogHeader>
          <DialogTitle>{getTitleText(state.phase)}</DialogTitle>
          <DialogDescription>
            {getDescriptionText(state.phase)}
          </DialogDescription>
        </DialogHeader>

        {isRecording && (
          <>
            <div className="flex items-center justify-center gap-3">
              <span className="font-semibold text-2xl text-foreground tabular-nums tracking-tight">
                {formatDuration(state.duration)}
              </span>
              <span className="relative flex h-3 w-3">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-400/70 opacity-75" />
                <span className="relative inline-flex h-3 w-3 rounded-full bg-red-500" />
              </span>
            </div>

            <WaveformView audioLevel={audioLevel} isActive={isRecording} />

            <Separator />

            <div className="flex items-center justify-center py-4 text-muted-foreground text-sm">
              Listening...
            </div>
          </>
        )}

        {isTranscribing && (
          <div className="flex flex-col items-center gap-3 py-6">
            <Spinner className="size-6" />
            <p className="text-muted-foreground text-sm">
              Transcribing your recording...
            </p>
            <span className="font-medium text-muted-foreground text-sm tabular-nums">
              {formatDuration(state.duration)} recorded
            </span>
          </div>
        )}

        {isLoading && (
          <div className="flex flex-col items-center gap-3 py-6">
            <Spinner className="size-6" />
            <p className="text-muted-foreground text-sm">Loading model...</p>
          </div>
        )}

        {state.phase === "idle" && (
          <div className="space-y-4">
            <div className="rounded-xl border border-border bg-muted/30 px-4 py-3">
              <p className="mb-3 font-medium text-sm">Audio source</p>
              <RadioGroup
                onValueChange={selectAudioSource}
                value={state.audioSource}
              >
                <Label htmlFor="audio-source-microphone">
                  <RadioGroupItem
                    id="audio-source-microphone"
                    value="microphone"
                  />
                  Microphone
                </Label>
                <Label htmlFor="audio-source-computer">
                  <RadioGroupItem
                    id="audio-source-computer"
                    value="computer_audio"
                  />
                  Computer audio
                </Label>
              </RadioGroup>
              {state.audioSource === "computer_audio" &&
                systemAudioPermission === false && (
                  <p className="mt-3 text-muted-foreground text-xs">
                    Enable Screen Recording permission to capture computer
                    audio.{" "}
                    <button
                      className="font-medium text-foreground underline underline-offset-2"
                      onClick={openComputerAudioSettings}
                      type="button"
                    >
                      Open System Settings
                    </button>
                  </p>
                )}
            </div>
            <div className="flex items-center justify-between rounded-xl border border-border bg-muted/30 px-4 py-3">
              <div className="flex items-center gap-2">
                <Switch
                  checked={state.saveAudio}
                  onCheckedChange={toggleSaveAudio}
                  size="sm"
                />
                <span className="text-sm">Save audio file</span>
              </div>
            </div>
          </div>
        )}

        {state.phase === "error" && (
          <p className="text-destructive text-sm">{state.error}</p>
        )}

        <div className="flex justify-center gap-3">
          {state.phase === "idle" && (
            <Button className="gap-2" onClick={handleStart} size="lg">
              <HugeiconsIcon icon={Mic01Icon} size={18} strokeWidth={2} />
              Start Recording
            </Button>
          )}

          {isRecording && (
            <Button
              className="gap-2"
              onClick={handleStop}
              size="lg"
              variant="destructive"
            >
              <HugeiconsIcon icon={StopCircleIcon} size={18} strokeWidth={2} />
              Stop Recording
            </Button>
          )}

          {(isSaving || isTranscribing) && (
            <Button className="gap-2" disabled size="lg">
              <Spinner className="size-4" />
              {isTranscribing ? "Transcribing..." : "Saving..."}
            </Button>
          )}

          {state.phase === "error" && (
            <Button className="gap-2" onClick={reset} size="lg">
              Dismiss
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
