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

- radius tiers: `short = 400m`, `nominal = 800m`, `long = 1200m`
- omitted `radius_tiers` selectors default to `nominal`
- smoke route angles: `r-60`, `r-30`, `r00`, `r+30`, `r+60`
- full route angles: `r-80`, `r-60`, `r-45`, `r-30`, `r-15`, `r00`, `r+15`,
  `r+30`, `r+45`, `r+60`, `r+80`

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
  - intended as the nominal-radius route-shape diagnostic pack before full
    seeds
- `transfer_radius_tier_suite`
  - 135 runs
  - the same 3 payload tiers and smoke seeds
  - smoke route angles only
  - all 3 radius tiers: `short`, `nominal`, `long`
  - intended as the fast distance-sensitivity gate
- `transfer_route_angle_radius_suite`
  - 297 runs
  - the same 3 payload tiers and smoke seeds
  - all 11 signed route angles and all 3 radius tiers
  - intended as the wide smoke distance/route-shape diagnostic
- `transfer_route_angle_radius_full_solved`
  - 1080 runs
  - the same 3 payload tiers and all 12 full seeds
  - all 3 radius tiers
  - route angles from `r-80` through `r+60`, excluding the known `r+80`
    frontier
  - intended as the full-seed reliability gate for the solved direct-transfer
    region
- `transfer_route_angle_radius_frontier_full`
  - 108 runs
  - the same 3 payload tiers and all 12 full seeds
  - all 3 radius tiers
  - `r+80` only
  - intended as an explicit frontier watch, not as the direct-transfer
    controller pass/fail gate
- `transfer_waypoint_rpos80_smoke`
  - 27 runs
  - `r+80` only, all 3 payload tiers, all 3 radius tiers, smoke seeds
  - injects the `single_dogleg_v1` waypoint profile
  - intended as the fast waypoint-guidance probe for the known direct-transfer
    frontier
- `transfer_waypoint_rpos80_full`
  - 108 runs
  - `r+80` only, all 3 payload tiers, all 3 radius tiers, all 12 transfer seeds
  - injects the `single_dogleg_v1` waypoint profile
  - intended as the full-seed waypoint-guidance frontier probe

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

## Waypoint Guidance Direction

Waypoint work should start with guidance semantics, not waypoint setup. For the
first waypoint slice, assume a higher-level planner has already chosen the
waypoint list and any terrain-valid spatial or energy envelopes.

The waypoint controller's job is to follow the currently active leg and keep the
vehicle in a useful state for the next leg. The controller should reason about
the previous anchor, the waypoint that ends the active leg, and the next target
that defines the outbound leg. A waypoint is a pass-through handoff surface, not
a place to stop, hover, or land. Full stop at each waypoint would collapse
waypoint guidance back into repeated transfer plus terminal landing, which is
not the intended product behavior.

Waypoint arrival should therefore be an envelope:

- position: pass within a configured capture radius, crossing band, or maximum
  cross-track miss at the waypoint plane
- progress: only capture after crossing the waypoint plane along the active leg,
  not by skimming the radius from the wrong side
- outbound state: velocity should have positive progress along the next leg and
  a bounded outbound heading error
- energy: speed and vertical rate should stay within bounds supplied by the
  route plan, so the next leg is not immediately impossible

Terrain remains outside waypoint guidance for v1. The planner may place
waypoints and envelopes so terrain clearance is valid, but the guidance
controller should not query terrain to decide avoidance behavior or branch on
terrain fixture labels. Terrain crashes and clearance margins can be reported as
evidence that a waypoint plan is bad; they should not become hidden controller
modes.

The maintained score should remain final landing on the target pad. Waypoint
arrival failures are guidance diagnostics that explain why final landing failed
or why a route is unsafe. Useful first report fields are active waypoint index,
active leg index, closest waypoint distance, cross-track miss at the waypoint
plane, waypoint capture time, outbound heading error, outbound speed, vertical
rate at capture, and final-leg handoff quality.

Implementation checkpoint:

- `TransferRouteSpec` now carries preplanned waypoints.
- `single_dogleg_v1` is the first matrix waypoint profile. It is intentionally
  narrow: the profile exists for the `r+80` frontier and inserts one dogleg
  waypoint before final descent to the target.
- `transfer_waypoint_pdg_v1` is the first terrain-blind waypoint controller
  variant. It tracks the active leg, blocks terminal handoff until the waypoint
  is captured, and then lets the existing terminal handoff logic solve the
  final target leg.
- V1 capture status is deliberately spatial: capture means reaching the
  configured radius or crossing the waypoint plane inside the cross-track band.
  Outbound heading, outbound progress, speed, and vertical rate are still
  reported as next-leg viability diagnostics, not hard gates.
- Waypoint-profile transfer runs use a `130s` sim cap. This keeps the first
  pass focused on route feasibility while leaving landing-time tightening as
  follow-up controller work.
- Waypoint misses are route-contract warnings in reports, not mission failures
  by themselves. The maintained score remains final landing, but capture
  warnings keep route quality visible.

