import { AudioWave01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { format } from "date-fns";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import {
  formatDuration,
  type Recording,
  type TranscriptLine,
} from "@/types/recording";
import { TranscriptLineList } from "./transcript-line-list";

const EMPTY_LINES: TranscriptLine[] = [];

interface RecordingDetailViewProps {
  recording: Recording;
}

export function RecordingDetailView({ recording }: RecordingDetailViewProps) {
  const updateLineText = useRecordingsStore((s) => s.updateLineText);
  const lines = useRecordingsStore(
    (s) => s.linesByRecordingId[recording.id] ?? EMPTY_LINES
  );

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background">
      {/* Recording Header */}
      <div className="shrink-0 space-y-3 border-border/70 border-b px-8 py-6 sm:px-10 sm:py-7">
        <h1 className="font-heading font-semibold text-2xl tracking-tight">
          {recording.title}
        </h1>
        <div className="flex flex-wrap items-center gap-2 text-muted-foreground text-sm">
          <span>{format(new Date(recording.createdAt), "MMM d, yyyy")}</span>
          <span>•</span>
          <span>{format(new Date(recording.createdAt), "h:mm a")}</span>
          <span>•</span>
          <span>{formatDuration(recording.duration)}</span>
          {recording.isPartial && (
            <Badge
              className="h-5 rounded-md px-1.5 font-medium"
              variant="secondary"
            >
              Incomplete
            </Badge>
          )}
          {recording.audioPath && (
            <Badge
              className="flex h-5 items-center gap-1.5 rounded-md px-1.5 font-medium"
              variant="secondary"
            >
              <HugeiconsIcon
                className="size-3"
                icon={AudioWave01Icon}
                strokeWidth={2}
              />
              Audio
            </Badge>
          )}
        </div>
      </div>

      {/* Transcript Content */}
      <ScrollArea className="min-h-0 flex-1">
        <div className="px-8 py-6 pb-8 sm:px-10">
          <TranscriptLineList lines={lines} onUpdateLineText={updateLineText} />
        </div>
      </ScrollArea>
    </div>
  );
}
