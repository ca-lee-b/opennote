import { Delete02Icon, Edit02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ComponentType } from "react";
import type { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import type { RecordingMenuSeparatorComponent } from "./recording-export-menu-items";

type MenuItemProps = React.ComponentProps<typeof DropdownMenuItem>;
type MenuItemComponent = ComponentType<MenuItemProps>;

interface RecordingMoreActionsMenuItemsProps {
  Item: MenuItemComponent;
  onDelete: () => void;
  onRename: () => void;
  Separator: RecordingMenuSeparatorComponent;
  showRename?: boolean;
}

export function RecordingMoreActionsMenuItems({
  Item,
  Separator,
  onRename,
  onDelete,
  showRename = true,
}: RecordingMoreActionsMenuItemsProps) {
  return (
    <>
      {showRename ? (
        <>
          <Item onClick={onRename}>
            <HugeiconsIcon
              className="mr-2 size-4"
              icon={Edit02Icon}
              strokeWidth={2}
            />
            Rename
          </Item>
          <Separator />
        </>
      ) : null}
      <Item onClick={onDelete} variant="destructive">
        <HugeiconsIcon
          className="mr-2 size-4"
          icon={Delete02Icon}
          strokeWidth={2}
        />
        Delete
      </Item>
    </>
  );
}
