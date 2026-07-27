import type {
  ClientInfo,
  SurfaceReviewItem,
  SurfaceReviewOwner,
} from "./types";

const PROFILE_OWNER_TYPES = new Set([
  "standard_profile",
  "profile_server_exposure",
]);

const CLIENT_OWNER_TYPES = new Set([
  "consumer_direct_exposure",
  "consumer_server_exposure",
]);

export function getProfileReviewCount(
  items: SurfaceReviewItem[],
  profileId: string,
): number {
  return items.filter((item) =>
    item.owners.some(
      (owner) =>
        PROFILE_OWNER_TYPES.has(owner.owner_type) &&
        owner.owner_id === profileId,
    ),
  ).length;
}

export function getClientReviewCount(
  items: SurfaceReviewItem[],
  client: ClientInfo,
): number {
  return items.filter((item) =>
    item.owners.some(
      (owner) =>
        (CLIENT_OWNER_TYPES.has(owner.owner_type) &&
          owner.owner_id === client.identifier) ||
        (owner.owner_type === "custom_profile" &&
          owner.owner_id === client.custom_profile_id),
    ),
  ).length;
}

export function getSurfaceReviewDestination(
  item: SurfaceReviewItem,
  owner: SurfaceReviewOwner,
  clients: ClientInfo[] = [],
): string {
  const query = new URLSearchParams({
    review_item: item.review_item_id,
    ref_id: item.ref_id,
  });
  const ownerId = encodeURIComponent(owner.owner_id);

  if (PROFILE_OWNER_TYPES.has(owner.owner_type)) {
    return `/profiles/${ownerId}?${query}`;
  }
  const customProfileClient =
    owner.owner_type === "custom_profile"
      ? clients.find((client) => client.custom_profile_id === owner.owner_id)
      : undefined;
  if (CLIENT_OWNER_TYPES.has(owner.owner_type) || customProfileClient) {
    const record =
      typeof item.target_record === "object" && item.target_record !== null
        ? item.target_record
        : item.before_record;
    const source =
      typeof record === "object" &&
      record !== null &&
      "source" in record &&
      typeof record.source === "object" &&
      record.source !== null
        ? record.source
        : null;
    const serverId =
      source &&
      "serverId" in source &&
      typeof source.serverId === "string"
        ? source.serverId
        : null;
    const clientId = encodeURIComponent(
      customProfileClient?.identifier ?? owner.owner_id,
    );
    return serverId
      ? `/clients/${clientId}/direct/${encodeURIComponent(serverId)}?${query}`
      : `/clients/${clientId}?${query}`;
  }
  if (owner.owner_type === "custom_profile") {
    return `/clients?filter=needs_review`;
  }
  return `/clients/${encodeURIComponent(item.consumer_id)}?${query}`;
}
