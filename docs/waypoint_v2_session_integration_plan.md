# Planner V2 session and CLI integration plan

This approved larger pass delivers a callable V2 flight session, an opt-in
`pd-cli` adapter, saved-flight replay, full-pack parity checks and the existing
rich reports. It goes beyond an internal refactor, while preserving the flight
policy already accepted in the lab. Execution was approved on 2026-10-04 through
`$worker-goal-loop`, including the bounded validation allowance and local
checkpoint commits below. Default promotion, pushes and service changes remain
excluded.

The intended result is usable synchronous lab integration on the current
vehicle, Earth gravity and 120 Hz physics / 60 Hz command setup. It is not an
evaluator-free game library, per-tick controller or hard real-time planner.

## Starting evidence

The repository is clean at `19aa430`, with five local commits ahead of
`origin/main`. The [reliability results](planner_v2_reliability_results.md)
record two accepted 44-case captures: 36/36 mandatory landings each, comprising
11 clear direct and 25 initially blocked corrected flights. Eight diagnostics
remain separate: two landed, four stopped before departure and two were
unsupported. Integrity is 44/44 and supported source replay is 42/42.

The existing `FlightLoop` in `pd-eval/src/waypoint_v2.rs` already owns the live
plant, original deadline, accumulated commands and correction count. Its `run`
method currently performs all planning/execution cycles before returning.
Nominal generation and admission helpers span evaluator research modules.
`pd-cli` does not currently depend on `pd-eval`; `pd-eval` does not depend on
`pd-cli`. These are current code observations, not missing trajectory features.

## Chosen scope and architecture

Keep the working backend in `pd-eval` for this pass. Add a default-off
`planner-v2` Cargo feature to `pd-cli`, with an optional `pd-eval` dependency.
This is an explicit lab-adapter tradeoff: two callers can use one implementation
without copying the backend or migrating the entire research tree. It does not
create a `pd-control` dependency on `pd-eval` or add a V2 `ControllerSpec`.
Default `pd-cli` builds and ordinary `run`, `replay` and `report` behavior stay
unchanged. A clean standalone runtime crate remains a separate future decision.

The session retains ownership of the full live simulation. Its unit of progress
is one executed flight piece, not one physics tick. A direct piece ends at
verified target contact. A corrected piece includes the nominal approach to
the chosen intervention and the correction through actual handoff. Planning
queries and continuation certificates use private clones; they are not flown
by the active vehicle. The consumer calls again after the actual handoff.

Keep terrain-blind nominal selection, fixed-program terrain audit, local
clearing and actual-state replanning unchanged. No landing suffix or literal
terrain-feature far edge becomes a requirement. No new terrain corpus, vehicle,
gravity, numerical policy, search grid or contact model is introduced.

## Checkpoint 1 Baseline and contract review

Before implementation, freeze and record the selected native capture, complete
input identity, raw evidence inventory, published site and relevant legacy
report inventory. Define the API, CLI outcomes and comparison exclusions below,
then review them against actual dependencies. If the optional lab-adapter
dependency is rejected, return the architecture choice to the user rather than
silently expanding into a shared-crate migration.

The baseline is historical evidence, not a new run. Source commits and binary
hashes will differ after implementation, so cross-version preservation compares
physical behavior while retaining each version's distinct provenance.

## Checkpoint 2 Stepwise session

Extract initialization, one-cycle advancement and finalization from the
existing loop. Suggested operations are `start`, `advance_piece` and `finish`;
final Rust names may differ. The session owns its request/context and live
state. An advance returns an executed handoff or a terminal outcome carrying
the existing typed stop. Physical and mission outcomes remain distinct from
planning status.

Terminal progress is not final acceptance: the caller must finalize the
session, and a failed final replay overrides an earlier landed advance. Do
not print or publish verified success before that finalization passes.

- Preflight rejection creates no live simulation. Supported finite stops keep
  their actual partial evidence, including a single initial sample if no
  command was executed.
- Advancing again after landing, stop or finalization cannot issue commands,
  change state, append duplicate cycles or increment correction counts.
- Count a correction only after actual handoff and the unchanged proofs pass.
  Preserve the final nominal attempt after the last allowed correction.
- Never refill fuel, reset time/deadline or reconstruct a stepped plant from
  `SimulationStateSnapshotV1::to_simulation_state`; that view is query-only.
- Preserve pending-cycle and partial-flight evidence on implementation errors.
  Finalization performs the existing final source replay outside advancement;
  admission, witness, prefix and certificate proofs remain in their existing
  logical positions.

Keep `run_waypoint_v2_flight` as a compatibility wrapper driving this session
to completion. Existing evaluator APIs, policy 1/2 compatibility, policy 3
selection and serialized flight/summary schemas remain unchanged. Progress
traces, if useful, are separate sidecar data rather than new numerical inputs.