Transfer reports derive handoff review metrics from controller telemetry without
changing controller behavior:

- `transfer_final_phase`
- `transfer_terminal_entry_kind`
- `transfer_terminal_handoff_time_s`
- `transfer_terminal_handoff_dx_m`
- `transfer_terminal_handoff_height_m`
- `transfer_terminal_handoff_speed_mps`
- `transfer_terminal_handoff_gate_mode`
- `transfer_terminal_handoff_projected_dx_m`
- `transfer_terminal_handoff_impact_angle_deg`
- `transfer_terminal_handoff_boost_quality`
- `transfer_terminal_handoff_latest_safe_margin_s`
- `transfer_terminal_handoff_required_accel_ratio`
- `transfer_boost_projected_dx_m`
- `transfer_boost_impact_angle_deg`
- `transfer_boost_apex_over_target_m`
- `transfer_boost_quality`
- `transfer_boost_settled_quality`
- `transfer_boost_settled_projected_dx_m`
- `transfer_boost_cutoff_time_s`
- `transfer_boost_cutoff_projected_dx_m`
- `transfer_boost_cutoff_impact_angle_deg`
- `transfer_boost_cutoff_apex_over_target_m`
- `transfer_boost_cutoff_quality`
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

Batch reports now render two transfer-specific triage sections before the
Review Tree:

- `Transfer Handoff Triage` is the primary controller-tuning view. It groups
  current-lane runs by condition, route, radius, and vehicle, then sorts by
  failed/frontier status, low handoff height, high handoff speed, wide handoff
  projected `dx`, and wide boost-cutoff projected `dx`.
- `Transfer Shape Triage` remains a visual-shape diagnostic sorted by worst
  successful shape RMSE. It should explain "landed but ugly" transfer paths,
  not replace the handoff gate/cutoff read.

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
- once the target-y crossing is reachable, boost steering and candidate scoring
  penalize both shortfall and overshoot around the projected landing `dx`
- uphill coast may enter terminal control just before crossing target height,
  but only when the crossing is imminent, the projected terminal miss is
  centered, and the latest-safe gate is already close

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

- generated locally after the source-clearance hold and transfer-scoped terminal
  gate horizon pass with `8` workers
- `transfer_bot_lab_suite`: `45 / 45` successes, `0` invalidations, `60.44s`
  mean sim time, `76.60s` max sim time
- `transfer_route_angle_suite`: `90 / 99` successes, `9` crashes, `0`
  invalidations, `56.24s` mean sim time, `76.60s` max sim time
- latest schema-23 handoff diagnostics report the same outcomes:
  - `transfer_bot_lab_suite`: all `45` runs enter through handoff; `42` use
    `latest_safe` gates and `3` use the narrow pre-target `pending` gate
  - `transfer_route_angle_suite`: `81` handoff entries, `9` direct entries,
    and `9` records without terminal-entry diagnostics; the no-entry records
    are the `r+80` frontier failures
- the overshoot-aware boost pass reduced representative uphill handoff
  projected misses without adding route-specific branches:
  - `full/r+30/seed0`: projected handoff `dx` moved from about `-136m` to
    `-110m`
  - `full/r+45/seed0`: projected handoff `dx` moved from about `-171m` to
    `-86m`
- all `r-80` through `r+60` cells solve across `empty`, `half`, and `full`
- only `r+80` remains failed across all payload tiers. At the nominal `800m`
  radius it is a near-vertical route: about `139m` horizontal for `788m` of
  climb.
- `r+80` should stay in `transfer_route_angle_suite`, but it is classified as
  the scored `near_vertical_transfer_route` frontier. It is waypoint/corridor
  debt, not terminal guidance debt, and it must not be invalidated.

Radius-tier expansion checkpoint:

- latest clean-cache refresh was generated from commit `673954f` with `8`
  workers
- `transfer_radius_tier_suite`: `135 / 135` successes, `0` invalidations,
  `59.58s` mean sim time, `79.35s` max sim time
- `transfer_route_angle_radius_suite`: `270 / 297` successes, `27` crashes,
  `0` invalidations, `55.39s` mean sim time, `83.24s` max sim time
- `transfer_radius_tier_suite` keeps the smoke route set fully solved across
  `short`, `nominal`, and `long`, so distance variation alone does not break
  the currently gated transfer slice
- `transfer_route_angle_radius_suite` preserves the known `r+80` frontier
  pattern across all payload and radius tiers: `27` crashes, all annotated as
  `near_vertical_transfer_route`
- the previous non-frontier `full/r-80` short/long radius failures are resolved
  by the focused handoff pass; the wide matrix now has no non-frontier transfer
  failures

Full-seed transfer coverage checkpoint:

- generated locally after the full-seed pack split with `8` workers
- `transfer_route_angle_radius_full_solved`: `1080 / 1080` successes, `0`
  invalidations, `59.24s` mean sim time, `83.24s` max sim time
