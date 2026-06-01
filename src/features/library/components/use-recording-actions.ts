import { useEffect, useState } from "react";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import type { Recording } from "@/types/recording";

interface UseRecordingActionsOptions {
  deleteRecordingIds?: string[];
  onDeleted?: () => void;
}

export function useRecordingActions(
  recording: Recording,
  options?: UseRecordingActionsOptions
) {
  const renameRecording = useRecordingsStore((s) => s.renameRecording);
  const deleteRecordings = useRecordingsStore((s) => s.deleteRecordings);

  const [isRenameDialogOpen, setIsRenameDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [newTitle, setNewTitle] = useState(recording.title);

  useEffect(() => {
    setNewTitle(recording.title);
  }, [recording.title]);

  const handleRename = async () => {
    if (newTitle.trim() && newTitle !== recording.title) {
      await renameRecording(recording.id, newTitle.trim());
    }
    setIsRenameDialogOpen(false);
  };

  const handleDelete = async () => {
    await deleteRecordings(options?.deleteRecordingIds ?? [recording.id]);
    setIsDeleteDialogOpen(false);
    options?.onDeleted?.();
  };

  return {
    deleteRecordingCount: options?.deleteRecordingIds?.length ?? 1,
    isRenameDialogOpen,
    setIsRenameDialogOpen,
    isDeleteDialogOpen,
    setIsDeleteDialogOpen,
    newTitle,
    setNewTitle,
    handleRename,
    handleDelete,
  };
}

export type RecordingActions = ReturnType<typeof useRecordingActions>;
