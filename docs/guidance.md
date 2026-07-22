# Guidance Architecture

This document is the stable boundary between terminal guidance, direct
transfer, waypoint guidance, and future waypoint planning. It describes
ownership and compatibility rather than controller tuning.

## Ownership

Terminal guidance owns braking, lateral cleanup, descent-rate control,
attitude, and touchdown after a terminal entry has been accepted. It may apply
local terrain-clearance constraints while selecting a terminal candidate, but
it does not plan a route or interpret waypoint profiles.

Direct transfer owns source-pad clearance, boost, coast, route-local corridor
constraints, and the decision to hand the craft to terminal guidance. It may
reacquire source clearance after a premature direct terminal entry. It does not
evaluate waypoint contracts.

Waypoint guidance owns the currently active preplanned leg, capture-window
lifecycle, handoff contract, continuation viability, and final-waypoint entry
into terminal guidance. It is terrain-blind: waypoint placement and arrival
envelopes must already encode a terrain-valid route.

Waypoint planning is upstream of guidance. It will own terrain-valid waypoint
placement, leg ordering, and arrival-envelope construction. Planning must not
move terminal or waypoint contract logic into route-label-specific controller
branches.

## Lifecycle Contract

- terminal guidance starts only after terminal spatial ownership and entry
  policy pass
- direct transfer progresses through source clearance, boost, coast, and
  terminal ownership
- waypoint guidance keeps the active leg until its window contract passes or
  the waypoint-plane deadline resolves it
- a recoverable final waypoint enters terminal guidance directly
- a final waypoint without recoverability evidence retains the transfer
  fallback instead of assuming terminal safety
- route-contract reports stop at contract completion; landing reports continue
  to physical touchdown

## Compatibility Surface

The following are persisted or consumed across crates and must remain stable
during behavior-preserving refactors:

- `ControllerSpec` JSON shape and built-in controller aliases
- canonical controller IDs
- terminal and transfer configuration field names and defaults
- controller phase strings
- telemetry metric keys and marker IDs
- batch schema `34` waypoint and terminal-recovery fields
- deterministic mission outcomes, handoff evidence, and landing summaries

Internal Rust types and module paths are not compatibility surfaces. They may
be reorganized to make ownership explicit as long as the persisted contracts
above remain unchanged.

## Implementation Layout

The current `pd-control` layout follows those ownership boundaries:

- `controllers.rs` owns the controller registry plus the legacy baseline and
  staged controllers
- `guidance.rs` owns shared state-target acceleration and command allocation
- `terminal/mod.rs` owns the terminal update loop and guidance-plan lifecycle;
  `terminal/config.rs` preserves the public serialized configuration,
  `terminal/state.rs` owns internal command and entry DTOs,
  `terminal/planning.rs` owns deterministic candidate ordering and ballistic
  helpers, and `terminal/terrain.rs` isolates local candidate clearance
- `transfer/mod.rs` owns the direct-transfer and waypoint update loop;
  `transfer/config.rs` preserves the public serialized configuration,
  `transfer/state.rs` owns lifecycle state and internal DTOs, and
  `transfer/math.rs` owns shared ballistic and command-conversion helpers
- `transfer/telemetry.rs` owns transfer and waypoint metric emission plus
  waypoint handoff-marker assembly; it receives already-computed guidance
  products and does not recompute control decisions
- `transfer/waypoint.rs` owns pure waypoint geometry, capture prediction, and
  handoff kinematics
- `transfer/experimental.rs` owns the frozen boost-scoring mode gates and
  weights retained for diagnostic reproducibility
- `transfer/tests.rs` owns the transfer and waypoint controller tests without
  changing their access to module-private fixtures

The evaluator follows the same separation. Persisted batch/report DTOs live in
`pd-eval/src/model.rs`; pack validation and expansion live in `resolution.rs`;
execution, artifact/cache support, comparison, and review derivation live in
their named modules. The batch report shell delegates overview, diagnostics,
review-tree, and comparison rendering to `pd-eval/src/report/` modules. Public
crate exports and persisted schema paths remain unchanged.

This split is internal. Public controller exports still resolve through
`pd-control`, and persisted controller, phase, telemetry, and artifact contracts
remain unchanged.

## Maintained Gates

The guidance regression set is:

- terminal bot-lab and trajectory-error smoke packs
- `transfer_route_angle_radius_suite`
- paired waypoint turn and ordered smoke landing/contract packs
- paired full-seed nominal waypoint landing/contract packs
- paired all-radius waypoint landing/contract packs

The all-radius turn contract and landing packs both pass `405 / 405`. Bounded
final authority-recovery search closes the former
`single_gentle_bend_v1/full/r-30/short/seed 02` landing residual without
changing waypoint contracts or adding route/profile branches.

## Experimental Boundary

Pathwise boost scoring, recoverability-weighted boost scoring, and the
no-terrain terminal alias remain reproducible diagnostics. They are not
maintained guidance modes and must not influence default controller behavior.
Their code is isolated from the maintained path, and their comparison packs use
the `diagnostic` expectation tier plus `experimental` tags. Reopen them only
when a new hypothesis justifies another experiment.

## Consolidation Rule

The 2026-07-13 guidance consolidation was behavior-preserving: it did not change
thresholds, candidate ordering, route geometry, phase transition conditions,
schema `33`, or artifact contracts. Future structural cleanup follows the same
rule. Behavioral changes require a separate controller change with fresh
evaluation evidence.
