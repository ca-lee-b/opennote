import { useState } from "react";
import { Textarea } from "@/components/ui/textarea";
import type { TranscriptLine } from "@/types/recording";

function parseTimeToSeconds(timeStr: string): number {
  const parts = timeStr.split(":").map(Number);
  if (parts.length === 3) {
    return parts[0] * 3600 + parts[1] * 60 + parts[2];
  }
  if (parts.length === 2) {
    return parts[0] * 60 + parts[1];
  }
  return 0;
}

function formatSecondsToTime(totalSeconds: number): string {
  const mins = Math.floor(totalSeconds / 60);
  const secs = Math.floor(totalSeconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

function computeElapsedDifferences(lines: TranscriptLine[]): string[] {
  if (lines.length === 0) {
    return [];
  }

  const diffs: string[] = [];
  let prevSeconds = 0;

  for (let i = 0; i < lines.length; i++) {
    const currentSeconds = parseTimeToSeconds(lines[i].startTime);
    if (i === 0) {
      diffs.push("00:00");
      prevSeconds = currentSeconds;
    } else {
      const diff = currentSeconds - prevSeconds;
      diffs.push(formatSecondsToTime(Math.max(0, diff)));
      prevSeconds = currentSeconds;
    }
  }

  return diffs;
}

interface TranscriptLineListProps {
  lines: TranscriptLine[];
  onUpdateLineText: (lineId: string, text: string) => Promise<void>;
}

export function TranscriptLineList({
  lines,
  onUpdateLineText,
}: TranscriptLineListProps) {
  if (lines.length === 0) {
    return (
      <div className="rounded-xl border border-border/70 border-dashed bg-background/40 py-12 text-center text-muted-foreground text-sm">
        No transcript lines yet.
      </div>
    );
  }

  const elapsedDifferences = computeElapsedDifferences(lines);

  return (
    <div className="flex flex-col">
      {lines.map((line, index) => (
        <TranscriptLineEditor
          elapsed={elapsedDifferences[index]}
          key={line.id}
          line={line}
          onUpdateLineText={onUpdateLineText}
        />
      ))}
    </div>
  );
}

interface TranscriptLineEditorProps {
  elapsed: string;
  line: TranscriptLine;
  onUpdateLineText: (lineId: string, text: string) => Promise<void>;
}

function TranscriptLineEditor({
  elapsed,
  line,
  onUpdateLineText,
}: TranscriptLineEditorProps) {
  const [text, setText] = useState(line.text);

  return (
    <div>
      <div className="flex items-start gap-4 py-2">
        <span className="w-12 shrink-0 pt-2 text-right font-medium text-muted-foreground text-xs tabular-nums">
          {elapsed}
        </span>
        <Textarea
          className="min-h-10 flex-1 rounded-md border-transparent bg-transparent px-0 py-0 text-[15px] text-foreground leading-7 shadow-none focus-visible:border-transparent focus-visible:ring-0"
          onBlur={() => {
            const nextText = text.trim();
            if (nextText !== line.text) {
              onUpdateLineText(line.id, nextText).catch((error) => {
                console.error("Failed to update transcript line:", error);
              });
            }
          }}
          onChange={(event) => setText(event.currentTarget.value)}
          value={text}
        />
      </div>
    </div>
  );
}
