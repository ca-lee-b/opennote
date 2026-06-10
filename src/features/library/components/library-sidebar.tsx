import {
  AudioWave01Icon,
  Cancel01Icon,
  Clock01Icon,
  Mic01Icon,
  Note01Icon,
  Search01Icon,
  Settings02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { open } from "@tauri-apps/plugin-dialog";
import { ChevronDown, Upload } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { type MouseEvent, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { ButtonGroup } from "@/components/ui/button-group";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarInput,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";

import { cn } from "@/lib/utils";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import { searchRecordings } from "../utils/recording-search";
import { SidebarEmptyState } from "./empty-states";
import { RecordingSidebarRow } from "./recording-sidebar-row";

interface LibrarySidebarProps {
  onOpenRecording?: () => void;
  onOpenSettings?: () => void;
}

type LibrarySection = "all" | "recents";

const LIST_ITEM_TRANSITION = {
  duration: 0.2,
  ease: [0.22, 1, 0.36, 1],
  type: "tween",
} as const;

function RecentsEmptyState() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center rounded-xl border border-border/70 border-dashed bg-background/40 p-8 text-center">
      <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-md border border-border bg-background">
        <HugeiconsIcon
          className="text-muted-foreground"
          icon={AudioWave01Icon}
          size={24}
          strokeWidth={2}
        />
      </div>
      <h3 className="font-semibold text-foreground text-sm">
        No recent recordings
      </h3>
      <p className="mt-1 text-muted-foreground text-xs">
        Open a recording to see it here.
      </p>
    </div>
  );
}

function SearchEmptyState() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center rounded-xl border border-border/70 border-dashed bg-background/40 p-8 text-center">
      <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-md border border-border bg-background">
        <HugeiconsIcon
          className="text-muted-foreground"
          icon={Search01Icon}
          size={24}
          strokeWidth={2}
        />
      </div>
      <h3 className="font-semibold text-foreground text-sm">No matches</h3>
      <p className="mt-1 text-muted-foreground text-xs">
        Try another recording title or transcript phrase.
      </p>
    </div>
  );
}