- `transfer_route_angle_radius_frontier_full`: `0 / 108` successes, `108`
  crashes, `0` invalidations, `16.83s` mean sim time, `21.70s` max sim time
- the solved-region full-seed pack covers every non-`r+80` route/radius/payload
  cell and confirms that no smoke-only success is hiding a seed outlier
- the frontier pack keeps the known `r+80` near-vertical route failure visible
  without polluting the solved-region reliability gate

Waypoint `r+80` checkpoint:

- generated locally after adding `single_dogleg_v1` and
  `transfer_waypoint_pdg_v1` with `8` workers and `--no-reuse`
- `transfer_waypoint_rpos80_smoke`: `27 / 27` successes, `0` timeouts, `0`
  invalidations, `94.56s` mean sim time, `120.59s` max sim time, `15`
  captured waypoint runs, and `12` contract warnings
- `transfer_waypoint_rpos80_full`: `108 / 108` successes, `0` timeouts, `0`
  invalidations, `94.56s` mean sim time, `120.59s` max sim time, `60`
  captured waypoint runs, and `48` contract warnings
- all waypoint `r+80` payload/radius/seed cases now land, including the
  previous `full/long/r+80` timeout cluster
- remaining debt is waypoint capture quality and outbound-leg shaping rather
  than basic waypoint route feasibility or final landing reliability

Focused `full/r-80` radius triage:

- the failures are deterministic across smoke seeds because these cases have no
  seed perturbation after route resolution
- all three radius tiers enter direct terminal capture from the source pad; there
  is no boost/cutoff phase for these steep downhill routes
- `short` fails while still near the elevated source pad: final position is about
  `47.6m` from target, vertical speed is about `-6.9 m/s`, and hull clearance
  first goes negative against the source-pad/plateau terrain
- `nominal` succeeds with the same direct-terminal pattern and lands on target
- `long` reaches the target laterally but impacts at about `-14.1 m/s`, so the
  miss is terminal vertical-speed control after a very high direct descent, not
  source-terrain clearance
- focused pack `transfer_rneg80_radius_focus_suite` was added to keep this
  evidence cheap and explicit:
  - baseline before the controller fix: `3 / 9` successes
  - after source-clearance hold only: `6 / 9` successes; all `short` runs solved
  - after the transfer-scoped terminal gate horizon tune: `9 / 9` successes
- the implemented controller slice avoided route-label branching:
  - short-radius source-pad crashes are covered by a route-local source
    clearance hold before direct terminal capture
  - long-radius high direct descents are covered by transfer-scoped terminal
    gate horizon tuning, leaving standalone terminal defaults unchanged

Pathwise boost-scoring experiment:

- added gated `transfer_pdg_pathwise` as a Pylander-lite candidate scorer that
  keeps the current Rust candidate grid but scores simulated boost samples
  across the candidate horizon
- the first compare packs intentionally keep legacy endpoint scoring as
  `baseline` and pathwise scoring as `current`:
  - `transfer_bot_lab_pathwise_compare`
  - `transfer_route_angle_pathwise_compare`
- the first broad check is not promotable:
  - bot-lab compare: both lanes landed `45 / 45`, but pathwise worsened mean
    shape RMSE and full-payload touchdown offset
  - route-angle compare: both lanes landed `90 / 99`, with only the existing
    `r+80` frontier crashes, but pathwise worsened aggregate shape for `empty`
    and `half` and only helped scattered full-payload downhill cells
- keep `transfer_pdg` on legacy endpoint scoring by default; use the pathwise
  alias only for follow-up experiments until the pathwise objective can improve
  shape without degrading solved cells

Recoverability boost-scoring experiment:

- added gated `transfer_pdg_recoverability` as a safer follow-up to pathwise
  scoring. It preserves the legacy endpoint objective and only uses terminal
  gate recoverability as a weak tie-breaker.
- compare packs:
  - `transfer_bot_lab_recoverability_compare`
  - `transfer_route_angle_recoverability_compare`
- broad check after reducing recoverability to a tie-breaker:
  - bot-lab compare: both lanes landed `45 / 45`; successful-run mean shape
    RMSE was effectively neutral at `93.42m` baseline vs `93.45m` current, and
    mean touchdown offset improved slightly from `0.403m` to `0.400m`
  - route-angle compare: both lanes landed `90 / 99`, with only the existing
    `r+80` frontier crashes; successful-run mean shape RMSE stayed neutral at
    `94.48m` baseline vs `94.60m` current, and mean touchdown offset improved
    slightly from `0.486m` to `0.484m`
- this is a useful diagnostic/tie-breaker experiment but not a default-promotion
  candidate yet. The original strong recoverability weighting preserved
  landings but badly distorted uphill shapes, which reinforces that recoverable
  terminal handoff cannot replace the boost shape objective.

## Deferred Work

- terminal climbing-arrival suite extension
- aggregate handoff quality thresholds in batch summaries
- waypoint/corridor planning above terminal guidance
