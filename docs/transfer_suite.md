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

Transfer seeds are deterministic geometry perturbations, not controller
randomness. Seed `0` keeps the nominal radius. Smoke seeds cover nominal and
`+/-3%` route radius. Full seeds cover the nominal radius plus progressively
wider radius offsets up to about `9%`. The selector `radius_tier` still names
the nominal tier (`short`, `nominal`, or `long`); resolved metadata records the
actual `route_radius_m`, `route_radius_nominal_m`, `route_radius_pct`, and
`route_radius_jitter_m`.

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
  - route angles from `r-80` through `r+60`; this historical partition still
    provides the broad full-seed direct-transfer reliability gate
- `transfer_route_angle_radius_frontier_full`
  - 108 runs
  - the same 3 payload tiers and all 12 full seeds
  - all 3 radius tiers
  - `r+80` only
  - retained as the focused steep-uphill regression despite its historical
    `frontier` name; the current controller lands all `108` runs
- `transfer_waypoint_rpos80_smoke`
  - 27 runs
  - `r+80` only, all 3 payload tiers, all 3 radius tiers, smoke seeds
  - injects the `single_dogleg_v1` waypoint profile
  - retained as the fast hairpin/stress probe for waypoint guidance
- `transfer_waypoint_rpos80_full`
  - 108 runs
  - `r+80` only, all 3 payload tiers, all 3 radius tiers, all 12 transfer seeds
  - injects the `single_dogleg_v1` waypoint profile
  - retained as the full-seed hairpin/stress probe
- `transfer_waypoint_contract_rpos80_smoke`
  - 27 runs
  - same geometry as `transfer_waypoint_rpos80_smoke`
  - uses `evaluation_goal = waypoint_handoff` to score the first waypoint
    contract directly
- `transfer_waypoint_contract_rpos80_full`
  - 108 runs
  - same geometry as `transfer_waypoint_rpos80_full`
  - full-seed contract probe for waypoint controller tuning
- `transfer_waypoint_bend_rpos80_smoke`
  - 27 runs
  - same axes as the dogleg smoke pack
  - injects the smoother `single_bend_v1` waypoint profile
  - retained as the focused smooth `r+80` waypoint regression
- `transfer_waypoint_bend_rpos80_full`
  - 108 runs
  - full-seed reliability gate for the smoother waypoint workbench
- `transfer_waypoint_bend_contract_rpos80_smoke`
  - 27 runs
  - scores the smoother waypoint profile at the first handoff
- `transfer_waypoint_bend_contract_rpos80_full`
  - 108 runs
  - full-seed contract probe for the smoother waypoint profile
- `transfer_waypoint_turn_smoke`
  - 81 landing runs over `r-30 | r00 | r+30`, nominal radius, all 3 payload
    tiers, all 3 smoke seeds, and three balanced turn profiles
  - the maintained broad waypoint-guidance workbench
- `transfer_waypoint_turn_contract_smoke`
  - the same `81` selector cells as `transfer_waypoint_turn_smoke`
  - scores `pass_through_v1` at the first waypoint handoff instead of allowing
    final-landing recovery to hide route-quality errors
- `transfer_waypoint_sequence_smoke`
  - 54 final-landing runs over two ordered profiles, three representative route
    angles, all payload tiers, nominal radius, and smoke seeds
  - the first multi-waypoint physical-outcome baseline
- `transfer_waypoint_sequence_contract_smoke`
  - the same 54 selector cells and geometry as the sequence landing pack
  - scores every waypoint in order with `evaluation_goal = waypoint_sequence`

Resolved transfer runs use transfer-specific selector fields:

- `mission = transfer_guidance`
- `route_family = signed_route_arc_transfer_v1`
- `route_angle = r-60` style signed labels
- `radius_tier = nominal`
- `resolved_seed = 0` style seed labels
- `vehicle_variant = empty | half | full`
- `waypoint_profile` selects `single_gentle_bend_v1`,
  `single_medium_bend_v1`, or `single_sharp_bend_v1` for the balanced turn
  corpus; the sequence corpus uses `double_bend_v1 | late_bend_v1`
