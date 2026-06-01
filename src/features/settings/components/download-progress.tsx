import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";

export function DownloadProgress({
  progress,
  modelId,
  onCancel,
}: {
  progress: number;
  modelId: string;
  onCancel: (id: string) => void;
}) {
  return (
    <div className="flex items-center gap-3 pt-2">
      <Progress className="h-2 flex-1" value={Math.max(progress * 100, 2)} />
      <span className="w-10 text-right font-medium text-muted-foreground text-xs">
        {Math.round(progress * 100)}%
      </span>
      <Button
        className="text-muted-foreground"
        onClick={() => onCancel(modelId)}
        size="icon-sm"
        variant="ghost"
      >
        <HugeiconsIcon
          className="size-4 text-muted-foreground"
          icon={Cancel01Icon}
          strokeWidth={2}
        />
      </Button>
    </div>
  );
}
