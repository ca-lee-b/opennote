import {
  Cancel01Icon,
  Chip02Icon,
  Delete02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  cancelDownload,
  clearAllAudioFiles,
  deleteModel,
  downloadModel,
  getDownloadedModels,
  listenToDownloadProgress,
} from "@/features/transcription/api/transcription-service";
import type {
  ModelArch,
  ModelDownloadInfo,
} from "@/features/transcription/types";
import { getAppPreferences, setSelectedModelId } from "@/lib/app-preferences";
import { formatBytes } from "@/lib/utils";
import { DownloadProgress } from "./download-progress";
import { ModelRightAction } from "./model-actions";
import { ModelStatusBadge } from "./model-status-badge";

interface ModelGroup {
  label: string;
  models: ModelDownloadInfo[];
}

function isNvidiaArch(arch: ModelArch): boolean {
  return arch === "parakeet_tdt";
}

export function SettingsView() {
  const [selectedModelId, setSelectedModelIdState] = useState(
    () => getAppPreferences().selectedModelId ?? "small_streaming"
  );
  const [models, setModels] = useState<ModelDownloadInfo[]>([]);
  const [isLoadingModels, setIsLoadingModels] = useState(true);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<
    Record<string, number>
  >({});
  const [downloadingModelId, setDownloadingModelId] = useState<string | null>(
    null
  );
  const [actionError, setActionError] = useState<string | null>(null);

  const modelGroups = useMemo<ModelGroup[]>(() => {
    const moonshine = models.filter((m) => !isNvidiaArch(m.arch));
    const nvidia = models.filter((m) => isNvidiaArch(m.arch));
    const groups: ModelGroup[] = [];
    if (moonshine.length > 0) {
      groups.push({ label: "Moonshine", models: moonshine });
    }
    if (nvidia.length > 0) {
      groups.push({ label: "NVIDIA Parakeet", models: nvidia });
    }
    return groups;
  }, [models]);

  // Fetch models on mount.
  useEffect(() => {
    let cancelled = false;

    getDownloadedModels()
      .then((result) => {
        if (cancelled) {
          return;
        }
        setModels(result);
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        setModelsError(String(err));
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoadingModels(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  // Listen for download progress events.
  useEffect(() => {
    const unlisten = listenToDownloadProgress((event) => {
      if (event.status === "downloading") {
        setDownloadProgress((prev) => ({
          ...prev,
          [event.modelId]: event.progress,
        }));
      } else if (
        event.status === "completed" ||
        event.status === "failed" ||
        event.status === "cancelled"
      ) {
        setDownloadProgress((prev) => {
          const next = { ...prev };
          delete next[event.modelId];
          return next;
        });
        setDownloadingModelId((prev) => (prev === event.modelId ? null : prev));

        if (event.status === "completed") {
          // Refresh model list after successful download.
          getDownloadedModels()
            .then(setModels)
            .catch(() => {
              /* ignored — model list refresh is non-critical */
            });
        }

        if (event.status === "failed") {
          setActionError("Failed to download model. Please try again.");
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleDownload = useCallback(async (modelId: string) => {
    setActionError(null);
    setDownloadingModelId(modelId);
    setDownloadProgress((prev) => ({ ...prev, [modelId]: 0 }));

    try {
      await downloadModel(modelId);
      // The download progress listener will handle completion.
    } catch (err) {
      setDownloadingModelId(null);
      setDownloadProgress((prev) => {
        const next = { ...prev };
        delete next[modelId];
        return next;
      });
      setActionError(String(err));
    }
  }, []);

  const handleCancelDownload = useCallback(async (modelId: string) => {
    try {
      await cancelDownload(modelId);
    } catch (err) {
      setActionError(String(err));
    }
  }, []);

  const handleDelete = useCallback(async (modelId: string) => {
    setActionError(null);
    try {
      await deleteModel(modelId);
      // Refresh model list after deletion.
      const updatedModels = await getDownloadedModels();
      setModels(updatedModels);
      // If the deleted model was selected, clear the selection.
      const prefs = getAppPreferences();
      if (prefs.selectedModelId === modelId) {
        setSelectedModelId(null);
        setSelectedModelIdState(
          getAppPreferences().selectedModelId ?? "small_streaming"
        );
      }
    } catch (err) {
      setActionError(String(err));
    }
  }, []);

  const handleSelect = useCallback((modelId: string) => {
    setSelectedModelId(modelId);
    setSelectedModelIdState(modelId);
  }, []);

  const [isClearingAudio, setIsClearingAudio] = useState(false);
  const [clearAudioError, setClearAudioError] = useState<string | null>(null);

  const handleClearAllAudio = useCallback(async () => {
    setIsClearingAudio(true);
    setClearAudioError(null);
    try {
      await clearAllAudioFiles();
    } catch (err) {
      setClearAudioError(String(err));
    } finally {
      setIsClearingAudio(false);
    }
  }, []);

  return (
    <div className="flex h-full flex-col overflow-hidden bg-background">
      {/* Header */}
      <div className="flex items-center border-border/70 border-b px-6 py-4">
        <h2 className="font-heading font-semibold text-xl tracking-tight">
          Settings
        </h2>
      </div>

      <div className="flex-1 space-y-6 overflow-y-auto p-6 sm:p-8">
        {/* Error message */}
        {actionError && (
          <div className="flex items-center justify-between rounded-xl border border-destructive/20 bg-destructive/5 px-4 py-3 text-destructive text-sm">
            <span>{actionError}</span>
            <Button
              className="size-8 shrink-0 text-destructive"
              onClick={() => setActionError(null)}
              size="icon-sm"
              variant="ghost"
            >
              <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
            </Button>
          </div>
        )}

        {/* Models Section */}
        <section className="space-y-6">
          {modelGroups.map((group) => (
            <div className="space-y-3" key={group.label}>
              <div className="flex flex-col">
                <h3 className="font-medium text-muted-foreground text-sm">
                  {group.label}
                </h3>
              </div>

              <div className="grid gap-3">
                {group.models.map((model) => {
                  const isDownloading =
                    downloadingModelId === model.id || model.isDownloading;
                  const progress =
                    downloadProgress[model.id] ?? model.downloadProgress;

                  return (
                    <div
                      className="rounded-xl border border-border bg-card p-4"
                      key={model.id}
                    >
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex items-start gap-4">
                          <div className="rounded-md border border-border bg-muted/40 p-2">
                            <HugeiconsIcon
                              className="size-5 text-muted-foreground"
                              icon={Chip02Icon}
                              strokeWidth={2}
                            />
                          </div>
                          <div className="flex min-w-0 flex-col">
                            <div className="flex flex-wrap items-center gap-2">
                              <span className="font-medium text-sm tracking-tight">
                                {model.displayName}
                              </span>
                              {selectedModelId === model.id &&
                                model.isDownloaded && (
                                  <Badge
                                    className="rounded-md text-[10px]"
                                    variant="secondary"
                                  >
                                    Active
                                  </Badge>
                                )}
                              <ModelStatusBadge
                                hasError={modelsError !== null}
                                isDownloaded={model.isDownloaded}
                                isDownloading={isDownloading}
                                isLoading={isLoadingModels}
                              />
                            </div>
                            <span className="text-muted-foreground text-xs leading-5">
                              {formatBytes(model.sizeBytes)} ·{" "}
                              {model.parameterCount} · {model.wer}
                            </span>
                            <span className="mt-0.5 text-muted-foreground/75 text-xs leading-5">
                              {model.blurb}
                            </span>
                          </div>
                        </div>

                        {/* Right side: action buttons */}
                        <ModelRightAction
                          isDownloaded={model.isDownloaded}
                          isDownloading={isDownloading}
                          isSelected={selectedModelId === model.id}
                          modelId={model.id}
                          onDelete={handleDelete}
                          onDownload={handleDownload}
                          onSelect={handleSelect}
                        />
                      </div>

                      {/* Bottom: progress bar (only visible during download) */}
                      {isDownloading && (
                        <DownloadProgress
                          modelId={model.id}
                          onCancel={handleCancelDownload}
                          progress={progress}
                        />
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </section>

        {/* Danger Zone */}
        <section className="space-y-3">
          <h3 className="font-medium text-muted-foreground text-sm">
            Danger Zone
          </h3>
          <div className="rounded-xl border border-border bg-card p-4">
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-start gap-4">
                <div className="rounded-md border border-border bg-muted/40 p-2">
                  <HugeiconsIcon
                    className="size-5 text-muted-foreground"
                    icon={Delete02Icon}
                    strokeWidth={2}
                  />
                </div>
                <div className="flex min-w-0 flex-col">
                  <span className="font-medium text-sm tracking-tight">
                    Clear all Audio Files
                  </span>
                  <span className="text-muted-foreground text-xs leading-5">
                    Permanently delete all saved audio recordings from disk.
                    Transcripts will not be affected.
                  </span>
                </div>
              </div>

              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button
                    disabled={isClearingAudio}
                    size="sm"
                    variant="destructive"
                  >
                    {isClearingAudio ? "Clearing…" : "Clear All"}
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Clear all Audio Files?</AlertDialogTitle>
                    <AlertDialogDescription>
                      This will permanently delete all saved audio recordings
                      from disk. Transcripts will not be affected. This action
                      cannot be undone.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  {clearAudioError && (
                    <p className="text-destructive text-sm">
                      {clearAudioError}
                    </p>
                  )}
                  <AlertDialogFooter>
                    <AlertDialogCancel>Cancel</AlertDialogCancel>
                    <AlertDialogAction
                      onClick={handleClearAllAudio}
                      variant="destructive"
                    >
                      Clear All
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
