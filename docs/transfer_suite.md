# Transfer Suite Design

This document defines the first `pd-lab` transfer-guidance matrix. It is a
separate Phase 3 family, not another terminal `condition_set`.

## Scope

The first transfer slice tests point-to-point travel from a source pad to a
target pad, then safe touchdown on the target. It should reuse terminal guidance
for the final landing phase instead of mutating the terminal suite into a
route-planning benchmark.

The v1 contract is:

- launch from the source pad
- boost toward the route
- coast or hand off when inside the terminal gate
- land on the designated target pad

The maintained score is still the final `landing_on_pad` goal. Boost-cutoff and
handoff quality should show up first as controller telemetry and report
diagnostics, not as a separate `timed_checkpoint` pass/fail goal.

## Geometry

The matrix uses a one-sided signed route arc around the target:

- target pad fixed at `(0, 0)`
- source pad resolved from:
  - `source_x = -radius * cos(route_angle)`
  - `source_y = -radius * sin(route_angle)`
- positive `route_angle` means the target is uphill from the source
- negative `route_angle` means the target is downhill from the source
- left/right route duplication is intentionally omitted for v1

The initial family is `signed_route_arc_transfer_v1`:

- `radius_nominal_m = 800`
- smoke route angles: `r-60`, `r-30`, `r00`, `r+30`, `r+60`
- full route angles: `r-80`, `r-60`, `r-45`, `r-30`, `r-15`, `r00`, `r+15`,
  `r+30`, `r+45`, `r+60`, `r+80`
- `radius_tier = nominal`

Radius is intentionally recorded as a selector but not varied yet. Travel
distance changes the transfer trajectory shape, so it should become a real axis
after the one-radius route family is useful.

## Terrain

Transfer v1 may generate simple monotonic source-to-target terrain for physical
miss/crash containment. That terrain is not a terrain-avoidance objective.

Generated terrain must:

- contain the full source-to-target route domain
- keep flat plateaus under both pads
- stay monotonic between the source and target elevations
- avoid introducing local obstacles that require route-level replanning

Terrain avoidance remains parked above terminal guidance. Future work should
validate approach corridors, waypoints, or collision-course warnings before
terminal handoff rather than teaching the terminal controller arbitrary terrain
navigation.

## Pack And Selector Shape

Transfer packs use a first-class pack entry:

```json
{
  "id": "transfer_guidance_clean_empty",
  "transfer_matrix": "signed_route_arc_transfer_v1",
  "base_scenario": "../scenarios/flat_terminal_descent.json",
  "lanes": [{ "id": "current", "controller": "transfer_pdg" }],
  "seed_tier": "smoke",
  "vehicle_variant": "empty",
  "expectation_tier": "reference"
}
```

Resolved transfer runs use transfer-specific selector fields:

- `mission = transfer_guidance`
- `route_family = signed_route_arc_transfer_v1`
- `route_angle = r-60` style signed labels
- `radius_tier = nominal`
- `vehicle_variant = empty | half | full`
- `lane = current`

For report compatibility, transfer records also populate the existing matrix
slots:

- `arrival_family = route_family`
- `arc_point = route_angle`
- `velocity_band = radius_tier`

Reports should label those levels as `route` and `radius` for transfer missions,
while terminal reports continue to label them as `arc` and `band`.

## Controller Direction

`transfer_pdg_v1` is intentionally staged:

- takeoff/bootstrap from the source pad
- boost toward the target route
- optional coast before terminal gate
- terminal handoff to `terminal_pdg_v1`

This keeps the terminal controller contract narrow: given a reachable,
terrain-valid approach corridor, handle braking, lateral cleanup, descent rate,
attitude, and touchdown.

## Deferred Work

- route radius tiers
- route-angle full packs
- terminal climbing-arrival suite extension
- handoff quality metrics in batch summaries
- waypoint/corridor planning above terminal guidance
