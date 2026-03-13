export function maskHeaderValue(key: string, value: string): string {
  const k = key.toLowerCase();
  const sensitive = new Set([
    'authorization',
    'proxy-authorization',
    'x-api-key',
    'api-key',
    'apikey',
    'cookie',
    'set-cookie',
    'x-auth-token',
    'authentication',
  ]);
  if (!sensitive.has(k)) return value;
  if (!value) return '***REDACTED***';
  if (value.length <= 12) return '***REDACTED***';
  return `${value.slice(0, 6)}***${value.slice(-2)}`;
}

export function sanitizeRecord(
  record?: Record<string, string> | null,
): Record<string, string> | undefined {
  if (!record) return undefined;
  const next: Record<string, string> = {};
  for (const [rawKey, rawValue] of Object.entries(record)) {
    const key = (rawKey || '').trim();
    if (!key) continue;
    const value = rawValue == null ? '' : String(rawValue).trim();
    next[key] = value;
  }
  return Object.keys(next).length ? next : undefined;
}