- `waypoint_handoff_envelope = pass_through_v1` for the balanced turn corpus
- `waypoint_handoff_envelope = sequence_pass_through_v1` for the ordered
  sequence corpus
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
- energy: total speed and any optional vertical-rate bounds supplied by the
  route plan should keep the next leg usable

Terrain remains outside waypoint guidance for v1. The planner may place
waypoints and envelopes so terrain clearance is valid, but the guidance
controller should not query terrain to decide avoidance behavior or branch on
terrain fixture labels. Terrain crashes and clearance margins can be reported as
evidence that a waypoint plan is bad; they should not become hidden controller
modes.

The maintained score for transfer reliability remains final landing on the
target pad. Waypoint arrival failures are guidance diagnostics that explain why
final landing failed or why a route is unsafe. The separate waypoint handoff
probe packs intentionally score only the selected waypoint contract so
controller tuning can target pass-through quality before final-landing recovery
hides the problem. Ordered sequence probes extend that rule across the complete
preplanned route: every intermediate contract must pass in order, and the first
failure terminates the probe. Useful report fields are active waypoint index,
active leg index, closest waypoint distance, cross-track miss at the waypoint
plane, waypoint capture time, outbound heading error, outbound speed, vertical
rate at capture, route passed/total, first failed index, and final-leg handoff
quality.

Implementation checkpoint:

- `TransferRouteSpec` now carries preplanned waypoints.
- `single_dogleg_v1` is the first matrix waypoint profile. It is intentionally
  narrow: the profile exists for the `r+80` frontier and inserts one dogleg
  waypoint before final descent to the target. It is now treated as a stress
  route, not the primary waypoint-guidance workbench.
- `single_bend_v1` is the first smoother waypoint profile. It places one
  pass-through waypoint at 55% of the source-to-target route plus a 20%
  route-radius source-side lateral offset, producing a roughly 44 degree
  nominal turn instead of the roughly 143 degree dogleg hairpin.
- Its distance capture radius stays tight at 10% of route radius, but its
  plane-crossing cross-track band is wider than the dogleg profile because this
  is a pass-through waypoint, not a precision stop.
- The balanced turn profiles keep progress and spatial tolerance fixed. Their
  initial route-normal offsets are `10% | 20% | 30%`, then the planner fixture
  lifts each waypoint vertically as needed to clear local terrain by at least
  `20% | 25% | 30%` of route radius. At nominal-radius `r00`, this produces
  `160m | 200m | 240m` terrain clearance and turns of roughly
  `43.9deg | 53.5deg | 62.3deg`.
- `single_straight_v1` was removed because its zero-offset capture volume
  intersected the monotonic route terrain. Pack-resolution tests now require
  every maintained waypoint to meet its planner floor and leave more vertical
  clearance than its cross-track allowance plus vehicle touchdown offset.
- `pass_through_v1` is an explicit route-relative handoff envelope: maximum
  outbound heading error `0.35rad`, minimum outbound progress `8m/s`, maximum
  outbound cross speed `20m/s`, and total speed from `10m/s` through `130m/s`.
  It intentionally leaves vertical speed unbounded so the first balanced
  corpus does not encode world-frame climb/descent policy into every turn.
- `double_bend_v1` places two waypoints at nominal route progress
  `0.33 | 0.67`, each with a `0.20R` source-side offset and terrain-clearance
  floor. The resulting turns are about `31deg | 31deg`; waypoint speed caps are
  `55m/s | 65m/s`.
- `late_bend_v1` uses the same nominal progress with offsets `0.12R | 0.26R`
  and clearance floors `0.15R | 0.26R`. The first leg stays nearly straight
  (`0-7deg`) before a `56-60deg` second turn; speed caps are `45m/s | 65m/s`.
- `sequence_pass_through_v1` uses the same `0.35rad` heading, `8m/s` progress,
  `20m/s` cross-speed, and `10m/s` minimum-speed bounds as `pass_through_v1`,
  but preserves each waypoint's profile-specific maximum speed.
- Sequence corpus resolution requires exactly two increasing waypoints,
  non-overlapping capture radii, terrain clearance beyond cross-track allowance
  plus vehicle offset, a terrain-valid sampled centerline outside pad
  footprints, expected turn ranges and speed caps, and paired landing/contract
  selector identity.
