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

Current corpus tiers:

- `transfer_bot_lab_suite`
  - 45 runs
  - 3 payload tiers: `empty`, `half`, `full`
  - smoke route angles: `r-60`, `r-30`, `r00`, `r+30`, `r+60`
  - smoke seeds: `0`, `1`, `2`
  - intended as the fast transfer controller gate
- `transfer_route_angle_suite`
  - 99 runs
  - the same 3 payload tiers and smoke seeds
  - all 11 signed route angles from `r-80` through `r+80`
  - still fixed at `radius_tier = nominal`
  - intended as the route-shape diagnostic pack before adding radius tiers or
    full seeds

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

Transfer reports derive handoff review metrics from controller telemetry without
changing controller behavior:

- `transfer_final_phase`
- `transfer_terminal_handoff_time_s`
- `transfer_terminal_handoff_dx_m`
- `transfer_terminal_handoff_height_m`
- `transfer_terminal_handoff_speed_mps`
- `transfer_shape_curve_rmse_m`
- `transfer_shape_apex_error_m`
- `transfer_shape_projected_dx_abs_mean_m`
- `transfer_shape_projected_dx_abs_max_m`
- `transfer_shape_shortfall_ratio`
- `transfer_boost_burn_duration_s`
- `transfer_boost_burn_fuel_used_kg`
- `transfer_boost_burn_avg_throttle`
- `transfer_terminal_gate_mode`
- `transfer_terminal_gate_latest_safe_margin_s`
- `transfer_terminal_gate_required_accel_ratio`
- `transfer_corridor_mode`
- `transfer_corridor_min_margin_m`

These appear in `summary.json` per-run review records and in seed-row details
for transfer batch reports. The shape fields are Pylander-inspired diagnostics:
they freeze the first boost-window route to the target, compare the actual path
against a parabolic reference, and keep final touchdown as the scored goal.

The current staged controller uses the transfer diagnostics directly:

- boost continues until the ballistic target-y crossing is in-band instead of
  stopping on along-track speed alone
- uphill boost tilt is limited by route steepness and current touchdown
  clearance so source-slope climbs stay more vertical
- when the target-y crossing is reachable but the projected `dx` misses, boost
  steers by projected miss direction rather than current `dx` alone
- boost-quality and apex-target calculations use the first boost-window route
  anchor so the intended shape does not shrink as the vehicle approaches the
  target
- terminal handoff is gated by the terminal controller's read-only
  recoverability estimate rather than just route distance/height
- uphill boost samples a local source-to-target terrain corridor and caps tilt
  near vertical until the craft has clearance over the immediate ramp
- coast pre-aligns upright retrograde without commanding max tilt during
  ascent

The corridor guard is still route-local, not broad terrain avoidance. It is
intended to protect near-source uphill climbs from terrain collision without
turning transfer guidance into a full route planner.

## Current Checkpoint

Initial `transfer_route_angle_suite` baseline:

- generated at commit `b32eb2f` with `8` workers
- `57 / 99` successes
- `42` crashes
- `0` invalidations
- `4.05s` wall clock
- `33.22s` mean sim time
- `72.33s` max sim time

Route-shape read:

- all downhill and flat cells from `r-80` through `r00` solve across all
  payload tiers
- `empty/r+15` solves
- `half/full r+15` fail after reaching terminal handoff
- `empty/r+30` reaches terminal handoff but fails; `half/full r+30` crash
  before handoff
- `r+45` and `r+60` crash during boost across all payload tiers
- `r+80` reaches terminal handoff but fails across all payload tiers
- all failing route/payload cells failed all three smoke seeds, so the current
  signal is route-shape and payload dependent rather than seed-noise dependent

Handoff read:

- `75 / 99` runs reached terminal handoff
- `24 / 99` runs ended while still in boost

Latest transfer tuning checkpoint:

- generated locally after the recoverability-gated terminal handoff and local
  uphill corridor pass with `8` workers
- `transfer_bot_lab_suite`: `45 / 45` successes, `0` invalidations
- `transfer_route_angle_suite`: `90 / 99` successes, `9` crashes, `0`
  invalidations
- latest gate/corridor rerun kept those same outcomes:
  - `transfer_bot_lab_suite`: `45 / 45`, `45.21s` mean sim time, `62.77s` max
  - `transfer_route_angle_suite`: `90 / 99`, `42.20s` mean sim time, `62.77s`
    max
- all `r-80` through `r+60` cells solve across `empty`, `half`, and `full`
- only `r+80` remains failed across all payload tiers; those runs remain in
  boost with `transfer_terminal_gate_mode = pending` and
  `transfer_corridor_mode = active`, so this behaves more like near-cliff
  launch/corridor debt than terminal handoff debt

## Deferred Work

- route radius tiers
- full-seed transfer packs
- terminal climbing-arrival suite extension
- aggregate handoff quality thresholds in batch summaries
- waypoint/corridor planning above terminal guidance
