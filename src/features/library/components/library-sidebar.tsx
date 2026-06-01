import {
  Cancel01Icon,
  Mic01Icon,
  Search01Icon,
  Settings02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { type MouseEvent, useMemo, useRef } from "react";
import { Button } from "@/components/ui/button";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarInput,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";

import { cn } from "@/lib/utils";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import { SidebarEmptyState } from "./empty-states";
import { RecordingSidebarRow } from "./recording-sidebar-row";

interface LibrarySidebarProps {
  onOpenRecording?: () => void;
  onOpenSettings?: () => void;
}

export function LibrarySidebar({
  onOpenRecording,
  onOpenSettings,
}: LibrarySidebarProps) {
  const { state } = useSidebar();
  const isCollapsed = state === "collapsed";

  const searchText = useRecordingsStore((s) => s.searchText);
  const setSearchText = useRecordingsStore((s) => s.setSearchText);
  const recordings = useRecordingsStore((s) => s.recordings);
  const createRecording = useRecordingsStore((s) => s.createRecording);
  const selectedRecordingIds = useRecordingsStore(
    (s) => s.selectedRecordingIds
  );
  const selectRecording = useRecordingsStore((s) => s.selectRecording);
  const selectRecordings = useRecordingsStore((s) => s.selectRecordings);
  const selectionAnchorId = useRef<string | null>(null);

  const filteredRecordings = useMemo(() => {
    if (!searchText) {
      return recordings;
    }
    const lower = searchText.toLowerCase();
    return recordings.filter(
      (recording) =>
        recording.title.toLowerCase().includes(lower) ||
        recording.fullText.toLowerCase().includes(lower)
    );
  }, [recordings, searchText]);

  const handleSelectRecording = (
    recordingId: string,
    event: MouseEvent<HTMLButtonElement>
  ) => {
    if (event.shiftKey && selectionAnchorId.current) {
      const visibleIds = filteredRecordings.map((recording) => recording.id);
      const anchorIndex = visibleIds.indexOf(selectionAnchorId.current);
      const recordingIndex = visibleIds.indexOf(recordingId);

      if (anchorIndex !== -1 && recordingIndex !== -1) {
        const start = Math.min(anchorIndex, recordingIndex);
        const end = Math.max(anchorIndex, recordingIndex);
        selectRecordings(visibleIds.slice(start, end + 1), recordingId);
        return;
      }
    }

    if (event.metaKey || event.ctrlKey) {
      const nextSelectedIds = selectedRecordingIds.includes(recordingId)
        ? selectedRecordingIds.filter((id) => id !== recordingId)
        : [...selectedRecordingIds, recordingId];
      selectRecordings(nextSelectedIds, recordingId);
      selectionAnchorId.current = recordingId;
      return;
    }

    selectRecording(recordingId);
    selectionAnchorId.current = recordingId;
  };

  const handleOpenRecordingContextMenu = (recordingId: string) => {
    if (!selectedRecordingIds.includes(recordingId)) {
      selectRecording(recordingId);
      selectionAnchorId.current = recordingId;
    }
  };

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader className={cn("gap-3", isCollapsed ? "p-2" : "mt-6 p-3")}>
        <SidebarMenuButton
          className="rounded-md font-medium"
          onClick={onOpenRecording ?? (() => createRecording())}
          tooltip="New Recording"
        >
          <HugeiconsIcon icon={Mic01Icon} size={20} strokeWidth={2.5} />
          <span>New Recording</span>
        </SidebarMenuButton>

        {!isCollapsed && (
          <div className="relative">
            <SidebarInput
              className="pr-8 pl-9"
              onChange={(e) => setSearchText(e.target.value)}
              placeholder="Search recordings..."
              value={searchText}
            />
            <div className="absolute inset-y-0 left-3 flex items-center justify-center text-muted-foreground">
              <HugeiconsIcon icon={Search01Icon} size={16} strokeWidth={2} />
            </div>
            {searchText && (
              <Button
                className="absolute inset-y-0 right-1 h-auto w-8 text-muted-foreground hover:text-foreground"
                onClick={() => setSearchText("")}
                size="icon-xs"
                variant="ghost"
              >
                <HugeiconsIcon icon={Cancel01Icon} size={14} strokeWidth={2} />
              </Button>
            )}
          </div>
        )}
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {filteredRecordings.length > 0 ? (
                filteredRecordings.map((recording) => (
                  <SidebarMenuItem key={recording.id}>
                    <RecordingSidebarRow
                      onOpenContextMenu={handleOpenRecordingContextMenu}
                      onSelect={handleSelectRecording}
                      recording={recording}
                    />
                  </SidebarMenuItem>
                ))
              ) : (
                <SidebarEmptyState />
              )}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className={cn(isCollapsed ? "p-2" : "p-3")}>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              className="rounded-md"
              onClick={onOpenSettings}
              tooltip="Settings"
            >
              <HugeiconsIcon icon={Settings02Icon} size={20} strokeWidth={2} />
              <span className="font-medium">Settings</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
