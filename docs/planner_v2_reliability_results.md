# Planner V2 reliability results

The native policy 3 evaluator now has a reusable acceptance gate and a checked
batch workflow. Two fresh runs of the frozen 44-case pack passed and reproduced
the same physical results. The current common-template report selects the
second capture. This closes the reliability pass for the existing offline lab
workflow; it does not add a runtime controller or expand terrain coverage.

## Measured outcomes

Both captures recorded the following results on the tested vehicle, Earth
gravity and 120 Hz physics / 60 Hz command setup:

| Evidence | Result in each capture |
| --- | --- |
| Mandatory ordinary target landings | 36/36 |
| Clear controls | 11 direct landings, zero corrections |
| Blocked terrain cases | 25 corrected landings |
| Separate diagnostics | 2 landings, 4 finite stops, 2 unsupported |
| Integrity | 44/44 |
| Supported final source replay | 42/42 |
| Crashes or unverified simulations | 0 |
| Executed handoffs | 36 observed, not an acceptance threshold |

The four supported stops are `NoClearing` in `v2_diag_high_900`,
`v2_diag_near_source`, `v2_diag_near_target` and `v2_diag_long_plateau`. They
retain zero commands, zero physics steps and one initial sample. They are not
flown direct routes, crashes or landings. Lunar gravity and the other vehicle
remain unsupported preflight inputs with no invented flight or replay claim.
The ordinary denominator is 36, not 44; total target landings including the
two diagnostic recoveries are 38.

Exactly two measured batches ran on 2026-10-04, using four workers: 88 case
attempts in total. The second was started only after the first passed native
acceptance. No case, flight policy, numerical parameter or acceptance threshold
was changed in response to these results.

## Captures and provenance

The create-only directories are
`outputs/eval/planner_v2_lab_suite/capture-reliability-20261004-a` and
`outputs/eval/planner_v2_lab_suite/capture-reliability-20261004-b`.

| Capture | Summary SHA256 |
| --- | --- |
| First | `7147234fe78e43a9c087b11142dd582cd01903edca09ff5224b910934e7a2c7d` |
| Repeat and current | `6be868cbd059b4b654cc8d2999f8af4d2e07f2a5a959740934dc15353e0b4364` |

Both captures record clean commit
`8a550b28d258d78d918b414e18c4aadc70c5166c`, identical before/after source
states and unchanged input identity. The Rust source tree digest is
`3b0b0605785a27f0a7a01708c0cc32c40d48d0e2a0222afbe5797234a40e5201`;
the release executable digest is
`bea21a332c24bcf065a40583ff75d2384b709a40a896bb195f9273b15b4d147d`.
The frozen pack digest is
`37d68e3fc7b53cd316c1cca05b5ec53ba75b010e15477f755581871273328a8d`.
Later checker and documentation commits do not change this recorded capture
commit, Rust source or executable.

The read-only repeat comparison passed all 132 per-case JSON artifacts
(scenario, full flight and compact summary), all case rows, the pack and ordered
expanded-input snapshots, and all 42 rich report payloads. Commands, states,
contacts, outcomes, handoffs and source identity match. Only the declared
wall-clock timing fields and their derived artifact hashes are excluded; there
were no root-path exclusions in this measured comparison. This is
reproducibility over the
frozen pack, not a statistical estimate for arbitrary terrain.

## Acceptance and publication

