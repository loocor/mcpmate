export function formatTransportTag(transport: string): string {
  const parts = transport.split(/[_\s-]+/).filter(Boolean);

  if (!parts.length) {
    return transport;
  }

  return parts
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(" ");
}
