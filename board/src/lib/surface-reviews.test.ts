import { describe, expect, test } from "bun:test";
import {
  getClientReviewCount,
  getProfileReviewCount,
  getSurfaceReviewDestination,
} from "./surface-reviews";
import type {
  ClientInfo,
  SurfaceReviewItem,
} from "./types";

const client = {
  identifier: "client-a",
  custom_profile_id: "profile-custom",
} as ClientInfo;

const item = {
  review_item_id: "review-a",
  ref_id: "ref-a",
  target_record: {
    format: "mcpmate.effective-capability/v1",
    source: {
      serverId: "server-a",
      kind: "tools",
      originKey: "analyze",
    },
    refId: "ref-a",
    externalKey: "server_a__analyze",
    definition: {
      kind: "tool",
      tool: {
        name: "analyze",
        inputSchema: { type: "object" },
      },
    },
  },
  owners: [
    { owner_type: "standard_profile", owner_id: "profile-a" },
    { owner_type: "custom_profile", owner_id: "profile-custom" },
    { owner_type: "consumer_direct_exposure", owner_id: "client-a" },
  ],
} as SurfaceReviewItem;

describe("surface review ownership", () => {
  test("counts profile and client ownership without double counting an item", () => {
    expect(getProfileReviewCount([item], "profile-a")).toBe(1);
    expect(getClientReviewCount([item], client)).toBe(1);
  });

  test("builds stable deep links for profile and consumer-owned reviews", () => {
    expect(
      getSurfaceReviewDestination(item, {
        owner_type: "standard_profile",
        owner_id: "profile-a",
      }),
    ).toBe("/profiles/profile-a?review_item=review-a&ref_id=ref-a");
    expect(
      getSurfaceReviewDestination(item, {
        owner_type: "consumer_direct_exposure",
        owner_id: "client-a",
      }),
    ).toBe(
      "/clients/client-a/direct/server-a?review_item=review-a&ref_id=ref-a",
    );
    expect(
      getSurfaceReviewDestination(
        item,
        {
          owner_type: "custom_profile",
          owner_id: "profile-custom",
        },
        [client],
      ),
    ).toBe(
      "/clients/client-a/direct/server-a?review_item=review-a&ref_id=ref-a",
    );
  });
});