- `TransferWaypointSpec::assess_handoff` is the single source of truth for
  spatial triggering and outbound-envelope classification in `pd-core`,
  `pd-control`, and `pd-eval`.
- `transfer_waypoint_pdg_v1` is the first terrain-blind waypoint controller
  variant. While a waypoint is active, it keeps the leg under powered guidance,
  blocks direct-transfer coast/terminal handoff, and guides to the fixed
  waypoint endpoint with a target velocity aligned to the outbound leg. After
  capture, the existing direct-transfer and terminal logic solve the final leg.
- Fixed acceptance geometry and moving steering geometry are separate. The
  state-target solve always uses the configured waypoint endpoint; the active-leg
  lookahead contributes only a bounded L1-style path correction that fades near
  the handoff and may use at most 15% of current thrust authority.
- Candidate target speeds come from the configured outbound envelope, current
  outbound progress, and transfer cruise speed. Candidate horizons come from
  remaining leg distance and those speeds, not from the mission timeout. The
  controller prefers the shortest authority-feasible candidate and retains that
  per-leg plan until it expires or becomes dynamically infeasible.
- The waypoint solve reuses the same state-target acceleration and thrust/tilt
  allocation primitives as terminal guidance. Telemetry exposes endpoint and
  steering coordinates, target velocity, time to go, required acceleration
  ratio, feasibility, path-correction magnitude, and replan count.
- Active waypoint guidance does not query terrain. Source-pad clearance remains
  a separate launch guard; waypoint placement and terrain-valid arrival
  envelopes remain planner responsibilities.
- V1 capture status is deliberately spatial: capture means reaching the
  configured radius or crossing the waypoint plane inside the cross-track band.
  The stricter waypoint contract is reported separately: spatial misses are
  split from captures whose configured outbound heading, progress, cross-speed,
  total-speed, or optional vertical-speed bounds are out of envelope.
- `evaluation_goal = waypoint_handoff` is the first waypoint contract probe. It
  evaluates at controller observation boundaries, stops at capture-radius entry
  or waypoint-plane crossing, and scores the selected waypoint's spatial and
  outbound envelope directly. This keeps paired landing/contract status aligned
  with what the controller could actually observe.
- `evaluation_goal = waypoint_sequence` owns ordered progress in `pd-core`.
  Intermediate contract passes advance the active evaluation index without
  ending the run; the first failed contract ends with `failed_checkpoint`; the
  final pass ends with `checkpoint_satisfied`. Run summaries preserve passed,
  total, and first-failed index.
- Controller route transitions emit one `waypoint/handoff` marker. Batch review
  preserves the ordered marker history and synthesizes the terminal handoff
  from the authoritative final sample because checkpoint evaluation ends before
  another controller update can occur.
- Waypoint-profile transfer runs use a `130s` sim cap. This keeps the first
  pass focused on route feasibility while leaving landing-time tightening as
  follow-up controller work.
- Waypoint misses and outbound-out-of-envelope captures are route-contract
  warnings in reports, not mission failures by themselves. The maintained
  score remains final landing, but capture/contract warnings keep route quality
  visible.
- Waypoint turn-feasibility telemetry now reports remaining distance to the
  waypoint plane, estimated time to plane, required turn distance, shaping-start
  distance, and turn margin. Earlier local target blending, boost-score
  tie-breakers, and active-waypoint coast previews did not create a corrective
  arrival objective and have been removed. The retained state-target mechanism
  is route-frame and envelope driven; it does not branch on profile or route
  labels.
- Waypoint-profile report rows now include the profile, resolved handoff
  envelope, turn angle, and outbound cross speed. Multi-profile review trees add
  a profile level before route; single-profile reports retain their compact
  route-first hierarchy.
- Multi-waypoint batch reports add a collapsed route summary and one aggregate
  row per expected waypoint. Existing scalar waypoint fields remain aliases for
  waypoint zero, so single-waypoint report behavior is unchanged.

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
- `transfer_terminal_post_handoff_apex_gain_m`
- `transfer_terminal_post_handoff_time_to_apex_s`
- `transfer_terminal_post_handoff_apex_dx_abs_m`
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

Terminal guidance telemetry separates the fresh selector result from an active
retained plan with `guidance.candidate_burn_time_s`, `guidance.plan_active`,
`guidance.plan_arrival_time_s`, and `guidance.plan_replan_count`.

