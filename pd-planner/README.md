# Maintained solver implementations

This concrete strategy package owns Policy 3 and opt-in ballistic execution.
It depends on core/runtime, pure `pd-plan` math and `pd-control` feedback. It has
no evaluation, report or collection-management dependency. Reusable platform
packages do not import it.

| Area | Ownership |
| --- | --- |
| `planner_flight` | Nominal realization, acquisition and fixed-program terrain audit |
| `local_clearing` | Actual plant progression and clearing realization/proof |
| `waypoint_v2` | Policy 3 session, corrections, handoff lifecycle and source proof |
| `ballistic_feedback` | Ballistic acquisition, recovery, correction, clearance and terminal composition |
| `configuration` | Separate versioned effective profiles and constructor flags |
| Diagnostic computations | Bounded counterfactual/capability queries, distinct from flown mission decisions |

`WaypointV2Session::start_configured` binds the original request and its effective
configuration. `BallisticOriginV1::from_source` constructs from the original
scenario and optional proven retained command prefix. `execute_configured`
accepts that opaque origin and a validated `BallisticFeedbackConfigV1`; callers
cannot supply a restored live state. Diagnostic helpers replay original commands
rather than restarting from saved snapshots.

The application supplies diagnostic recorders and owns file hashes, destinations,
create-only output, portable bundles, rich report adaptation and publication.
`pd-eval-cli` retains compatibility exports for existing Rust callers. No algorithm
copy remains there. The ordinary planner is Policy 3 with cap 6; cap-24 profiles,
v18 acquisition and v19 combined safety remain explicit experimental choices.

Existing hashed descriptors and flight records retain their original encoding.
`TerminalPdgResolvedConfigV1` exposes all coupled terminal adapter flags separately.
New ballistic captures record their effective profile in `configuration-v1.json`.
Configuration is fresh-constructor input; it contains no saved plan/live state.

See [ownership](../docs/architecture.md), [guidance](../docs/guidance.md),
[validation](../docs/development.md) and [preservation evidence](../docs/platform_restructure_results.md).
