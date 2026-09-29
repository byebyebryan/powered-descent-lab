# Strict saved-coverage nominal direct execution V1

## Authorization and seal

Implement the reviewed
[completion contract](nominal_direct_execution_completion_contract.md), with
strict finite saved coverage and no continuation. Baseline is `abc8bca`.
The primary owns policy, admission/audit, provenance and acceptance; Luna owns
the settled additive core/control execution slice. No commit, push or deployment.

Before runtime edits or physical evaluation, resolve the four prescribed inputs
in `fixtures/research/nominal_direct_operational_fresh_inputs_v1.json`. Its
SHA-256 is `6df19fbd3ebb85d5a7f4a783398aaaa48e50c74a9de49454161b78c8e47658f8`.
Order is 685 m flat, 845 m flat, 845 m uphill +75 m, 845 m downhill -75 m.
The original release binary's read-only preflight returns supported for all
four, with no simulation created. This is input readiness, not flight success.
Its SHA-256 is
`0f34e28cc8a2e9d70f09854274bee183cbc7f47bc35d7d9609340137133a1400`.

All four retain the supported vehicle, frozen default generation/terminal
policies, 36 m flat shelves, 160 m domain padding, source at upright rest,
continuous uncut terrain, no obstacles/waypoints, 9.81 m/s2 gravity and 120/60 Hz
global clock. A before-work inventory pins all 1,395 files across the existing
nominal-flight, body-aware-terminal and contact-phase roots; compare exact
inventory/bytes after the new gates. Never alter those roots or old protocols.

## Settled runtime semantics

Keep `FlightProgramV1`, its structural/context validation, strict nominal
playback and selected-witness physical acceptance unchanged. The new mode is
opt-in, not a built-in controller or planner dependency. Invalid, Unsupported
and finite Unknown decisions execute no flight.

Let `C` be expected contact, `K = updates.len * control_interval_steps` be
checked saved coverage, and `H` be original planned end. Authorize transitions
only through `min(K,H)`. Process ordinary contact/progress/horizon handling on
that final transition before considering a new callback. If still flying,
deadline wins a `H=K` tie and both reached flags are retained. No update at K,
step beyond the bound, added four-tick reserve, idle fallback or last-command
repeat. The held command is legal only between its originally covered updates.

Use the ordinary plant transition once. Add incoming contact capture before
stable-contact normalization. A guard validates the initial state separately,
checks the actually selected/held command's actuator/fuel budget before every
transition, then audits each resulting state. Only airborne states use the
existing phase-aware AABB/heightfield clearance policy. Contact uses the incoming
core classification, strict body domain and unchanged predicate audit. Preserve
source support and descending target-pad corridor exceptions; do not impose a
new clearance/contact threshold.

Unpowered zero-fuel coast remains legal. Underfunded powered commands are safety
rejections before transition, not core crashes. Exact final fuel burn/contact
can be valid. Guard safety failure is sticky; numeric/clock/replay disagreement
is invalid evidence and cannot become completed-safe.

## API responsibility contract

Core exposes additive bounded execution/replay, neutral limits/stop/snapshot
DTOs and a guard with initial, pre-transition and post-transition stages.
Core knows no generator/evaluator policy. Existing APIs share unchanged
ordinary physics/mission handling and retain exact deterministic legacy fields.
The new runner retains a final snapshot even at an off-sample stop. Guard failure
and fallible command rejection preserve typed prefixes, not only an error.

Control derives limits from the complete validated program and consumes its
exact saved prefix through that runner, with controller identity
`flight_program_operational_v1`. Its separate bounded replay validates every
recorded action against the complete program, never deriving limits from the
shorter observed log. No edits to the default controller registry/trait contract.

The evaluator's fixed guard identity is `body_aware_operational_validity_v1`,
bound to immutable request/program/policy identities. A generic permissive core
guard is not admitted physical proof. The adapter independently verifies the
selected witness and exact conversion before motion, reports operational
outcome separately from `Match/Deviation/NotComparable`, and independently
replays actual authorized execution with a fresh fixed guard.

Trustworthy bounded runs must reproduce all deterministic actions, events,
samples, final/incoming state, stop cause and guard evidence. Invalid runs still
write the last valid finite prefix and first bad boundary/typed numeric detail;
failure writing does not require an unrecorded fault to recur. Replay diagnostic
failure stays `ExecutionInvalid/NotComparable`. Do not serialize NaN as a fake
finite value or turn a replay error into completed-safe.

Airborne prefixes retain `Flying/InProgress/Running` and no fabricated mission
events. A versioned bounded envelope records execution termination separately.
New create-only bundles persist request, policies, full generation/selection,
program/witness/admission, bounded execution, replay/audit, separate outcome/
nominal checks and observational compute evidence. A scoped operational HTML
summary links to a clearly labeled ordinary prefix trace; default reports and
old artifact schemas remain unchanged. Timings never affect deterministic IDs.

## Gates and stop rule

1. Artifact-free core/control tests cover covered odd/even steps, safe early/
   within-coverage late contact, K/H/horizon contact precedence, missing/off-clock
   command rejection, truthful partial and zero-action artifacts, incoming state,
   exact fuel burn/zero-fuel coast, forged/truncated/extra replay inputs and
   unchanged ordinary/strict behavior.
2. Adapter tests cover unchanged full witness admission, source-rest/first-step
   support, all three guard stages, airborne versus contact clearance, sticky
   safety failure, retained invalid evidence, separate nominal comparison,
   fixed-policy bindings, read-only preflight/create-only output and CLI modes.
3. Independently run the existing 24-control nominal regression on current
   source. The new operational gate records all those cases and all four sealed
   fresh cases, regenerates from physical inputs only, and preserves full
   selected-program/contact/fuel/legacy parity. Stored programs are comparisons,
   never generator seeds. Repeat the operational gate independently.
4. All 24 exposed controls must retain exact nominal proof and operational
   completed-safe Match. Fresh coverage targets 4/4 Direct, safe and Match.
   Record every Unknown/Unsupported/Invalid without replacement or tuning;
   any fresh coverage gap is explicitly separate from executor correctness.
5. Require unchanged source/protocol/physical-input pins during each measurement,
   exact archive inventory/bytes, and identical deterministic repeats excluding
   only observational compute timings. Preserve failed roots and case evidence.
6. Run focused tests, an artifact-free full workspace gate with Git metadata,
   strict all-target workspace Clippy, formatting and diff checks after source
   changes. Do not repeat full suites absent a changed source or unresolved risk.

Close with the implemented contract, inspectable evidence and precise verdict.
If a gate exposes a defect, fix that bounded defect; do not expand the family,
disturbance axes, source fitting or terminal policy. A finite generation gap is
Unknown, not physical infeasibility/waypoint demand. This pass does not recover
the contact-phase study's 48 late observations, establish operating tolerance,
prove swept safety, support arbitrary incoming waypoint states, optimize setup
cost or wire the default/direct-first waypoint planner.
