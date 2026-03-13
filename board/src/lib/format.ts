// Human-readable formatter for potentially JSON-encoded strings.
// - Pretty prints objects/arrays/primitives with JSON indentation.
// - If input is a string that looks like JSON, attempts to parse once,
//   and if the result is still a JSON string, attempts a second parse.
// - Falls back to original string when parsing fails.
export function smartFormat(value: unknown): string {
  const pretty = (v: unknown) => {
    try {
      return JSON.stringify(v, null, 2);
    } catch {
      return String(v);
    }
  };

  const tryParse = (text: string): unknown | undefined => {
    try {
      return JSON.parse(text);
    } catch {
      return undefined;
    }
  };

  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
      const once = tryParse(trimmed);
      if (once !== undefined) {
        if (typeof once === "string") {
          const t = once.trim();
          if (t.startsWith("{") || t.startsWith("[")) {
            const twice = tryParse(t);
            if (twice !== undefined) return pretty(twice);
          }
        }
        return pretty(once);
      }
    }
    return trimmed;
  }

  return pretty(value);
}

