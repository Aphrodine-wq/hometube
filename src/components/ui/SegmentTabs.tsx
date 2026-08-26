import { Tabs } from "radix-ui";
import type { ReactNode } from "react";

interface SegmentTab {
  value: string;
  label: string;
  count?: number;
}

interface SegmentTabsProps {
  value: string;
  onValueChange: (value: string) => void;
  label: string;
  tabs: SegmentTab[];
  children: ReactNode;
}

export function SegmentTabs({ value, onValueChange, label, tabs, children }: SegmentTabsProps) {
  return (
    <Tabs.Root value={value} onValueChange={onValueChange}>
      <Tabs.List className="segment-tabs" aria-label={label}>
        {tabs.map((tab) => (
          <Tabs.Trigger key={tab.value} value={tab.value}>
            {tab.label}
            {tab.count !== undefined ? <span>{tab.count}</span> : null}
          </Tabs.Trigger>
        ))}
      </Tabs.List>
      {children}
    </Tabs.Root>
  );
}

export const SegmentTabContent = Tabs.Content;
