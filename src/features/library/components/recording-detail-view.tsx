import {
  Alert02Icon,
  ArrowDown01Icon,
  ArrowUp01Icon,
  AudioWave01Icon,
  Cancel01Icon,
  ClipboardCopyIcon,
  Search01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { format } from "date-fns";
import { AnimatePresence, motion } from "motion/react";
import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import type { RecordingProcessingStatus } from "@/features/transcription/types";
import { useRecordingsStore } from "@/stores/use-recordings-store";
import {
  formatDuration,
  type Recording,
  type TranscriptLine,
} from "@/types/recording";
import { createTranscriptSearchState } from "../utils/recording-search";
import { getRecordingStatusView } from "../utils/recording-status";
import { TranscriptLineList } from "./transcript-line-list";

const EMPTY_LINES: TranscriptLine[] = [];
const TRANSCRIPT_SEARCH_TRANSITION = {
  duration: 0.2,
  ease: [0.22, 1, 0.36, 1],
  type: "tween",
} as const;

interface RecordingDetailViewProps {
  recording: Recording;
}

interface RecordingProcessingPanelProps {
  canResume: boolean;
  isProcessing: boolean;
  isResuming: boolean;
  onResume: () => void;
  processingStatus: RecordingProcessingStatus;
  progressLabel: string | null;
  progressPercentage: number | null;
  statusLabel: string;
}

function RecordingProcessingPanel({
  canResume,
  isProcessing,
  isResuming,
  onResume,
  processingStatus,
  progressLabel,
  progressPercentage,
  statusLabel,
}: RecordingProcessingPanelProps) {
  return (
    <div className="rounded-lg border bg-muted/40 p-3">
      <div className="flex items-start gap-3">
        {isProcessing && (
          <div className="mt-0.5 shrink-0">
            <Spinner className="size-5 text-primary" />
          </div>
        )}
        {canResume && (
          <div className="mt-0.5 shrink-0">
            <HugeiconsIcon
              className="size-5 text-destructive"
              icon={Alert02Icon}
              strokeWidth={2}
            />
          </div>
        )}
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <span className="font-medium text-sm">
              {isProcessing ? "Processing audio..." : statusLabel}
            </span>
            {progressLabel && (
              <span className="text-muted-foreground text-sm">
                {progressLabel}
              </span>
            )}
          </div>
          {isProcessing && (
            <Progress className="mt-2 h-1.5" value={progressPercentage ?? 2} />
          )}
          {processingStatus.error && (
            <p className="mt-1 text-destructive text-sm">
              {processingStatus.error}
            </p>
          )}
        </div>
        {canResume && (
          <Button
            className="h-8 shrink-0 gap-2 px-3"
            disabled={isResuming}
            onClick={onResume}
            size="sm"
            variant="secondary"
          >
            {isResuming && <Spinner className="size-3" />}
            Resume
          </Button>
        )}
      </div>
    </div>
  );
}

interface TranscriptSearchBarProps {
  activeMatchIndex: number;
  hasMatches: boolean;
  matchCount: number;
  onChange: (value: string) => void;
  onNext: () => void;
  onPrevious: () => void;
  query: string;
  value: string;
}

function TranscriptSearchBar({
  activeMatchIndex,
  hasMatches,
  matchCount,
  onChange,
  onNext,
  onPrevious,
  query,
  value,
}: TranscriptSearchBarProps) {
  return (
    <motion.div
      className="flex flex-wrap items-center gap-2"
      layout
      transition={TRANSCRIPT_SEARCH_TRANSITION}
    >
      <motion.div
        className="relative min-w-56 flex-1"
        layout
        transition={TRANSCRIPT_SEARCH_TRANSITION}
      >
        <Input
          className="h-9 pr-8 pl-9"
          onChange={(event) => onChange(event.currentTarget.value)}
          placeholder="Find in transcript..."
          value={value}
        />
        <div className="absolute inset-y-0 left-3 flex items-center justify-center text-muted-foreground">
          <HugeiconsIcon icon={Search01Icon} size={15} strokeWidth={2} />
        </div>
        {value && (
          <Button
            className="absolute top-1/2 right-2 size-6 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            onClick={() => onChange("")}
            size="icon-xs"
            variant="ghost"
          >
            <HugeiconsIcon icon={Cancel01Icon} size={14} strokeWidth={2} />
          </Button>
        )}
      </motion.div>
      <AnimatePresence initial={false}>
        {query && (
          <motion.div
            animate={{ opacity: 1, width: "auto" }}
            className="flex items-center gap-1 overflow-hidden text-muted-foreground text-sm"
            exit={{ opacity: 0, width: 0 }}
            initial={{ opacity: 0, width: 0 }}
            layout
            transition={TRANSCRIPT_SEARCH_TRANSITION}
          >
            <span className="min-w-18 text-right tabular-nums">
              {hasMatches
                ? `${activeMatchIndex + 1} of ${matchCount}`
                : "No matches"}
            </span>
            <Button
              disabled={!hasMatches}
              onClick={onPrevious}
              size="icon-sm"
              title="Previous match"
              variant="ghost"
            >
              <HugeiconsIcon icon={ArrowUp01Icon} strokeWidth={2} />
              <span className="sr-only">Previous match</span>
            </Button>
            <Button
              disabled={!hasMatches}
              onClick={onNext}
              size="icon-sm"
              title="Next match"
              variant="ghost"
            >
              <HugeiconsIcon icon={ArrowDown01Icon} strokeWidth={2} />
              <span className="sr-only">Next match</span>
            </Button>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

export function RecordingDetailView({ recording }: RecordingDetailViewProps) {
  const [isResuming, setIsResuming] = useState(false);
  const [transcriptSearchText, setTranscriptSearchText] = useState("");
  const [activeMatchIndex, setActiveMatchIndex] = useState(0);
  const loadRecordingWithLines = useRecordingsStore(
    (s) => s.loadRecordingWithLines
  );
  const processingStatus = useRecordingsStore(
    (s) => s.processingStatusesByRecordingId[recording.id]
  );
  const resumeRecordingProcessing = useRecordingsStore(
    (s) => s.resumeRecordingProcessing
  );
  const updateLineText = useRecordingsStore((s) => s.updateLineText);
  const lines = useRecordingsStore(
    (s) => s.linesByRecordingId[recording.id] ?? EMPTY_LINES
  );
  const statusView = getRecordingStatusView({
    isPartial: recording.isPartial,
    processingStatus,
  });
  const progressLabel = processingStatus
    ? `${processingStatus.completedChunks} of ${processingStatus.totalChunks || "?"} parts`
    : null;
  const progressPercentage =
    processingStatus?.totalChunks && processingStatus.totalChunks > 0
      ? Math.round(
          (processingStatus.completedChunks / processingStatus.totalChunks) *
            100
        )
      : null;
  const transcriptSearch = useMemo(
    () =>
      createTranscriptSearchState(
        lines,
        transcriptSearchText,
        activeMatchIndex
      ),
    [activeMatchIndex, lines, transcriptSearchText]
  );
  const normalizedTranscriptSearchText = transcriptSearch.query;
  const hasTranscriptMatches = transcriptSearch.matches.length > 0;

  const handleCopyTranscript = async () => {
    const text = lines.map((line) => line.text).join("\n");
    if (text) {
      await navigator.clipboard.writeText(text);
    }
  };

  const handleTranscriptSearchChange = (value: string) => {
    setTranscriptSearchText(value);
    setActiveMatchIndex(0);
  };

  const handlePreviousMatch = () => {
    setActiveMatchIndex((current) =>
      transcriptSearch.matches.length > 0
        ? (current - 1 + transcriptSearch.matches.length) %
          transcriptSearch.matches.length
        : 0
    );
  };

  const handleNextMatch = () => {
    setActiveMatchIndex((current) =>
      transcriptSearch.matches.length > 0
        ? (current + 1) % transcriptSearch.matches.length
        : 0
    );
  };

  const handleResume = async () => {
    setIsResuming(true);
    try {
      await resumeRecordingProcessing(recording.id);
      await loadRecordingWithLines(recording.id);
    } finally {
      setIsResuming(false);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background">
      {/* Recording Header */}
      <div
        className="shrink-0 space-y-3 border-border/70 border-b px-8 py-6 sm:px-10 sm:py-7"
        data-tauri-drag-region
      >
        <div className="flex items-center justify-between gap-4">
          <h1 className="font-heading font-semibold text-2xl tracking-tight">
            {recording.title}
          </h1>
          <div className="flex items-center gap-1">
            {lines.length > 0 && (
              <Button
                onClick={handleCopyTranscript}
                size="icon"
                title="Copy transcription"
                variant="ghost"
              >
                <HugeiconsIcon icon={ClipboardCopyIcon} strokeWidth={2} />
                <span className="sr-only">Copy transcription</span>
              </Button>
            )}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 text-muted-foreground text-sm">
          <span>{format(new Date(recording.createdAt), "MMM d, yyyy")}</span>
          <span>•</span>
          <span>{format(new Date(recording.createdAt), "h:mm a")}</span>
          <span>•</span>
          <span>{formatDuration(recording.duration)}</span>
          {statusView.label && (
            <Badge
              className="h-5 rounded-md px-1.5 font-medium"
              variant="secondary"
            >
              {statusView.label}
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
        {processingStatus && statusView.label && (
          <RecordingProcessingPanel
            canResume={statusView.canResume}
            isProcessing={statusView.isActive}
            isResuming={isResuming}
            onResume={handleResume}
            processingStatus={processingStatus}
            progressLabel={progressLabel}
            progressPercentage={progressPercentage}
            statusLabel={statusView.label}
          />
        )}
        {lines.length > 0 && (
          <TranscriptSearchBar
            activeMatchIndex={transcriptSearch.activeMatchIndex}
            hasMatches={hasTranscriptMatches}
            matchCount={transcriptSearch.matches.length}
            onChange={handleTranscriptSearchChange}
            onNext={handleNextMatch}
            onPrevious={handlePreviousMatch}
            query={normalizedTranscriptSearchText}
            value={transcriptSearchText}
          />
        )}
      </div>

      {/* Transcript Content */}
      <ScrollArea className="min-h-0 flex-1">
        <div className="px-8 py-6 pb-8 sm:px-10">
          <TranscriptLineList
            lines={lines}
            onUpdateLineText={updateLineText}
            searchQuery={normalizedTranscriptSearchText}
            searchResults={transcriptSearch.lines}
          />
        </div>
      </ScrollArea>
    </div>
  );
}
