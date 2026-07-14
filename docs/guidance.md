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
- batch schema `33` waypoint and terminal-recovery fields
- deterministic mission outcomes, handoff evidence, and landing summaries

Internal Rust types and module paths are not compatibility surfaces. They may
be reorganized to make ownership explicit as long as the persisted contracts
above remain unchanged.

## Maintained Gates

The guidance regression set is:

- terminal bot-lab and trajectory-error smoke packs
- `transfer_route_angle_radius_suite`
- paired waypoint turn and ordered smoke landing/contract packs
- paired full-seed nominal waypoint landing/contract packs
- paired all-radius waypoint landing/contract packs

The all-radius turn landing pack intentionally retains one final-recovery
frontier at `single_gentle_bend_v1/full/r-30/short/seed 02`. Its waypoint
contract passes, so it is not a waypoint-guidance failure.

## Experimental Boundary

Pathwise boost scoring, recoverability-weighted boost scoring, and the
no-terrain terminal alias remain reproducible diagnostics. They are not
maintained guidance modes and must not influence default controller behavior.
Their code and fixtures should remain isolated from the maintained path until a
new hypothesis justifies reopening them.

## Consolidation Rule

Guidance cleanup is behavior-preserving. Do not change thresholds, candidate
ordering, route geometry, or phase transition conditions while moving code.
Any behavioral defect discovered during consolidation should be documented and
handled in a separate controller change with fresh evaluation evidence.
