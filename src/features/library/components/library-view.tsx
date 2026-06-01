import { MoreHorizontalIcon, Share04Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { buttonVariants } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import { DetailPlaceholder } from "@/features/library/components/empty-states";
import { LibrarySidebar } from "@/features/library/components/library-sidebar";
import { RecordingActionDialogs } from "@/features/library/components/recording-action-dialogs";
import { RecordingDetailView } from "@/features/library/components/recording-detail-view";
import { RecordingExportMenuItems } from "@/features/library/components/recording-export-menu-items";
import { RecordingMoreActionsMenuItems } from "@/features/library/components/recording-more-actions-menu-items";
import { ActiveRecordingDialog } from "@/features/recording/components/active-recording-dialog";
import { SettingsView } from "@/features/settings/components/settings-view";
import { cn } from "@/lib/utils";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import type { Recording } from "@/types/recording";
import { useRecordingActions } from "./use-recording-actions";

function RecordingToolbar({ recording }: { recording: Recording }) {
  const selectRecording = useRecordingsStore((s) => s.selectRecording);
  const actions = useRecordingActions(recording, {
    onDeleted: () => selectRecording(null),
  });

  return (
    <>
      <div className="flex items-center gap-1">
        <DropdownMenu>
          <DropdownMenuTrigger
            className={cn(buttonVariants({ size: "icon", variant: "ghost" }))}
          >
            <HugeiconsIcon icon={Share04Icon} strokeWidth={2} />
            <span className="sr-only">Export</span>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <RecordingExportMenuItems Item={DropdownMenuItem} />
          </DropdownMenuContent>
        </DropdownMenu>

        <DropdownMenu>
          <DropdownMenuTrigger
            className={cn(buttonVariants({ size: "icon", variant: "ghost" }))}
          >
            <HugeiconsIcon icon={MoreHorizontalIcon} strokeWidth={2} />
            <span className="sr-only">More actions</span>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <RecordingMoreActionsMenuItems
              Item={DropdownMenuItem}
              onDelete={() => actions.setIsDeleteDialogOpen(true)}
              onRename={() => actions.setIsRenameDialogOpen(true)}
              Separator={DropdownMenuSeparator}
            />
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <RecordingActionDialogs actions={actions} />
    </>
  );
}

export function LibraryView() {
  const initialize = useRecordingsStore((s) => s.initialize);
  const isLoading = useRecordingsStore((s) => s.isLoading);
  const recordings = useRecordingsStore((s) => s.recordings);
  const selectedRecordingId = useRecordingsStore((s) => s.selectedRecordingId);
  const selectedRecording =
    recordings.find((recording) => recording.id === selectedRecordingId) ??
    null;

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isRecordingOpen, setIsRecordingOpen] = useState(false);

  useEffect(() => {
    initialize();
  }, [initialize]);

  if (isLoading) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-background">
        <Spinner className="size-6" />
      </div>
    );
  }

  return (
    <SidebarProvider>
      <LibrarySidebar
        onOpenRecording={() => setIsRecordingOpen(true)}
        onOpenSettings={() => setIsSettingsOpen(true)}
      />

      <SidebarInset>
        <div className="flex h-svh flex-col">
          {/* Titlebar area with sidebar toggle and recording actions */}
          <div
            className="flex h-12 shrink-0 items-center justify-end gap-1 border-border/70 border-b bg-background/90 px-2 backdrop-blur-[2px]"
            data-tauri-drag-region
          >
            {selectedRecording && (
              <RecordingToolbar recording={selectedRecording} />
            )}
            <SidebarTrigger />
          </div>

          {/* Detail pane */}
          <div className="min-h-0 flex-1 overflow-hidden">
            {selectedRecording ? (
              <RecordingDetailView
                key={selectedRecording.id}
                recording={selectedRecording}
              />
            ) : (
              <DetailPlaceholder />
            )}
          </div>
        </div>
      </SidebarInset>

      {/* Recording Dialog */}
      <ActiveRecordingDialog
        onOpenChange={setIsRecordingOpen}
        open={isRecordingOpen}
      />

      {/* Settings Sheet */}
      <Sheet onOpenChange={setIsSettingsOpen} open={isSettingsOpen}>
        <SheetContent className="w-140" side="right">
          <SheetHeader className="sr-only">
            <SheetTitle>Settings</SheetTitle>
          </SheetHeader>
          <SettingsView />
        </SheetContent>
      </Sheet>
    </SidebarProvider>
  );
}
