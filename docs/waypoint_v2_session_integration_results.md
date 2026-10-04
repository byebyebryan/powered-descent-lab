# Planner V2 session and CLI integration results

The session extraction preserves the accepted native Planner V2 behavior, but
the optional CLI integration is not accepted yet. On 2026-10-04 the native
44-case matrix passed and exactly matched the previous accepted capture. The
real CLI matrix passed eight direct cases, then stopped on its first corrected
case because its progress validator confused the whole piece origin with the
local correction entry. Later matrices, saved CLI replay and publication did
not run.

This is a bounded integration result, not a new trajectory policy or a failure
of the existing accepted planner evaluation workflow. The current report and
ordinary CLI/controller defaults remain unchanged. The
[approved plan](waypoint_v2_session_integration_plan.md) requires a replacement
source freeze and validation allowance before repair and measured continuation.

## Implemented boundary

`pd-eval::WaypointV2Session` owns the request, one live vehicle, the original
deadline and accumulated evidence. `advance_piece` executes either a direct
piece or the nominal approach through an intervention E and actual handoff H.
The next advance starts at that retained H. Queries and continuation
certificates stay on private clones. Corrections count only after the existing
handoff proofs pass. `finish` performs final source verification; repeated
advance or finalization after a stop cannot issue more commands.

The whole-flight evaluator wrapper now drives that session. Native capture can
use `run-pack --no-publish`; `publish-planner-v2 --dir CAPTURE` checks saved native
acceptance and uses the existing common publisher without running a mission.
Neither publication path was used to replace the current report in this pass.

The default-off `pd-cli` feature `planner-v2` adds `waypoint-v2-flight` and
`waypoint-v2-replay`, backed by the evaluator. Flight takes ordinary scenario
JSON, explicit source/target pad IDs and a create-only output root. It drives
the session once and reuses the shared rich renderer. Additive bundle and
progress receipts bind the complete request, policy and artifacts. Replay
starts from the original scenario and recorded command prefix, not a stepped
snapshot or regenerated proposal. A matching partial replay is not a landing.
This adapter is implemented but must not yet be presented as accepted for
corrected missions, a clean game library or a real-time controller.

Completed local-search diagnostics are checkpointed for implementation errors.
Integrity-failed implementation-error results retain raw and compact evidence
without an invented rich projection. That session-error path is distinct from
the CLI metadata-validation failure described below.

## Frozen source and validation

All measured attempts used clean commit
`22828907f5e7bc48b2baae17930913042fb98a78`. No source, input or executable changed
during the native matrix or stopped CLI matrix. No source repair or replacement
capture was made after the measured failure.

| Identity | SHA256 |
| --- | --- |
| Native Rust source tree | `1bfe5323b9b24daf12fd596c42f4f99fed93427f31fc93af2f991d81eefec8bd` |
| Integration source tree | `22c5965f49b7e4c9af8192eeb5c7acab2a9e8caa9173b6bcd5de71ef13a18cf3` |
| Native evaluator executable | `ce591810fb103baaf0847d2a86240f2c1cf176015841f3e005dbb8c635b300c3` |
| Feature-enabled CLI executable | `46db87f0dd49ec5527f7aeb4209eb0a129ef2989e428e982c9a0395931b54e12` |
| Frozen pack file | `37d68e3fc7b53cd316c1cca05b5ec53ba75b010e15477f755581871273328a8d` |

The default workspace suite passed 1,004 tests with nine ignored. The final
feature-enabled workspace suite passed 1,011 with nine ignored, including all
seven CLI unit/integration tests. Formatting and strict all-target workspace
Clippy passed with only the established `single_element_loop` exception. All
60 JavaScript tests passed. Both release executables built successfully.

The default-off CLI build exposes only `run`, `replay` and `report`; its normal
dependency tree excludes `pd-eval`. Real CLI input-only preflight checked all
44 bound scenarios: 42 supported and two rejected, with zero simulations or
output roots. The copied-scenario CLI test verifies a direct flight and portable
read-only replay, but it did not cover a corrected flight through bundle writing.
Native repeated-correction session tests did cover actual handoff continuity.