Batch reports render transfer-specific triage sections before the Review Tree:

- `Transfer Handoff Triage` is the primary controller-tuning view. It groups
  current-lane runs by condition, route, radius, and vehicle, then sorts by
  failed/frontier status and post-handoff climb before handoff height, speed,
  projected `dx`, and boost-cutoff projected `dx`. The climb cell includes
  time-to-apex and apex lateral offset; worst-seed selection uses the same
  signal.
- `Waypoint Handoff Triage` appears for waypoint packs and keeps profile plus
  envelope provenance beside spatial status, outbound heading/progress/cross
  speed, and the worst seed.
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
- while that steep corridor is tilt-limited, targetward lateral speed above
  `3m/s` selects bounded opposite-tilt braking candidates. This is a generic
  geometry/velocity rule, not an `r+80` branch.
- coast pre-aligns upright retrograde without commanding max tilt during
  ascent
- once the target-y crossing is reachable, boost steering and candidate scoring
  penalize both shortfall and overshoot around the projected landing `dx`
- uphill coast may enter terminal control just before crossing target height,
  but only when the crossing is imminent, the projected terminal miss is
  centered, and the latest-safe gate is already close
- an active waypoint stays under powered state-target guidance; generic
  boost/coast/terminal staging resumes only after spatial capture
- after waypoint capture, long-horizon terminal entries retain an absolute
  arrival time instead of selecting a fresh time-to-go every update. Short
  candidates remain receding; an ascending short candidate can still enter the
  retained plan if it reaches the long-capture ceiling before apex.
- the retained horizon counts down against simulation time, targets the upright
  touchdown-center height, and releases permanently to the existing terminal
  recovery once projected touchdown is inside the safe footprint, lateral
  speed is out of the high-energy regime, and the latest-safe braking boundary
  has been reached
- retained terminal plans are enabled only for waypoint transfer. Standalone
  terminal and direct-transfer entries preserve their existing receding-horizon
  behavior.

The corridor guard is still route-local, not broad terrain avoidance. It is
intended to protect near-source uphill climbs from terrain collision without
turning transfer guidance into a full route planner.

## Current Checkpoint

Current direct-transfer checkpoint, refreshed on 2026-07-10 with `8` workers
and `--no-reuse`:

- `transfer_route_angle_radius_suite`: `297 / 297` successes, `0` crashes, and
  `0` invalidations across every route angle, radius, payload, and smoke seed;
  wall clock is `45.99s`
- `transfer_route_angle_radius_frontier_full`: `108 / 108` successes and `0`
  invalidations across the full-seed `r+80` partition
- the focused uphill-corridor brake closes the former direct `r+80` failure
  without regressing the wider matrix
- `near_vertical_transfer_route` remains useful as a stress annotation, but it
  no longer describes a failing direct-transfer region

Current balanced waypoint-turn checkpoint, refreshed on 2026-07-10:

- `transfer_waypoint_turn_contract_smoke`: `81 / 81` handoff successes, `0`
  failures, `21.43s` mean sim time, and `32.13s` max sim time
- `transfer_waypoint_turn_smoke`: `81 / 81` final-landing successes; `r-30`,
  `r00`, and `r+30` each land `27 / 27`; mean sim time is `52.65s` and max sim
  time is `70.87s`
- the state-target controller therefore closes the maintained pass-through
  guidance contract across all profiles, route orientations, payloads, and
  smoke seeds; planner-side terrain-clearance floors keep those routes valid
  without terrain queries or fixture branches in guidance
- the contract and landing packs complete in `2.07s` and `10.35s` wall clock
  with `8` workers. Mean landing-pack controller-update compute is `161.5us`,
  versus `156.7us` before horizon retention, and remains well below the `1ms`
  budget
- across empty-payload `r00/r+30`, mean post-handoff climb improves by `69%`,
  mean sim time by `32%`, and mean fuel use by `29%`. Candidate density, not
  plan retention, now limits the remaining gentle/medium `r+30` apex height.
- direct-transfer regression remains `297 / 297`, confirming that the waypoint
  mechanism did not alter the non-waypoint controller path

Initial ordered waypoint-sequence baseline, refreshed on 2026-07-10 with
`12` workers and `--no-reuse`:

