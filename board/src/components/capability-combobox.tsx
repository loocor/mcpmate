import * as React from "react";
import type { ReactNode } from "react";
import { Button } from "./ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "./ui/command";
import { ChevronDown } from "lucide-react";
import { cn } from "../lib/utils";

export type CapabilityKind = "tool" | "prompt" | "resource" | "template";

export interface CapabilityRecordLike {}

export interface CapabilityComboboxProps<T extends CapabilityRecordLike = CapabilityRecordLike> {
  kind: CapabilityKind;
  items: T[];
  value?: string;
  onChange: (key: string, item?: T) => void;
  loading?: boolean;
  error?: string | null;
  container?: HTMLElement | null;
  placeholder?: string;
  triggerClassName?: string;
  triggerLabelClassName?: string;
  renderTriggerLeading?: (item: T) => ReactNode;
  renderTriggerTrailing?: (item: T) => ReactNode;
  renderItemLeading?: (item: T) => ReactNode;
  renderItemTrailing?: (item: T) => ReactNode;
  menuBleed?: boolean;
  menuGroupClassName?: string;
  menuItemClassName?: string;
  getKey: (item: T) => string;
  getLabel: (item: T) => string;
  getDescription?: (item: T) => string | undefined;
}

export function CapabilityCombobox<T extends CapabilityRecordLike>(props: CapabilityComboboxProps<T>) {
  const {
    items,
    value,
    onChange,
    loading,
    error,
    container,
    placeholder = "Search...",
    triggerClassName,
    triggerLabelClassName,
    renderTriggerLeading,
    renderTriggerTrailing,
    renderItemLeading,
    renderItemTrailing,
    menuBleed = true,
    menuGroupClassName,
    menuItemClassName,
    getKey,
    getLabel,
    getDescription,
  } = props;

  const [open, setOpen] = React.useState(false);
  const triggerRef = React.useRef<HTMLButtonElement | null>(null);
  const [menuWidth, setMenuWidth] = React.useState<number | undefined>(undefined);

  React.useEffect(() => {
    try {
      const el = document.documentElement;
      if (open) el.setAttribute("data-inspector-combobox-open", "true");
      else el.removeAttribute("data-inspector-combobox-open");
    } catch {
      /* noop */
    }
  }, [open]);

  React.useEffect(() => {
    if (!open || !triggerRef.current) return;
    const el = triggerRef.current;
    const update = () => setMenuWidth(el.offsetWidth);
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    window.addEventListener("resize", update);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", update);
    };
  }, [open]);

  const selectedItem = React.useMemo(() => {
    if (!value) return undefined;
    return items.find((it) => getKey(it) === value);
  }, [items, value, getKey]);

  const selectedLabel = selectedItem ? getLabel(selectedItem) : "";
  const selectedTrailing = selectedItem ? renderTriggerTrailing?.(selectedItem) : null;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          ref={triggerRef}
          variant="outline"
          className={cn("w-full justify-between", triggerClassName)}
          type="button"
          aria-expanded={open}
        >
          <span className="flex min-w-0 flex-1 items-center gap-3 text-left">
            {selectedItem && renderTriggerLeading
              ? renderTriggerLeading(selectedItem)
              : null}
            <span
              className={cn(
                "truncate",
                triggerLabelClassName ?? "font-normal",
              )}
            >
              {selectedLabel || placeholder}
            </span>
          </span>
          {selectedTrailing ? (
            <span className="ml-2 flex shrink-0 items-center">{selectedTrailing}</span>
          ) : null}
          <span className="ml-2 flex items-center gap-1 text-slate-500">
            {loading ? (
              <span className="inline-flex h-4 w-4 animate-spin rounded-full border-2 border-slate-400 border-t-transparent" />
            ) : null}
            <ChevronDown className="h-4 w-4 opacity-60" aria-hidden="true" />
          </span>
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        sideOffset={6}
        className={cn(
          "p-0",
          menuBleed &&
          "max-w-none overflow-hidden rounded-lg w-[var(--radix-popover-trigger-width)]",
        )}
        container={container}
        style={menuWidth ? { width: `${menuWidth}px` } : undefined}
      >
        <Command className={menuBleed ? "rounded-none" : undefined}>
          <CommandInput placeholder={placeholder} />
          <CommandList
            className={
              menuBleed
                ? "max-h-[300px] overflow-y-auto overflow-x-hidden p-0 [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden"
                : undefined
            }
          >
            {error ? (
              <CommandEmpty>
                <span className="text-red-500 text-xs">{error}</span>
              </CommandEmpty>
            ) : (
              <CommandEmpty>No results found.</CommandEmpty>
            )}
            <CommandGroup
              className={cn(
                menuGroupClassName,
                menuBleed &&
                "p-0 [&_[cmdk-group-items]]:m-0 [&_[cmdk-group-items]]:w-full [&_[cmdk-group-items]]:p-0",
              )}
            >
              {items.map((entry, index) => {
                const key = getKey(entry) || `index:${index}`;
                const label = getLabel(entry) || key;
                const desc = getDescription?.(entry);
                const trailing = renderItemTrailing?.(entry);
                return (
                  <CommandItem
                    key={key}
                    value={key}
                    className={cn(
                      "group py-2.5",
                      menuBleed ? "w-full rounded-none px-0" : "px-4",
                      menuItemClassName,
                    )}
                    onSelect={(v) => {
                      onChange(v, entry);
                      setOpen(false);
                    }}
                  >
                    <div
                      className={cn(
                        "flex w-full min-w-0 items-center gap-3",
                        menuBleed && "px-4",
                      )}
                    >
                      {renderItemLeading ? renderItemLeading(entry) : null}
                      <div className="flex min-w-0 flex-1 flex-col">
                        <span className="truncate font-medium text-slate-900 dark:text-slate-100 group-hover:text-accent-foreground group-aria-selected:text-accent-foreground">
                          {label}
                        </span>
                        {desc ? (
                          <span className="truncate text-xs text-slate-500 dark:text-slate-400" title={desc}>
                            {desc}
                          </span>
                        ) : null}
                      </div>
                      {trailing ? (
                        <div className="ml-auto flex shrink-0 items-center">
                          {trailing}
                        </div>
                      ) : null}
                    </div>
                  </CommandItem>
                );
              })}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

export default CapabilityCombobox;