The checkpoint gate is focused parity and lifecycle tests: direct flight,
one/repeated correction, pause between handoffs, unsupported preflight,
pre-departure stop, correction/deadline boundary and no advance after terminal
outcome. Verify commands, actual entry/handoff states, incoming contact, held
command, clock, fuel and correction count. Do not substitute final landing
count for state/command parity.

## Checkpoint 3 Optional CLI adapter and saved replay

Add feature-gated `pd-cli waypoint-v2-flight` and `waypoint-v2-replay` commands.
Flight accepts ordinary scenario JSON, explicit source/target pad IDs and a
fresh output directory; it uses policy 3 explicitly. A preflight-only mode
does no simulation or output creation. Do not change the existing `run`
argument model or overload generic controller flags.

The flight handler drives the session itself, rather than simply invoking the
old whole-flight convenience function. Split the existing writer into execution
and writing an already completed result, so the adapter does not run the
mission twice. Reuse the V2 scenario/full-flight/compact-summary format and
annotation-capable rich renderer. Keep unsupported output honest, without
invented trajectory plots. Standard output is structured outcome JSON;
optional progress belongs on standard error or a separately identified trace.
Add a CLI bundle receipt binding the complete request, explicit pad IDs,
policy and artifact digests. Replay must not infer the original request from
an ambiguous set of pads. Keep the existing flight/summary DTOs unchanged;
the receipt is additive CLI metadata, with matrix source/binary provenance
recorded separately by the validation harness.

Flight exit zero requires verified target landing, mission success, a landed
planning stop, integrity and final source replay. Valid finite non-landings and
unsupported inputs return nonzero after retaining their evidence. Typed JSON
distinguishes those from implementation failures. These single-flight semantics
do not change the
existing evaluator collection-only command.

Saved replay accepts the bundle directory, validates its input/artifact binding,
starts from the original scenario and consumes the recorded command prefix;
it does not regenerate a proposal, continue a partial flight
or step a saved snapshot. Compare actions, events, samples, full endpoint state,
incoming contact and original clock against the saved evidence. It can verify a
finite partial flight, including zero-command stops. Replay exit zero means
matching replay, not successful landing. Unsupported preflight has no physical
flight to replay and must be reported as such.

Add portable tests for parsing, default-off builds, create-only paths, rejected
inputs, honest exit semantics, output completeness and tamper detection. A
copied supported scenario must run from a different working directory without
requiring archived answers or fixture manifests at execution time. The adapter
remains lab-backed even when this portability check passes.

## Checkpoint 4 Full evaluator and CLI parity

Build a maintained validation harness around the frozen 44-case pack. Resolve
the tracked scenarios once and bind every case ID, group, scenario and pad ID.
Invoke the real CLI executable for all cases, not a mock runner or an evaluator
call mislabeled CLI execution. Continue across expected diagnostic nonzero
exits; reject missing or contradictory records.

Keep CLI matrix evidence in a distinct versioned validation schema. Do not
forge a controller `BatchReport` or claim that the native batch publication
gate accepts a CLI aggregate. Record source/input seals and before/after
executable digests for both binaries. Same-checkout evaluator/CLI parity must
retain the deliberate executable difference; repeat checks require the same
CLI executable. Never normalize away provenance wholesale.

Compare exact scenario/request values, selected program identities, ordered
commands, cycles, segments, contact records, state history, fuel, physical and
mission outcomes, stop and correction count. Permit only predeclared timing
fields and proven output-root path differences. The new comparison must also
preserve the current accepted baseline's physical behavior; do not drop
different arrays or outcome fields as an extraction convenience.

The approved final-source measured allowance is:

| Stage | Maximum case attempts | Entry gate |
| --- | --- | --- |
| Native evaluator matrix | 44 | Source frozen and preflight/tests passed |
| CLI matrix | 44 | Native acceptance and baseline parity passed |
| CLI repeat | 44 | Full CLI acceptance and evaluator parity passed |
| Total | 132 | No extra matrices or ad hoc tuning runs |

Also allow at most 42 saved-source replay invocations through the real CLI,
one for each supported case in the first CLI matrix. These are replay checks,
not additional planning/selection attempts. Unit and lifecycle tests are
separate bounded verification, not extra measured matrix captures. Deliberate
tamper tests should reject before unnecessary physical execution where possible.

### Approved replacement after the stopped CLI matrix

The first source freeze `2282890` used 53 case attempts: 44 native and nine CLI.
Native acceptance and exact baseline parity passed. The CLI matrix completed
eight direct cases, then stopped at `v2_ridge_early` on a progress validator
that confused piece origin with correction entry E. No repeat, measured saved
CLI replay or publication ran. That source and evidence remain historical;
the [stopped result](waypoint_v2_session_integration_results.md) is not replaced
or retroactively accepted.

The user subsequently approved the focused repair and resumed this goal.
The replacement allowance is three new conditional 44-case matrices (132 new
case attempts) and 42 saved-source CLI replays from a new clean source freeze.
If all matrices run, cumulative case attempts across both freezes are 185.
This is a replacement allowance, not permission to continue the failed matrix
or spend its unused cases on ad hoc captures. Every existing entry, parity,
stop and publication gate still applies.