Evidence is retained under
`outputs/validation/waypoint_v2_session_20261004/`: `baseline.json`,
`pre_capture.json`, `native-parity.json`, `stopped-result.json` and `cli-a/`.
Generated evidence is not checked into Git.

## Native preservation result

The new capture is
`outputs/eval/planner_v2_lab_suite/capture-session-20261004-native/`.
It passed 36/36 mandatory landings: 11 clear direct cases and 25 initially
blocked corrected cases. Its eight diagnostics remain separate: two landed,
four stopped with `NoClearing` before departure, and two were unsupported.
Integrity passed for all 44; final source replay passed for all 42 supported
cases. There were no crashes or unverified simulations.

Cross-version comparison against `capture-reliability-20261004-b` passed all
132 scenario/flight/compact JSON comparisons and all 42 complete rich report
payloads. Commands, cycles, segments, selected programs, full state, contact,
fuel and clocks were preserved. The only numerical exclusions were the three
flight wall-time fields and five compact-summary wall-time fields. Native row
comparison separately excluded those three timings and independently checked
derived artifact hashes. Both source/executable provenances were retained.

## Why the CLI matrix stopped

The retained matrix is `cli-a/matrix.json`, schema
`planner_v2_cli_validation_matrix_v1`. It records nine attempted cases, eight
complete records and a failed matrix. The first eight direct cases passed
native/CLI physical and rich-report parity. Attempt nine was `v2_ridge_early`.
Its stdout was empty and stderr reported:

```text
Error: V2 progress handoff differs from its executed actual H segment
```

The frozen native evidence shows the three distinct boundaries:

| Boundary | Physics step | Meaning |
| --- | --- | --- |
| Piece origin | 0 | Start of this planning/execution cycle |
| E | 1272 | Start of the local correction after nominal approach |
| H | 2234 | Actual handoff after clearing |

The native segments are initial nominal `0→1272`, local correction
`1272→2234`, then airborne nominal `2234→4912`. Progress correctly describes
the entire piece as starting at 0. However, `validate_progress` also requires
the local-correction segment's entry/start to equal that piece origin. The
guard therefore compares 1272 with 0 and rejects the valid structure. Its H
endpoint comparisons are not the source of this failure. Independent read-only
review confirmed this interpretation.

The CLI runs progress validation before writing the flight artifacts. Its ninth
output root is consequently empty: the actual CLI physical result was not
retained, so neither its landing nor replay can be claimed. The successful
native artifact establishes the boundary distinction, not the outcome of that
unrecorded CLI attempt. The retained logs, inputs, matrix and artifact inventory
remain available. This was an output-validation error, not evidence of a
first-step physics crash.

## Stop accounting and preservation

Exactly 53 of the 132 allowed planning attempts were used: 44 native and nine
CLI. No CLI repeat or measured saved-source CLI replay ran; zero of the 42
saved-replay allowance was used. Full CLI acceptance, CLI repeat parity,
complete CLI replay, representative latency and new report/browser acceptance
remain unestablished. Unit/integration tests are separate bounded verification,
not additional measured matrices.

All 20 protected historical scopes and all five current-publication scopes
remain unchanged. The existing `pdlab-reports` server remained PID 483461 on
port 8000. There was no push, service restart, tuning, fixture change or default
promotion. The disposable validation browser was closed without touching the
report server or the user's browser.

## Proposed repair and replacement validation

Keep the session/progress meaning unchanged. Validate piece origin against
the cycle's current state, correction entry/start against selected E, and actual
segment endpoint against H. Bind the selected proposal and correction segment
without treating a nominal approach as part of the local-correction segment.
Add a regression test with distinct origin, E and H, negative endpoint checks,
and a real CLI corrected-flight test that reaches bundle writing and replay.
Also retain raw flight evidence if output metadata validation fails, and record
invocation exit metadata before parsing stdout so failed attempts remain easier
to diagnose.

After approval, use a new clean source freeze and fresh evidence roots for one
native matrix, one CLI matrix, one CLI repeat and 42 saved-source CLI replays.
That is a replacement allowance of 132 planning attempts, not permission to
resume or overwrite the stopped matrix. Keep exact retained-baseline parity,
all existing proofs and the current publication gates. Publish the new native
common report only after every applicable gate passes. Do not enlarge the
terrain corpus or redesign trajectory selection to fix this adapter defect.