`pd-eval check-planner-v2 --dir CAPTURE_DIRECTORY` checks an existing capture
without writing reports, simulating or replaying. It binds the complete ordered
input identity to the tracked default pack, cross-checks full and compact
execution evidence, then applies the
[acceptance contract](guidance.md#native-v2-acceptance-contract-2026-10-03).
Recorded replay evidence is validated; this command does not independently
rerun the physics.

For native batches, `--enforce-regression-policy` returns failure after retaining
a failed capture and its rich reports. Collection-only operation still prints
the verdict but retains completion-only exit semantics. Neither mode can
publish a failed batch as current. Historical policies remain inspectable but
are not eligible for this policy 3 acceptance gate. Corrupt captures fail before
publication writes; refreshing a refused current capture reports an explicit
error rather than pretending it was uncaptured.

The shared batch report adds an acceptance verdict and case-linked failure
reasons without replacing its existing sections, review tree, plots or detailed
pages. The first maintained report-check attempt incorrectly required visible
tree rows to follow raw input order. That checker assertion was corrected
after both captures: visible rows bind by exact case identity while preserving
intentional category grouping. A portable regression test rejects missing,
duplicate and foreign rows. No renderer or flight correction was needed.

## Validation and preservation

The final Rust source passed `cargo test --workspace` with 992 tests passed and
9 ignored, formatting, all-target checking, release build and workspace Clippy
with `-D warnings` and only the pre-existing `single_element_loop` exception.
All 47 JavaScript tests passed, including ten new maintained-workflow tests.
Portable tests cover frozen input binding, ordinary misses, diagnostic honesty,
corruption, contradictory execution records, zero-command clocks, CLI outcomes
and preservation of existing views when publication is refused.

The maintained report checker passed against the selected capture, verifying
46 rendered HTML page hashes, all 44 projected and visible mission rows, exact
handoff coordinates and all 42 original rich payloads. LAN browser checks
passed at 1440 and 390 px through both home URLs, topic, batch and detail pages,
including direct, one-handoff, three-handoff, finite-stop and unsupported
examples. Expansion and rich plot/handoff selection and toggles work. Four
screenshots were inspected; wide tables scroll locally on mobile. There were
no report/chart/resource errors apart from the separately recorded optional
favicon 404. Browser evidence is in
`outputs/validation/planner_v2_reliability_20261004-browser/`.

All 17 protected historical scopes are unchanged. The previous accepted raw
capture still has 223 files with aggregate digest
`3df87a86ea29f2ffde1dce787cdd55e20039579d100d4b1967a06b47d951d0cb`.
Flight/controller algorithms, fixtures, physics and contact semantics are
unchanged. The existing `pdlab-reports` tmux server, PID 483461, was neither
restarted nor replaced; only the disposable validation browser was closed.
Generated evidence remains outside Git. This pass makes local commits, not a
push or runtime-default promotion.

## Review the report

Start at the [current batch](http://192.168.1.110:8000/reports/eval/planner_v2_lab_suite/),
also reachable from the report home through Waypoint planning. Expand clear
controls for a direct route; `v2_ridge_early` shows one executed handoff and
`v2_plateau_wide` shows H1/H2/H3 followed by target landing. Compare
`v2_diag_high_900` to see an honest stop before departure, not a first-step
crash. The report's raw source links lead to the selected capture.

To repeat the saved-evidence checks without another measured batch:

```bash
target/release/pd-eval check-planner-v2 --dir outputs/eval/planner_v2_lab_suite/capture-reliability-20261004-b
node scripts/check-planner-v2-workflow.mjs --compare-dir outputs/eval/planner_v2_lab_suite/capture-reliability-20261004-a
```

## Recommended next phase

The offline planner lab is usable with a repeatable regression gate. Further
nominal fitting or reopening the old floor-cutaway research is not the next
requirement. The remaining integration gap is a normal runtime consumer;
planning here is still evaluator-owned and synchronous. These captures spend
about 0.37 s planning per ordinary flight on this host, which is not a per-tick
or hard real-time guarantee. The four difficult diagnostic geometries and
unsupported setups remain explicit limits.

The reviewed [runtime design](guidance.md#optional-v2-runtime-consumer-next-phase-design)
starts with an internal session/iteration seam and command/state parity before
adding an optional consumer. Preserve the actual full live state, incoming
contact, global clock, original deadline, fuel and completed correction count.
Return a segment or a typed stop; replan after the actual handoff, without
requiring a landing suffix or terrain feature far edge. Candidate and prefix
proofs that participate in admission must remain intact. Runtime implementation,
new measured captures, latency work and default promotion require their own
approved scope; none was implemented during this reliability pass.
