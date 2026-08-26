import { DotsThreeIcon } from "@phosphor-icons/react";
import { DropdownMenu } from "radix-ui";
import type { ReactNode } from "react";

export interface ActionMenuItem {
  label: string;
  icon?: ReactNode;
  onSelect: () => void;
  disabled?: boolean;
  destructive?: boolean;
}

interface ActionMenuProps {
  label: string;
  items: ActionMenuItem[];
}

export function ActionMenu({ label, items }: ActionMenuProps) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger className="icon-button" aria-label={label}>
        <DotsThreeIcon size={20} weight="bold" />
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className="action-menu" sideOffset={7} align="end">
          {items.map((item) => (
            <DropdownMenu.Item
              key={item.label}
              className={item.destructive ? "destructive" : undefined}
              disabled={item.disabled}
              onSelect={item.onSelect}
            >
              {item.icon}
              <span>{item.label}</span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
