import { Label, ListBox, Select } from "@heroui/react";
import "./themed-picker.css";
// React Aria retains keyboard/touch semantics; its popover stays in our themed
// overlay root instead of opening Android's native, independently themed dialog.
export function ThemedPicker({
  label,
  value,
  onChange,
  options,
  className = "",
  visibleLabel = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  className?: string;
  visibleLabel?: boolean;
}) {
  const key = (value: string) => value || "__fontes_empty__";
  return (
    <Select
      aria-label={visibleLabel ? undefined : label}
      className={`themed-picker ${className}`}
      value={key(value)}
      onChange={(v) => {
        if (typeof v === "string") onChange(v === "__fontes_empty__" ? "" : v);
      }}
    >
      {visibleLabel && <Label>{label}</Label>}
      <Select.Trigger>
        <Select.Value />
        <Select.Indicator />
      </Select.Trigger>
      <Select.Popover
        className="themed-picker-popover"
        UNSTABLE_portalContainer={
          document.getElementById("overlay-root") || undefined
        }
      >
        <ListBox aria-label={`${label} options`}>
          {options.map((o) => (
            <ListBox.Item
              id={key(o.value)}
              key={key(o.value)}
              textValue={o.label}
            >
              <span>{o.label}</span>
              <ListBox.ItemIndicator />
            </ListBox.Item>
          ))}
        </ListBox>
      </Select.Popover>
    </Select>
  );
}