The repair separates piece origin, selected correction entry E and actual H
in progress validation, adds corrected-flight CLI regression coverage, retains
raw evidence on output-metadata failure, and records invocation exit metadata
before parsing stdout. Trajectory selection, policy, inputs and physics remain
unchanged. The primary owns integration and acceptance; Luna owns the bounded
Rust repair. Use new evidence roots dated `20261005` in UTC; preserve the first
native capture and the complete stopped validation tree as protected scopes.

Each complete matrix must meet the existing native acceptance expectations:
36/36 mandatory landings, 11 zero-correction clear controls, 25 initially
blocked corrected landings, integrity for all 44 and source replay for all 42
supported cases. Diagnostic target landings or honest finite stops are allowed;
unsupported cases remain preflight-only. Exact parity with the retained
baseline is required for this no-behavior-change pass. Any diagnostic change
needs investigation rather than being silently accepted as an improvement.

Freeze all implementation/checker sources, policy, inputs and binaries before
measured matrices; do not commit or edit between them. Measure planning and
piece latency from these runs only, without adding timing success thresholds
or claiming hard real-time behavior. Retain timing scope and distinguish
planning, admission replay, execution and final replay; do not label an
unmeasured subtotal as total work.

## Checkpoint 5 Common reports and controlled publication

Reuse the existing detailed and batch templates. Verify the session and CLI
retain the spatial/time plots, telemetry, raw source links and exact executed
H markers. The canonical batch remains at
`/reports/eval/planner_v2_lab_suite/`, reachable through the existing home and
Waypoint planning topic. Do not introduce another research preview edition,
duplicate 44-case navigation tree or a custom stripped-down report.

Separate final validation capture from publication. Add native
`run-pack --no-publish`, defaulting to the existing publish behavior when the
flag is absent, and `publish-planner-v2 --dir CAPTURE_DIRECTORY`. Reject the
native-only flag on controller packs before execution. The publish command
checks the saved capture and uses the existing publisher, returning nonzero
if publication is refused; it runs no mission and does not manually change
`current.json`. Keep the accepted current selection/site intact until all
native/CLI/parity/replay gates pass. Publication must still use the existing
native acceptance gate and common site publisher. Its PASSED badge remains a
native-pack verdict, not an invented certification for unrelated CLI runs.

Before publication, inspect an internal rendering and CLI detail examples for
direct, one-handoff, repeated-handoff, finite-stop and unsupported cases. After
publication, run the maintained read-only report checker and desktop/mobile
browser checks on the existing LAN server. CLI details reuse the common
renderer; their explicit output location is printed for inspection. No new
standalone-report catalogue is required in this pass, and validation matrices
do not become competing user-facing batch editions.

## Checkpoint 6 Final review and handoff

Run formatting, focused session/CLI/validator tests, the full workspace suite,
feature-enabled CLI tests and strict workspace Clippy with only the established
baseline exception. Check both default-off and feature-on CLI builds. Run the
maintained JavaScript tests, real saved-evidence comparisons, replay checks,
report/link/visual checks and protected-evidence inventories.

Review and locally commit cohesive checkpoints during implementation once
execution is approved, then make a clean source-freeze commit before matrices.
After measured acceptance, commit the reviewed results and current-only doc
reconciliation. Record exact commits, binary/input identities, denominators,
allowance used, parity exclusions, latency scope and surviving diagnostic limits.
Do not push or restart the report server.

Completion requires every applicable checkpoint, real CLI evidence, unchanged
historical artifacts/default controller behavior and no outstanding correctness
finding. Deliver the commands and report links needed to use the opt-in adapter,
not merely a new API. A session-only result is a useful checkpoint but is not
completion of this larger pass.

## Stop conditions and exclusions

Stop promotion and retain evidence on a state/command mismatch, crash, source
or input drift, nonfinite propagation, missing replay, implementation error,
artifact corruption or protected-history mutation. A hard failure stops later
matrices immediately. A well-formed finite ordinary miss may be retained with
the rest of its matrix for diagnosis, but fails acceptance and forbids repeats
or publication. Do not repair a coverage miss by retuning policy or replacing
an input during this pass.

If an implementation defect is found before measured capture, fix it within
scope and rerun affected tests. After a measured failure, diagnose and propose
the correction; do not spend an extra capture or continue later matrices
without an approved replacement freeze/validation allowance. A checker-only
defect may be corrected with read-only revalidation when it does not change
physical evidence or weaken acceptance; record the correction and its limits.

Excluded work includes solving all hard diagnostics, new terrain sweeps,
arbitrary airborne restart from snapshots, other vehicles/gravities/cadences,
generic controller API redesign, shared-crate migration, removing admission
proofs for speed, asynchronous planning, runtime perturbation guarantees,
game-default promotion, historical report rewrites, pushes and service changes.
The next architecture decision after this pass is whether an independent game
host needs an evaluator-free library; the optional lab adapter must not be
described as having already provided one.
