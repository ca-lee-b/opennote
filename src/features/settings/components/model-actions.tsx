import { CloudDownloadIcon, Delete02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

import { Button } from "@/components/ui/button";

export function ModelActions({
  isDownloaded,
  isSelected,
  modelId,
  onDelete,
  onSelect,
}: {
  isDownloading?: never;
  isDownloaded: boolean;
  isSelected: boolean;
  progress?: never;
  modelId: string;
  onDownload?: never;
  onCancel?: never;
  onDelete: (id: string) => void;
  onSelect: (id: string) => void;
}) {
  if (isDownloaded) {
    return (
      <div className="flex items-center gap-2">
        {!isSelected && (
          <Button onClick={() => onSelect(modelId)} size="sm" variant="outline">
            Select
          </Button>
        )}
        <Button
          className="text-muted-foreground"
          onClick={() => onDelete(modelId)}
          size="icon-sm"
          variant="ghost"
        >
          <HugeiconsIcon
            className="size-4 text-muted-foreground"
            icon={Delete02Icon}
            strokeWidth={2}
          />
        </Button>
      </div>
    );
  }

  return null;
}

export function ModelRightAction({
  isDownloading,
  isDownloaded,
  isSelected,
  modelId,
  onDelete,
  onDownload,
  onSelect,
}: {
  isDownloading: boolean;
  isDownloaded: boolean;
  isSelected: boolean;
  modelId: string;
  onDelete: (id: string) => void;
  onDownload: (id: string) => void;
  onSelect: (id: string) => void;
}) {
  if (isDownloading) {
    return null;
  }

  if (isDownloaded) {
    return (
      <ModelActions
        isDownloaded={isDownloaded}
        isSelected={isSelected}
        modelId={modelId}
        onDelete={onDelete}
        onSelect={onSelect}
      />
    );
  }

  return (
    <Button onClick={() => onDownload(modelId)} size="sm">
      <HugeiconsIcon
        className="size-4"
        icon={CloudDownloadIcon}
        strokeWidth={2}
      />
      Download
    </Button>
  );
}
