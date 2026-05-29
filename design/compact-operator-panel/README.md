# Tray Operator Panel

This folder preserves the research baseline, discussion summary, and visual
concepts for the MCPMate Tray Operator Panel design track.

Project item: `v0.5: Compact Operator Panel design research`
Working branch used for exploration and first implementation:
`feat/compact-operator-panel`
Date: 2026-05-28

## Artifacts

- `research.md` captures the evidence baseline from Board code inspection,
  Playwright observation, and community research.
- `tray-operator-panel-spec.md` is the final design spec after product,
  interaction, and desktop feasibility review.
- `images/direction-a-tray-rich-panel.png` explores a tray-style rich panel.
- `images/direction-b-compact-board-home.png` explores a compact Board home.
- `images/direction-c-hybrid-mini-panel.png` explores a hybrid mini panel with
  the existing Full Console behind it. This clarified the Full Console
  relationship, but the horizontal strip layout was not selected.

The generated image originals remain under:

`/Users/loocor/.codex/generated_images/019e6e11-ced5-7d72-bcda-3f74f723ba6e/`

## Discussion Summary

The starting question was whether MCPMate should implement Express and Expert
as two UI modes, and whether this had to wait for the future Capability Grid.

The final conclusion is:

1. Existing Board should remain the Full Board surface.
2. The feature should not be implemented as scattered application-mode
   conditionals across current Board pages.
3. The final direction is a Tray Operator Panel inspired by mini-mode and
   tray-rich-panel interactions.
4. The panel should focus on daily operational state and high-frequency actions:
   core status, active profile, client attention, server health, import, quick
   profile switching, token/traffic trend, and recent errors.
5. Deep configuration should stay in Full Board: client config parsing,
   transport rules, server inspector, raw capability JSON, profile capability
   bulk editing, runtime reset/install, API docs, and developer settings.
6. Capability Grid remains a later architecture track. The panel can summarize
   capability load and profile effect, but should not pretend to enforce a new
   capability model.

## Final Direction

The final direction is:

`Tray Operator Panel + Full Board`

This removes the Express/Expert or Full/Compact product-mode framing. The
existing Board remains the Full Board for complete configuration, management,
inspection, and troubleshooting. The new surface is a tray-triggered operator
panel for everyday status, attention, and high-frequency actions.

The vertical direction from Direction A remains the visual basis because
MCPMate's core objects are naturally list-shaped: Core, Profiles, Clients,
Servers, Traffic, and Attention. The final panel is not embedded in Dashboard,
does not use a right-side detail region, and does not branch the current Board
with an application mode.

## Evidence Notes

The Board was observed against a temporary local backend data directory:

`MCPMATE_DATA_DIR=/private/tmp/mcpmate-compact-panel-data`

Playwright screenshots were captured in `/private/tmp` during research. Those
screenshots were temporary evidence and are not part of this preserved artifact
set.
