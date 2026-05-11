import { Badge } from "../../../components/ui/badge";
import type { ServerEntryData } from "../../../lib/types";

interface ServerEntryTransportBadgeProps {
  entry: ServerEntryData;
}

export function ServerEntryTransportBadge({
  entry,
}: ServerEntryTransportBadgeProps) {
  const isSkipped = entry.import_status === "skipped";
  let variant: "destructive" | "warning" | "success" = "success";

  if (isSkipped) {
    variant = "destructive";
  } else if (entry.transport === "unclassified") {
    variant = "warning";
  }

  return (
    <Badge variant={variant} className="text-[10px] px-1.5 py-0">
      {entry.transport}
    </Badge>
  );
}
