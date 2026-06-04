import { AudioWave01Icon, File01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { formatDistanceToNow } from "date-fns";
import type { MouseEvent } from "react";
import { Badge } from "@/components/ui/badge";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { SidebarMenuButton, useSidebar } from "@/components/ui/sidebar";
import { cn } from "@/lib/utils";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import { formatDuration, type Recording } from "@/types/recording";
import { RecordingActionDialogs } from "./recording-action-dialogs";
import { RecordingMoreActionsMenuItems } from "./recording-more-actions-menu-items";
import { RecordingSidebarExportContextSubmenu } from "./recording-sidebar-export-context-submenu";
import { useRecordingActions } from "./use-recording-actions";

interface RecordingSidebarRowProps {
  onOpenContextMenu: (recordingId: string) => void;
  onSelect: (recordingId: string, event: MouseEvent<HTMLButtonElement>) => void;
  recording: Recording;
}

export function RecordingSidebarRow({
  onOpenContextMenu,
  onSelect,
  recording,
}: RecordingSidebarRowProps) {
  const { state } = useSidebar();
  const isCollapsed = state === "collapsed";

  const selectedRecordingIds = useRecordingsStore(
    (state) => state.selectedRecordingIds
  );
  const isSelected = selectedRecordingIds.includes(recording.id);
  const deleteRecordingIds = isSelected ? selectedRecordingIds : [recording.id];

  const actions = useRecordingActions(recording, {
    deleteRecordingIds,
  });

  const icon = recording.audioPath ? AudioWave01Icon : File01Icon;

  return (
    <>
      <ContextMenu
        onOpenChange={(open) => {
          if (open) {
            onOpenContextMenu(recording.id);
          }
        }}
      >
        <ContextMenuTrigger asChild>
          <SidebarMenuButton
            className={cn(
              "rounded-md border border-transparent px-3 py-2.5 transition-colors",
              !isCollapsed && "min-h-16",
              isSelected
                ? "bg-background text-foreground ring-1 ring-border"
                : "hover:bg-background/70"
            )}
            isActive={isSelected}
            onClick={(event) => onSelect(recording.id, event)}
            tooltip={recording.title}
          >
            {isCollapsed && (
              <HugeiconsIcon icon={icon} size={18} strokeWidth={2} />
            )}
            {!isCollapsed && (
              <div className="flex min-w-0 flex-1 flex-col items-start gap-0.5">
                <span
                  className={cn(
                    "w-full truncate text-sm leading-tight tracking-tight",
                    isSelected ? "font-semibold" : "font-medium"
                  )}
                >
                  {recording.title}
                </span>
                <span className="w-full truncate text-muted-foreground text-xs leading-5">
                  {formatDistanceToNow(new Date(recording.createdAt), {
                    addSuffix: true,
                  })}{" "}
                  • {formatDuration(recording.duration)}
                </span>
                {recording.isPartial && (
                  <Badge
                    className="mt-1 h-4 rounded-md px-1.5 font-medium text-[10px]"
                    variant="secondary"
                  >
                    Incomplete
                  </Badge>
                )}
              </div>
            )}
          </SidebarMenuButton>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <RecordingSidebarExportContextSubmenu />
          <ContextMenuSeparator />
          <RecordingMoreActionsMenuItems
            Item={ContextMenuItem}
            onDelete={() => actions.setIsDeleteDialogOpen(true)}
            onRename={() => actions.setIsRenameDialogOpen(true)}
            Separator={ContextMenuSeparator}
            showRename={deleteRecordingIds.length === 1}
          />
        </ContextMenuContent>
      </ContextMenu>
      <RecordingActionDialogs actions={actions} />
    </>
  );
}