- `transfer_waypoint_sequence_smoke`: `49 / 54` final landings and `5` crashes,
  but only `2 / 54` runs satisfy both route contracts
- `transfer_waypoint_sequence_contract_smoke`: `2 / 54` ordered successes and
  `52` failed checkpoints; passed-handoff distribution is `0:22 | 1:30 | 2:2`
- the paired packs identify the same two complete routes, both
  `double_bend_v1/r+30/full` seeds. This confirms final landing is not a proxy
  for route-following quality.
- every recorded sequence failure reaches the waypoint spatial envelope. The
  dominant violation is outbound heading, sometimes combined with excessive
  outbound cross speed; two late-bend second handoffs also lack outbound
  progress.
- the baseline does not justify route/profile-specific tuning. Next work should
  diagnose why fixed-endpoint state targeting reaches the point without shaping
  velocity far enough toward the next leg, then test a general leg-transition
  correction while preserving `81 / 81`, `81 / 81`, and `297 / 297` gates.
- route-radius expansion remains a later evidence axis after the nominal-radius
  two-waypoint mechanism is credible. Generalized terrain avoidance remains out
  of scope; waypoint planning still owns terrain-valid placement.

Legacy waypoint regression checkpoint:

- dogleg smoke landing `21 / 27`; dogleg smoke contract `0 / 27`
- smooth-bend smoke landing `16 / 27`; smooth-bend smoke contract `27 / 27`
- these `r+80` packs remain diagnostic stress routes rather than acceptance
  gates. Their older full-seed snapshots predate the state-target controller and
  are not current evidence.

The following checkpoints are retained as historical tuning evidence.

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

Historical transfer tuning checkpoint:

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

Historical radius-tier expansion checkpoint:

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

Historical full-seed transfer coverage checkpoint:

- generated locally after the full-seed pack split with `8` workers
- `transfer_route_angle_radius_full_solved`: `1080 / 1080` successes, `0`
  invalidations, `59.24s` mean sim time, `83.24s` max sim time
- `transfer_route_angle_radius_frontier_full`: `0 / 108` successes, `108`
  crashes, `0` invalidations, `16.83s` mean sim time, `21.70s` max sim time
- the solved-region full-seed pack covers every non-`r+80` route/radius/payload
  cell and confirms that no smoke-only success is hiding a seed outlier
- the frontier pack keeps the known `r+80` near-vertical route failure visible
  without polluting the solved-region reliability gate

Legacy dogleg waypoint `r+80` checkpoint:

- generated locally after adding `single_dogleg_v1`, `transfer_waypoint_pdg_v1`,
  contract diagnostics, and the first outbound-velocity blend pass with `8`
  workers and `--no-reuse`
- `transfer_waypoint_rpos80_smoke`: `27 / 27` successes, `0` timeouts, `0`
  invalidations, `103.86s` mean sim time, `124.99s` max sim time, `15` spatial
  waypoint misses, `12` outbound-unviable captures, and `0` contract-passing
  handoffs
- `transfer_waypoint_rpos80_full`: `108 / 108` successes, `0` timeouts, `0`
  invalidations, `103.63s` mean sim time, `126.54s` max sim time, `56` spatial
  waypoint misses, `52` outbound-unviable captures, and `0` contract-passing
  handoffs
- `transfer_waypoint_contract_rpos80_smoke`: `0 / 27` contract successes, `0`
  invalidations, `22.37s` mean sim time, `30.72s` max sim time, `15` spatial
  waypoint misses, and `12` outbound-unviable captures
- `transfer_waypoint_contract_rpos80_full`: `0 / 108` contract successes, `0`
  invalidations, `22.31s` mean sim time, `31.19s` max sim time, `56` spatial
  waypoint misses, and `52` outbound-unviable captures
- all waypoint `r+80` payload/radius/seed cases now land, including the
  previous `full/long/r+80` timeout cluster
- remaining debt is pass-through route quality: the controller can still land
  after the dogleg, but it does not yet produce viable waypoint handoffs

Focused `full/r-80` radius triage:

- earlier artifacts were deterministic across smoke seeds because transfer
  seeds did not perturb geometry after route resolution; current transfer seeds
  perturb route radius within the selected radius tier
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
