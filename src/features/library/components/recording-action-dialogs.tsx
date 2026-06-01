import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { RecordingActions } from "./use-recording-actions";

interface RecordingActionDialogsProps {
  actions: RecordingActions;
}

export function RecordingActionDialogs({
  actions,
}: RecordingActionDialogsProps) {
  const {
    isRenameDialogOpen,
    setIsRenameDialogOpen,
    isDeleteDialogOpen,
    setIsDeleteDialogOpen,
    newTitle,
    setNewTitle,
    handleRename,
    handleDelete,
    deleteRecordingCount,
  } = actions;
  const isBatchDelete = deleteRecordingCount > 1;

  return (
    <>
      <Dialog onOpenChange={setIsRenameDialogOpen} open={isRenameDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Recording</DialogTitle>
            <DialogDescription>
              Enter a new title for this recording.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Input
              autoFocus
              onChange={(e) => setNewTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleRename();
                }
              }}
              value={newTitle}
            />
          </div>
          <DialogFooter>
            <Button
              onClick={() => setIsRenameDialogOpen(false)}
              variant="ghost"
            >
              Cancel
            </Button>
            <Button onClick={handleRename}>Rename</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog onOpenChange={setIsDeleteDialogOpen} open={isDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              Delete{" "}
              {isBatchDelete
                ? `${deleteRecordingCount} Recordings`
                : "Recording"}
              ?
            </DialogTitle>
            <DialogDescription>
              This will permanently delete{" "}
              {isBatchDelete
                ? "these recordings and their transcripts"
                : "this recording and transcript"}
              . This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              onClick={() => setIsDeleteDialogOpen(false)}
              variant="ghost"
            >
              Cancel
            </Button>
            <Button onClick={handleDelete} variant="destructive">
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
