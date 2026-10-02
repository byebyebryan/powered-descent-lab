# Practical waypoint V2 design review

This is the historical review of the original loop implementation contract.
The current [unified nominal plan](waypoint_v2_unified_nominal_plan.md) replaces
the next-action proposal, not these earlier readiness checks or measured results.
Its review verdict is ready for estimator/entry characterization, with numerical
choices and a separate implementation review still required.

## Verdict

The [plan](waypoint_v2_practical_plan.md) is ready for a bounded implementation
phase, subject to the user's approval. It targets a complete best-effort loop
and a representative game-oriented mission suite, not perfect landing or general
viability. The user confirmed the current vehicle/gravity/rates with broad terrain
coverage. No runtime implementation or new flight is performed in this review.

This is a separate primary review of the drafted plan against current source at
`a60146d` on 2026-10-01; no delegated or independent-agent review is claimed.
All numerical coverage/performance targets are proposed acceptance choices,
not measured V2 capabilities.

## Review findings and resolutions

| Concern | Source evidence and resolution |
| --- | --- |
| Does each waypoint secretly need to land? | Existing `search_row` and `local_rank` do not use suffix generation. Preserve that separation; later nominal rejection is an allowed coverage miss. |
| Can conflict queries accidentally restart at source? | `canonical_initial_direct::first_conflict_evidence` uses `query_program_state_at`, which creates `SimulationState::new`. The plan now requires a live-origin query for airborne conflicts, matched to the fixed audit. |
| Can the guard reject legitimate landing or leak the launch exception? | `LocalGuard` treats every tick after first E as local. The plan requires a segment-ledger guard: original launch only, strict local reserve, existing nominal terminal rules. Do not reuse the one-clearing assumption globally. |
| Can ordinary difficulty be mislabeled an implementation error? | V2 distinguishes finite NoNominal/NoClearing and valid non-terrain NominalRejected from invalid binding, an unexpected in-bounds command gap, nonfinite state and parity failure. Explicit coverage ending without contact is incomplete, not an invented missing command. Existing canary semantics stay intact. |
| Can planning rewind or consume the certificate? | Candidate entries are forward queries from current C; the chosen prefix is physically executed to E. Regeneration uses actual H while the two-second certificate remains a separate clone. Whole-source proof uses the full accumulated command ledger. |
| Can repeated plateau conflicts loop indefinitely? | Each selected H advances time and at least one conservative body diameter beyond the current conflict/entry. Six corrections and original deadline bound the loop. Same feature identity is not itself a no-progress failure. A final Direct attempt remains legal after correction six. |
| Is this really a drop-in V1 RoutePlan or legacy-controller proof? | No. The first reusable mode executes the existing supplied-command primitives through ordinary core transitions. It has separate timed-segment evidence and an explicit CLI. V1 dispatch and optional legacy controller comparison remain separate. |
| Is admission consistent before and after launch? | Initial nominal preflight tolerates an empty authored route, while airborne admission requires route-free input. V2 must enforce route-free LandingOnPad up front; all planned scenarios already satisfy it. No silent normalization. |
| Can the suite hide failures or inflate coverage with harmless obstacles? | Eight clear controls and sixteen ordinary cases have fixed denominators; ordinary Unknown/Unsupported still count as misses. Require 13/16 and at least 2/4 per family, the unchanged reference landing, and at least twelve initially blocked ordinary proposals. The last requirement needs future generation, not preflight. |
| Are cost claims realistic or already validated? | No V2 latency measurement exists. Proposed planning targets are median <=2 s and p95 <=5 s, measured over all common attempts including failures. Planning, execution, replay, output and total wait are separate. No real-time guarantee is inferred. |

The source seams support an additive runner with narrow helper extraction; no
new core integrator, restore API, landing predicate or global optimizer is needed
by this design. Whether repeated clearing attains the proposed coverage remains
an implementation experiment, not a review finding of success.

## Readiness validation

The recipe manifest has SHA-256
`92869e10225a72e8716ad87c20fbc1ca3795bd692aa9e41d011cd9ae18cf438c`.
It pins complete base files and resolves eight clear, sixteen ordinary and eight
diagnostic scenarios in memory. Existing input-only preflight accepts all thirty
intended supported inputs and rejects the two other-vehicle/gravity diagnostics
as Unsupported. Every preflight reports `simulation_created: false`.

The canonical JSON digest of the complete expanded scenario/pad-ID list is
`c340bb444a24f6c99197ffd18e4f38460c17e80c5e46f03bb4a49cb554f8c5a5`.
Future suite implementation must reproduce these complete inputs, not merely
their count or names. This does not claim that the new V2 preflight exists yet.

The [checker](../scripts/check_waypoint_v2_practical_plan.mjs) verifies base hashes,
case IDs, group/family counts, complete vehicle/initial-state/clock/mission
preservation before diagnostic mutations, strict terrain ordering/domain, unchanged
pad shelves, non-lowered floor and exact reference terrain. Six structural
negatives reject lowered-height features, bad vertex order, source-pad intrusion,
overlap, ordinary-case configuration mutation and missing bases. It also checks
later-entry rounding/deduplication, short/invalid intervals, exclusive command
endpoints and the 1,008-row / 362,880-boundary arithmetic. These are design tests,
not implementation overflow tests, plant propagation or landing acceptance.

Fresh checks also pass the existing four pure local-policy tests, the existing
CLI operational/preflight test, a current pd-eval binary build, Node syntax,
workspace formatting and whitespace. The initial Node-to-Rust stdin check failed
before input parsing because `/dev/stdin` could not reopen a subprocess socket;
the checker now supplies a real read-only shell pipe without temporary files.
No terrain, policy or acceptance target changed to resolve that plumbing error.

All 143 files bound by the accepted local-clearing source/input/protocol closure
still match. Both accepted roots' summary and experiment files retain their
original bytes. No Rust source, old fixture/protocol, physics, controller, reserve
or production default changed. The prior 861-test/full-flight evidence remains the
previous checkpoint; the full workspace suite and flight gates are not repeated
by this design-only pass.

Commands for reproducing readiness from repository root:

```sh
rtk proxy cargo build -p pd-eval --quiet
rtk proxy node --check scripts/check_waypoint_v2_practical_plan.mjs
rtk proxy node scripts/check_waypoint_v2_practical_plan.mjs --preflight-bin target/debug/pd-eval
rtk proxy cargo test -p pd-plan local_clearing::tests --quiet
rtk proxy cargo test -p pd-eval --bin pd-eval operational_modes_require_create_only_output_or_read_only_preflight --quiet
rtk proxy cargo fmt --all -- --check
rtk proxy git diff --check
```

## What this review does not establish

Input support does not mean the fixed nominal arc is blocked, the local family
will succeed, the reference will land or the performance goal will be met. No
V2 flight command exists yet. The planned suite is deliberately development
coverage, with known inputs and new recipes; it is not held-out evidence or a
claim about every engine configuration.

The next authorized implementation should build the full reusable loop, then
run the matrix and at most one explicit uniform policy revision if warranted.
Saving this checkpoint does not authorize that work, a commit, push, deployment
or default promotion. The design closes readiness, not flight acceptance.
