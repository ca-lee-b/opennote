import { AudioWave01Icon, TextAlignLeftIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export function SidebarEmptyState() {
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
      <h3 className="font-semibold text-foreground text-sm">No recordings</h3>
      <p className="mt-1 text-muted-foreground text-xs">
        Record something to see it here.
      </p>
    </div>
  );
}

export function DetailPlaceholder() {
  return (
    <div className="flex h-full w-full flex-col items-center justify-center p-8 text-center">
      <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-md border border-border bg-background">
        <HugeiconsIcon
          className="text-muted-foreground"
          icon={TextAlignLeftIcon}
          size={24}
          strokeWidth={2}
        />
      </div>
      <h3 className="font-semibold text-foreground text-lg">
        Select a Recording
      </h3>
      <p className="mt-2 max-w-xs text-muted-foreground text-sm leading-6">
        Choose a transcription from the sidebar, or start a new recording.
      </p>
    </div>
  );
}
