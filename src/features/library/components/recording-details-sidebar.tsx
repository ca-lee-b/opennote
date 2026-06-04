import { format } from "date-fns";
import { ScrollArea } from "@/components/ui/scroll-area";
import { formatDuration, type Recording } from "@/types/recording";

interface RecordingDetailsSidebarProps {
  recording: Recording;
}

interface DetailRowProps {
  label: string;
  value: string;
}

const AUDIO_PATH_SEPARATOR = /[\\/]/;

function DetailRow({ label, value }: DetailRowProps) {
  return (
    <div className="flex flex-col gap-1">
      <dt className="text-muted-foreground text-xs">{label}</dt>
      <dd className="break-words text-foreground text-sm">{value}</dd>
    </div>
  );
}

function getAudioBasename(audioPath: string): string {
  return audioPath.split(AUDIO_PATH_SEPARATOR).pop() ?? audioPath;
}

export function RecordingDetailsSidebar({
  recording,
}: RecordingDetailsSidebarProps) {
  const createdAt = new Date(recording.createdAt);

  return (
    <aside className="h-full w-64 shrink-0 border-border/70 border-l bg-background">
      <ScrollArea className="h-full">
        <div className="flex flex-col gap-5 p-5">
          <div>
            <p className="font-semibold text-sm">Metadata</p>
          </div>
          <dl className="flex flex-col gap-4">
            <DetailRow label="Title" value={recording.title} />
            <DetailRow
              label="Created"
              value={format(createdAt, "MMM d, yyyy")}
            />
            <DetailRow label="Time" value={format(createdAt, "h:mm a")} />
            <DetailRow
              label="Duration"
              value={formatDuration(recording.duration)}
            />
            <DetailRow
              label="Characters"
              value={recording.fullText.length.toLocaleString()}
            />
            {recording.audioPath && (
              <DetailRow
                label="Audio file"
                value={getAudioBasename(recording.audioPath)}
              />
            )}
          </dl>
        </div>
      </ScrollArea>
    </aside>
  );
}