function getEmptyState({
  hasSearch,
  section,
  visibleRecordingCount,
}: {
  hasSearch: boolean;
  section: LibrarySection;
  visibleRecordingCount: number;
}) {
  if (hasSearch && visibleRecordingCount > 0) {
    return SearchEmptyState;
  }

  if (section === "recents" && visibleRecordingCount === 0) {
    return RecentsEmptyState;
  }

  return SidebarEmptyState;
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
  const recentRecordingIds = useRecordingsStore((s) => s.recentRecordingIds);
  const createRecording = useRecordingsStore((s) => s.createRecording);
  const importAudioFile = useRecordingsStore((s) => s.importAudioFile);
  const isImportingAudio = useRecordingsStore((s) => s.isImportingAudio);
  const selectedRecordingIds = useRecordingsStore(
    (s) => s.selectedRecordingIds
  );
  const selectRecording = useRecordingsStore((s) => s.selectRecording);
  const selectRecordings = useRecordingsStore((s) => s.selectRecordings);
  const selectionAnchorId = useRef<string | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [section, setSection] = useState<LibrarySection>("all");

  const visibleRecordings = useMemo(() => {
    if (section === "all") {
      return recordings;
    }

    const recordingsById = new Map(
      recordings.map((recording) => [recording.id, recording])
    );
    return recentRecordingIds.flatMap((id) => {
      const recording = recordingsById.get(id);
      return recording ? [recording] : [];
    });
  }, [recentRecordingIds, recordings, section]);

  const filteredRecordingResults = useMemo(
    () => searchRecordings(visibleRecordings, searchText),
    [searchText, visibleRecordings]
  );
  const hasSearch = Boolean(searchText.trim());
  const EmptyState = getEmptyState({
    hasSearch,
    section,
    visibleRecordingCount: visibleRecordings.length,
  });

  const handleSelectRecording = (
    recordingId: string,
    event: MouseEvent<HTMLButtonElement>
  ) => {
    if (event.shiftKey && selectionAnchorId.current) {
      const visibleIds = filteredRecordingResults.map(
        ({ recording }) => recording.id
      );
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
  const resultLabel =
    hasSearch && visibleRecordings.length > 0
      ? `${filteredRecordingResults.length} ${
          filteredRecordingResults.length === 1 ? "result" : "results"
        }`
      : null;

  const handleOpenRecordingContextMenu = (recordingId: string) => {
    if (!selectedRecordingIds.includes(recordingId)) {
      selectRecording(recordingId);
      selectionAnchorId.current = recordingId;
    }
  };

  const handleImportAudio = async () => {
    setImportError(null);
    let selected: string | null = null;
    try {
      selected = await open({
        filters: [{ name: "Audio", extensions: ["wav", "mp3"] }],
        multiple: false,
      });
    } catch {
      return;
    }
    const sourceAudioPath = Array.isArray(selected) ? selected[0] : selected;
    if (!sourceAudioPath) {
      return;
    }

    const toastId = toast.loading("Importing audio file…");
    try {
      await importAudioFile(sourceAudioPath);
      toast.success("Audio file imported", { id: toastId });
    } catch (error) {
      const message = String(error);
      console.error("Failed to import audio:", error);
      setImportError(message);
      toast.error("Failed to import audio", {
        id: toastId,
        description: message,
      });
    }
  };

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader
        className={cn("gap-3", isCollapsed ? "px-2 pt-8 pb-2" : "mt-6 p-3")}
        data-tauri-drag-region
      >
        <div>
          <ButtonGroup className="w-full">
            <Button
              className={cn(
                "min-w-0 justify-start font-medium",
                isCollapsed ? "size-9 px-0" : "flex-1"
              )}
              onClick={onOpenRecording ?? (() => createRecording())}
              size="sm"
              title="New Recording"
              variant="ghost"
            >
              <HugeiconsIcon icon={Mic01Icon} size={20} strokeWidth={2.5} />
              {!isCollapsed && <span>New Recording</span>}
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  className={cn(isCollapsed ? "size-9 px-0" : "px-2")}
                  size="sm"
                  title="Recording options"
                  variant="ghost"
                >
                  <ChevronDown className="size-4" />
                  <span className="sr-only">Recording options</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  disabled={isImportingAudio}
                  onSelect={handleImportAudio}
                >
                  <Upload className="size-4" />
                  Upload audio file
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </ButtonGroup>
          {importError && !isCollapsed && (
            <p className="mt-2 px-2 text-destructive text-xs">{importError}</p>
          )}
        </div>

        {!isCollapsed && (
          <div className="relative">
            <SidebarInput
              className="h-9 px-10 py-0 leading-none"
              onChange={(e) => setSearchText(e.target.value)}
              placeholder="Search recordings..."
              value={searchText}
            />
            <div className="absolute top-1/2 left-2 flex size-6 -translate-y-1/2 items-center justify-center text-muted-foreground">
              <HugeiconsIcon icon={Search01Icon} size={16} strokeWidth={2} />
            </div>
            {searchText && (
              <Button
                className="absolute top-1/2 right-2 size-6 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                onClick={() => setSearchText("")}
                size="icon-xs"
                variant="ghost"
              >
                <HugeiconsIcon icon={Cancel01Icon} size={14} strokeWidth={2} />
              </Button>
            )}
            {resultLabel && (
              <div className="mt-1 pl-3 text-muted-foreground text-xs">
                {resultLabel}
              </div>
            )}
          </div>
        )}
        <SidebarMenu className="gap-y-1">
          <SidebarMenuItem>
            <SidebarMenuButton
              isActive={section === "all"}
              onClick={() => setSection("all")}
              size="sm"
              tooltip="All Notes"
            >
              <HugeiconsIcon icon={Note01Icon} strokeWidth={2} />
              <span>All Notes</span>
            </SidebarMenuButton>
            <SidebarMenuBadge>{recordings.length}</SidebarMenuBadge>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton
              isActive={section === "recents"}
              onClick={() => setSection("recents")}
              size="sm"
              tooltip="Recents"
            >
              <HugeiconsIcon icon={Clock01Icon} strokeWidth={2} />
              <span>Recents</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu className="gap-y-1">
              {filteredRecordingResults.length > 0 ? null : <EmptyState />}
              <AnimatePresence initial={false}>
                {filteredRecordingResults.map((result) => (
                  <motion.li
                    animate={{ opacity: 1, y: 0 }}
                    className="group/menu-item relative"
                    data-sidebar="menu-item"
                    data-slot="sidebar-menu-item"
                    exit={{ opacity: 0, y: -4 }}
                    initial={{ opacity: 0, y: 4 }}
                    key={result.recording.id}
                    layout
                    transition={LIST_ITEM_TRANSITION}
                  >
                    <RecordingSidebarRow
                      onOpenContextMenu={handleOpenRecordingContextMenu}
                      onSelect={handleSelectRecording}
                      recording={result.recording}
                      searchMatch={{
                        snippet: result.snippet,
                        titleSegments: result.titleSegments,
                      }}
                    />
                  </motion.li>
                ))}
              </AnimatePresence>
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
