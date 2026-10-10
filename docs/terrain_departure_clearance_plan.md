# Departure clearance implementation and validation plan

Status update: **parked** in favor of
[per-cycle diagnostics](terrain_planning_cycle_review.md). This plan is not
active authorization or the current task list. Its bounds remain a retained
proposal if the departure mechanism is selected again later.

Date: 2026-10-08. Status: reviewed proposal, **not flight authorization**.
The [design study](terrain_departure_clearance_design.md) supports one isolated
lift/advance fallback. This goal completed design and read-only validation only.
A later implementation goal must explicitly accept the scope below; previous
flight allowances are closed.

## Frozen behavior

Use one maneuver per existing admitted E after unchanged initial source-bridge
exhaustion: +0/+30-degree powered stages, factor 1 of the existing conservative
cap, at most 1920 powered ticks **total**, 720 idle-coast ticks and 240 certificate
ticks. A row has at most 2880 physics transitions, with the original deadline
still taking precedence. Eight rows mean at most 23,040 additional row transitions
before proofs. Evaluate the cheap switch screen only at normal 60 Hz boundaries
while upright lift is settled.

Keep the 42-template family, its sealed V1 policy and persisted old proposal
bytes unchanged. Production remains policy 3/cap 6. Measure with a new default-off
`departure-clearance-probe` evaluator feature and separately identified,
source-sealed cap-24 binary. Compiling all features must not activate the fallback
on ordinary policy identities. Reject feature/policy/budget mismatches explicitly.

No nominal-fit changes, progress relaxation, global cap promotion, arbitrary
airborne support, source-rest exception, old ranking/early-exit changes or
snapshot restoration. Preserve actual fuel/attitude/clocks and source replay.

## Implementation sequence

1. Put pure turn/forward/coast math in a small child of
   `pd-plan/src/local_clearing.rs`: scalar inputs, no terrain/simulator dependency.
2. Add an isolated evaluator departure module for terrain envelope evaluation,
   staged live query and independent schedule validation from real E. Reuse
   neutral query/execution, exact reserve and replay helpers; do not copy plant
   code or add a serialized-snapshot execution frontdoor.
3. Add a distinct versioned departure specification/proposal and optional
   `departure_search` record to `WaypointV2LocalSearch`. Old `selected` and old
   counters still describe the old family; omit the new field when unused.
   Use an internal selection enum without changing old serialized proposals.
4. Wire it after both old searches exhaust on cycle-zero `source_bridge`.
   Among new eligible rows use the original ordering: latest E, earliest H,
   fuel, stable ID. Do not add braking-room preference or early-exit queries.
5. Bind new proposal identity, source prefix, E-to-H commands, actual H and
   certificate in native output, session progress and final replay. Reuse
   `local_correction` segments and common rich reports. H markers must reflect
   executed states, never failed query endpoints or certificate endpoints.
6. Reuse collectors' create-only input/source/ledger/receipt helpers under a new
   frozen contract. Keep the rejected half-split capture and verifier unchanged.

The primary owns integration/review. Any later requested workers need explicit
file ownership and must preserve existing reporting/study changes. This plan
does not infer delegation, commit or push permission.

## Required diagnostics and contract tests

Each new query row saves actual E, incoming velocity/attitude/fuel, switch state
and tick, progress coordinate, predicted reserve/certificate horizon, actual
phase durations, minimum actual reserve and stop phase/reason/state. Bind any
accepted H/certificate identity. Separate no screened switch, power-budget
exhaustion, upright drift loss, forward reserve loss and unsafe idle continuation.
Full rejected traces are unnecessary; these compact records must not alter
unrelated old identities.

Validation regenerates the switch and every command from genuine E and the frozen
specification. Tampering with clock, cap, phase order, thrust, attitude, deadline,
E/H or certificate must fail. The certificate endpoint cannot replace actual H.

Test finite/domain guards, state-derived duration, turn drift, narrow peaks,
quadratic interior minima, steep slopes, descending unsupported screens, high-vx
upright drift and the **combined** power budget. Native tests cover real turning,
velocity/fuel continuity, aligned commands, deadline exhaustion, tamper rejection,
old serialization, default-off/non-source/old-success behavior, new session/output
bindings and actual report markers. Synthetic tests are not mission evidence.

Before sealing any measured candidate:

```sh
rtk proxy node scripts/check-planner-development.mjs
rtk proxy python3 -B -m unittest discover -s studies/terrain_profiles
```

## Proposed flight stages

Freeze source, screen, specification and all inputs before the first measurement.
Compare to latest retained early-exit/cap-24 records, not original cap-6 diagnostic
outcomes. No per-world parameters, resampling, retries or inter-stage tuning.

| Stage | Frozen missions | Maximum attempts | Expansion condition |
| --- | --- | ---: | --- |
| A | Ten diagnostic primaries; repeats `327/024/030` | 13 | New verified landing on `327/791`; unaffected records preserved |
| B | Other 44 source-bridge failures; repeats `261/847/988` | 47 | New executed H and verified landings beyond the two subjects; intact evidence |
| C | Frozen maintained 44-case pack, isolated candidate | 44 | All 36 core landings; honest diagnostics/unsupported cases; required proofs |

Total ceiling: **104 measured attempts**. Stage B's full 46-world primary
denominator includes the two Stage A subjects. Repeats and other diagnostic
controls are not source-cohort successes. Fixed repeats are review cases, not
held-out data. Stage C is experimental acceptance, not a replacement for the
accepted cap-6 site or saved policy.

Run sequentially, 120 s per attempt; wall ceilings A=900 s, B=3600 s, C=3600 s.
Input-only preflights/unit tests spend no mission attempts. Native source replay
within a mission is part of that attempt. Stop on integrity/replay/native failure,
unexpected crash, preservation regression, protected source/evidence drift or
exhausted wall bound. Retain partial evidence. Repairs require a reviewed new
source seal and allowance, not automatic retries or a mixed-source capture.

If A produces no H, retire runtime edits and close negatively. H but no landing
is capability evidence: review its downstream stop before expansion. This
expansion gate does **not** require a landing suffix for local H admission.
Stopped B/C does not authorize bigger power bounds, new margins or entry clocks.

Unaffected diagnostics must be complete non-timing equals to baseline except
explicit root experimental policy/input identities. Preserve all six current
diagnostic landings and exact old searches even on new paths. Separately count
accepted query H, replay-proved executed H, next nominal outcome and the complete
physical/mission/integrity/replay landing tuple. Repeats match complete non-timing
records, not only terminal labels.

## Final review and later decisions

Review trigger isolation, cheap-screen versus actual-query distinction, proof
reuse, serialized bindings and source sealing before flights. Afterwards verify
saved receipts/results with native subprocesses disabled, rerun the developer
gate and confirm accepted selector/report bodies and ordinary binary unchanged.

Keep a candidate only for measured broad usefulness without lost old successes.
Report actual gains, fuel/time/correction costs and misses, not analytical 46/46
as coverage. Retire a failed runtime while retaining its evidence and saved
verification; do not accumulate executable failed experiments.

Untouched validation, full paired 1k rerun, cap/default promotion, report
publication/server changes, commits and push remain separate decisions. The
observed fresh 100 is not an untouched test. If the candidate succeeds, propose
another frozen 100 worlds excluding all 1330 previous seeds before claiming
generalization. Late airborne acquisition stays a separate workstream.
