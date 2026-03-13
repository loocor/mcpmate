import * as React from "react";
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

export type CapabilityKind = "tool" | "prompt" | "resource" | "template";

export interface CapabilityRecordLike {
  [key: string]: unknown;
}

export interface CapabilityComboboxProps<T extends CapabilityRecordLike = CapabilityRecordLike> {
  kind: CapabilityKind;
  items: T[];
  value?: string;
  onChange: (key: string, item?: T) => void;
  loading?: boolean;
  error?: string | null;
  container?: HTMLElement | null;
  placeholder?: string;
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

  const selectedLabel = React.useMemo(() => {
    if (!value) return "";
    const found = items.find((it) => getKey(it) === value);
    return found ? getLabel(found) : "";
  }, [items, value, getKey, getLabel]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          ref={triggerRef}
          variant="outline"
          className="w-full justify-between"
          type="button"
          aria-expanded={open}
        >
          <span className="truncate text-left font-normal">
            {selectedLabel || placeholder}
          </span>
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
        className="p-0"
        container={container}
        style={{ width: menuWidth ? `${menuWidth}px` : undefined }}
      >
        <Command>
          <CommandInput placeholder={placeholder} />
          <CommandList>
            {error ? (
              <CommandEmpty>
                <span className="text-red-500 text-xs">{error}</span>
              </CommandEmpty>
            ) : (
              <CommandEmpty>No results found.</CommandEmpty>
            )}
            <CommandGroup>
              {items.map((entry, index) => {
                const key = getKey(entry) || `index:${index}`;
                const label = getLabel(entry) || key;
                const desc = getDescription?.(entry);
                return (
                  <CommandItem
                    key={key}
                    value={key}
                    className="py-2.5 px-4 group"
                    onSelect={(v) => {
                      onChange(v, entry);
                      setOpen(false);
                    }}
                  >
                    <div className="flex min-w-0 flex-col">
                      <span className="truncate font-medium text-slate-900 dark:text-slate-100 group-hover:text-accent-foreground group-[aria-selected=true]:text-accent-foreground">
                        {label}
                      </span>
                      {desc ? (
                        <span className="truncate text-xs text-slate-500 dark:text-slate-400" title={desc}>
                          {desc}
                        </span>
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
