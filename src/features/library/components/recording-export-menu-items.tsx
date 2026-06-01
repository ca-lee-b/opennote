import type { ComponentType } from "react";
import type { ContextMenuSeparator } from "@/components/ui/context-menu";
import type {
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";

type MenuItemProps = React.ComponentProps<typeof DropdownMenuItem>;
type MenuItemComponent = ComponentType<MenuItemProps>;

export type RecordingMenuSeparatorComponent = ComponentType<
  React.ComponentProps<typeof DropdownMenuSeparator> &
    React.ComponentProps<typeof ContextMenuSeparator>
>;

interface RecordingExportMenuItemsProps {
  Item: MenuItemComponent;
}

export function RecordingExportMenuItems({
  Item,
}: RecordingExportMenuItemsProps) {
  return (
    <>
      <Item disabled>Plain Text (.txt)</Item>
      <Item disabled>Markdown (.md)</Item>
      <Item disabled>PDF (.pdf)</Item>
    </>
  );
}
