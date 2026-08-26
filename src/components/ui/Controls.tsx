import { Switch, ToggleGroup } from "radix-ui";
import type { ReactNode } from "react";

export interface ToggleOption<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

interface ToggleControlProps<T extends string> {
  value: T;
  onValueChange: (value: T) => void;
  label: string;
  options: ToggleOption<T>[];
}

export function ToggleControl<T extends string>({
  value,
  onValueChange,
  label,
  options,
}: ToggleControlProps<T>) {
  return (
    <ToggleGroup.Root
      className="toggle-control"
      type="single"
      value={value}
      aria-label={label}
      onValueChange={(next) => {
        if (next) onValueChange(next as T);
      }}
    >
      {options.map((option) => (
        <ToggleGroup.Item key={option.value} value={option.value}>
          {option.icon}
          <span>{option.label}</span>
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}

interface SwitchControlProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label: string;
  description?: string;
}

export function SwitchControl({
  checked,
  onCheckedChange,
  label,
  description,
}: SwitchControlProps) {
  return (
    <label className="switch-control">
      <span>
        <strong>{label}</strong>
        {description ? <small>{description}</small> : null}
      </span>
      <Switch.Root checked={checked} onCheckedChange={onCheckedChange}>
        <Switch.Thumb className="switch-thumb" />
      </Switch.Root>
    </label>
  );
}
