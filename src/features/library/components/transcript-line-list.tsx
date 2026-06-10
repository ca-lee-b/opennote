import { useEffect, useRef, useState } from "react";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import type { TranscriptLine } from "@/types/recording";
import type { HighlightSegment } from "../utils/recording-search";

function formatSecondsToTime(totalSeconds: number): string {
  const mins = Math.floor(totalSeconds / 60);
  const secs = Math.floor(totalSeconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

interface TranscriptLineListProps {
  lines: TranscriptLine[];
  onUpdateLineText: (lineId: string, text: string) => Promise<void>;
  searchQuery?: string;
  searchResults?: Array<{
    line: TranscriptLine;
    segments: HighlightSegment[];
  }>;
}

export function TranscriptLineList({
  lines,
  onUpdateLineText,
  searchQuery = "",
  searchResults,
}: TranscriptLineListProps) {
  const visibleLines =
    searchResults ?? lines.map((line) => ({ line, segments: null }));

  if (visibleLines.length === 0) {
    return (
      <div className="rounded-xl border border-border/70 border-dashed bg-background/40 py-12 text-center text-muted-foreground text-sm">
        No transcript lines yet.
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {visibleLines.map(({ line, segments }) => (
        <TranscriptLineEditor
          elapsed={formatSecondsToTime(line.startTimeSecs)}
          key={line.id}
          line={line}
          onUpdateLineText={onUpdateLineText}
          searchQuery={searchQuery}
          segments={segments}
        />
      ))}
    </div>
  );
}

interface TranscriptLineEditorProps {
  elapsed: string;
  line: TranscriptLine;
  onUpdateLineText: (lineId: string, text: string) => Promise<void>;
  searchQuery: string;
  segments: HighlightSegment[] | null;
}

function TranscriptLineEditor({
  elapsed,
  line,
  onUpdateLineText,
  searchQuery,
  segments,
}: TranscriptLineEditorProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [text, setText] = useState(line.text);
  const rowRef = useRef<HTMLDivElement>(null);
  const isActiveMatch = Boolean(segments?.some((segment) => segment.isActive));

  useEffect(() => {
    setText(line.text);
  }, [line.text]);

  useEffect(() => {
    if (isActiveMatch) {
      rowRef.current?.scrollIntoView({ block: "center", behavior: "smooth" });
    }
  }, [isActiveMatch]);

  const handleBlur = () => {
    setIsEditing(false);
    const nextText = text.trim();
    if (nextText !== line.text) {
      onUpdateLineText(line.id, nextText).catch((error) => {
        console.error("Failed to update transcript line:", error);
      });
    }
  };

  return (
    <div ref={rowRef}>
      <div className="flex items-start gap-4 py-1">
        <span
          className={cn(
            "w-12 shrink-0 pt-2 text-right font-medium text-muted-foreground text-xs tabular-nums",
            isActiveMatch && "text-foreground"
          )}
        >
          {elapsed}
        </span>
        <div className="relative min-h-10 flex-1">
          {isEditing ? (
            <Textarea
              autoFocus
              className="min-h-10 flex-1 resize-none rounded-md border-transparent bg-transparent px-0 py-0 text-[15px] text-foreground leading-7 shadow-none focus-visible:border-transparent focus-visible:ring-0"
              onBlur={handleBlur}
              onChange={(event) => setText(event.currentTarget.value)}
              value={text}
            />
          ) : (
            <button
              className="block min-h-10 w-full whitespace-pre-wrap break-words rounded-md py-0 text-left text-[15px] text-foreground leading-7 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
              onClick={() => setIsEditing(true)}
              type="button"
            >
              {searchQuery && segments ? (
                <HighlightedTranscriptText segments={segments} />
              ) : (
                line.text
              )}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function HighlightedTranscriptText({
  segments,
}: {
  segments: HighlightSegment[];
}) {
  return (
    <>
      {segments.map((segment) =>
        segment.isMatch ? (
          <mark
            className={cn(
              "rounded-sm px-0.5 text-foreground",
              segment.isActive ? "bg-primary/25" : "bg-primary/15"
            )}
            key={`${segment.start}-${segment.text}`}
          >
            {segment.text}
          </mark>
        ) : (
          <span key={`${segment.start}-${segment.text}`}>{segment.text}</span>
        )
      )}
    </>
  );
}
