import { Share04Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  ContextMenuItem,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
} from "@/components/ui/context-menu";

export function RecordingSidebarExportContextSubmenu() {
  return (
    <ContextMenuSub>
      <ContextMenuSubTrigger>
        <HugeiconsIcon
          className="mr-2 size-4"
          icon={Share04Icon}
          strokeWidth={2}
        />
        Export
      </ContextMenuSubTrigger>
      <ContextMenuSubContent>
        <ContextMenuItem disabled>Plain Text (.txt)</ContextMenuItem>
        <ContextMenuItem disabled>Markdown (.md)</ContextMenuItem>
        <ContextMenuItem disabled>PDF (.pdf)</ContextMenuItem>
      </ContextMenuSubContent>
    </ContextMenuSub>
  );
}
