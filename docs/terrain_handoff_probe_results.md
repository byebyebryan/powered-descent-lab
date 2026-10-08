# Handoff timing/selection diagnostic results

Date: 2026-10-07 (local). Completes the
[bounded diagnostic plan](terrain_handoff_probe_plan.md).

## Verdict

**Delayed replanning is a demonstrated limitation in these five selected cases.**
The unchanged nominal constructor fails at all five original handoffs, yet
constructs a feasible nominal at all five earlier settled-coast points on the
**same selected maneuver**. Three pass the actual-terrain audit; two expose
another terrain obstruction. No acquisition/controller tuning was needed to
construct those earlier nominals.

Maximum horizontal braking room is not a sufficient general selection rule:
the preselected already-accepted alternative produces a clear nominal in two
cases and still produces no nominal in three. This is a useful contrast, not
evidence for adopting that ranking globally.

All **19/19 probes** record and verify: 17 primary queries and two fixed repeats.
Seven original queries reproduce saved nominal/audit results, including the
airborne and original-rest controls. Both repeats match complete query files
byte-for-byte. There are **zero new executed mission landings** and no sweep.
The paired 1k result remains **748/1000**; these observations cannot be added to
that rate or extrapolated to all 98 `NoNominal` cases.

## Three-arm comparison

| Saved failure | Original H | Earlier, same maneuver | Max-room accepted alternative |
| --- | --- | --- | --- |
| `983` | No nominal | Nominal, terrain blocked | No nominal |
| `967` | No nominal | Clear nominal audit | Clear nominal audit |
| `955` | No nominal | Clear nominal audit | No nominal |
| `024` | No nominal | Clear nominal audit | No nominal |
| `516` | No nominal | Nominal, terrain blocked | Clear nominal audit |

Controls `271` (actual last airborne H) and `000` (original rest) both retain
their clear nominal and complete original audit. Across all 19 queries, including
controls/repeats: eight `no_nominal`, eight `nominal_clear`, three
`nominal_terrain_blocked`, zero other rejection or evidence/runner errors.

### What the extra coast costs

All earlier states are upright, settled and coasting at the same forward speed
and fuel as their later original H. Earlier and original arms have exactly the
same consumed command prefix up to the earlier query. The difference is how much
unpowered time passes before asking the unchanged planner to resume.

| Case | Earlier by | Remaining distance, early -> original | Vertical speed, early -> original | Minimum reserve in early 2 s coast |
| --- | ---: | ---: | ---: | ---: |
| `983` | 5.70 s | 555.41 -> 147.60 m | +18.92 -> -37.00 m/s | 776.25 m |
| `967` | 2.80 s | 322.11 -> 163.34 m | -28.34 -> -55.80 m/s | 201.88 m |
| `955` | 1.55 s | 381.33 -> 231.88 m | +26.62 -> +11.41 m/s | 194.84 m |
| `024` | 1.95 s | 456.23 -> 294.27 m | +1.73 -> -17.40 m/s | 93.50 m |
| `516` | 4.02 s | 627.45 -> 295.27 m | +22.25 -> -17.16 m/s | 245.77 m |

Positive vertical speed means rising. The reserves are unchanged body-aware
clearance queries, not apex height or distance above the target.

The current gate requires H.x to exceed
`max(E.x, old_nominal_conflict.x) + body_diameter`. That old collision coordinate
belongs to the **old nominal**, not necessarily to the physical far edge of a
terrain feature. Once the clearing burn has changed the trajectory, waiting for
that coordinate can consume useful braking/turning room or vertical energy.
The original failures reject analytical acceleration/terminal screens before a
physical nominal witness; all five early queries reach one valid physical witness.
This establishes a timing-related construction limitation, not that each late
state is physically unrecoverable by every possible controller.

### The two blocked earlier nominals are not new crashes

`983-early` first violates the unchanged 5 m reserve at tick 3486 during
`terminal_bridge`; the audit subsequently contacts terrain at tick 3496,
x=1068.56 m. The query starts at tick 2428, so its first reserve conflict is
8.82 s later, well beyond the safe short-coast horizon.

`516-early` first violates reserve at tick 3108 during `ballistic_coast`; audit
contact follows at tick 3119, x=974.41 m. Its query starts at tick 2274; first
conflict is 6.95 s later. This is a feasible nominal obstructed by actual terrain,
the normal reason to consider another local correction in a future execution
experiment. No additional correction was queried or executed in this pass.

Those two audit suffixes contain physical `crashed` query outcomes because they
follow the blocked nominal all the way to actual contact for diagnostic truth.
They are **not** new flown missions or changes to the retained 1k reports, whose
original executed endpoints remain `flying/in_progress`. Incomplete nominal
endpoint consumption is legitimate only with native replay agreement and genuine
terrain conflict; it is not silently treated as a clear route.

The three clear early audits retain stable target touchdown, mission success,
safe contact, every airborne reserve, command agreement and ordinary/neutral
parity. Whole-prefix source/action replay also passes. Nevertheless, the early
states do not satisfy the current production progress gate, and a modified
complete V2 session was not run: **these remain counterfactual query results**.

### Why the alternative ranking is not the first fix

For `967`, the alternative reduces vx from 56.70 to 23.39 m/s and descent from
55.80 to 25.28 m/s while retaining about 80 m more altitude; it works. For `516`,
it retains about 41 m more altitude and less descent at almost the same vx and
clock; it works too. In contrast, `024` gains horizontal room but loses about
74 m altitude and descends at 39.28 rather than 17.40 m/s; it still fails.
The heuristic ignores vertical energy. `983/955` alternatives also still fail.
Do not turn one scalar into a landing certificate or add case-specific thresholds.

## Validation and retention

- Before collection, authenticate the entire retained paired 1k capture and its
  original baseline dependency. Pin the seven exact scenario/flight hashes,
  row identities, clocks and contract before any query; no tuning or retry.
- Reconstruct from original source using actual forward commands, never snapshot
  restoration. Recompute each selected native row. Original/early proposals
  equal the complete saved proposal; alternatives match their recorded accepted
  H state and fuel. Preserve the original 9600-tick absolute deadline.
- All 19 prefix proofs pass unchanged segment guards, command-source execution,
  saved-action replay and complete ordinary evidence comparison. Earlier states
  also preserve the unchanged 5 m/two-second coast and supported-family checks.
- Current and captured-source Python saved verifiers pass without new queries or
  replay. A separate read-only Node review checks every receipted file, proof
  command/piece/clock, audit outcome tuple, source/protected hash and full repeat.
  This is numerical/static review, not browser or human visual acceptance.
- All **147 selected source files**, isolated executable and **1064 protected
  evidence/report/navigation/binary files** remain unchanged across collection.
  Comparing 88 preexisting core/control/plan/evaluator Rust files to the saved
  baseline finds unchanged computation, after excluding the one diagnostic module
  declaration; only three existing CLI/report-oriented files differ.
- All **94 terrain-study tests** and **11 maintained development checks** pass
  both before collection and on the final source/docs. Final docs checking covers
  129 files and 776 local links with zero issues. Optional
  44-case retained flight parity is not run. No publication, server operation,
  production-cap/default promotion, commit or push occurs.

Capture:
`outputs/eval/planner_v2_random_terrain/capture-handoff-probe-20261007-v1`.
It retains 368 receipted files, about 102 MiB. Measured collection takes 6.931 s,
excluding baseline authentication and build. This is not a controlled planner
speed benchmark or per-frame latency guarantee.

- Contract SHA-256:
  `fef2c150ac15fd1d35d1795b9a17ced02afac4c17873226d48fd517f5bc8ad0e`.
- Manifest SHA-256:
  `add9474a6e6c13eca63eeb321f9dc3d10e008af20e1df957d737474297a814bc`.
- Receipt SHA-256:
  `6be3b7f2863cfe4ded655314ae7be5bd56ce4c610a2589510c9c604155c2c5a8`.
- Diagnostic executable SHA-256:
  `b2cb7367b0704ec6d79d1c3abd8d389edfc9a4e2e2e40472d6053c76ea9c9d75`.
- Unchanged ordinary evaluator SHA-256:
  `fdbf20a694de3ac82b79492d119fe850e4bfea5d3c4c70a3ca854a72cfe5d2af`.

Full native searches are retained for every query; compact rejection/attempt
counters summarize the airborne arm, not the initial-rest constructor's grid.
The allowance is closed. Commands are maintained in the
[evaluation workflow](evaluation.md#bounded-handoff-timingselection-diagnostic).

## Smallest justified next step

The [proposed implementation plan](terrain_early_exit_plan.md) tests one bounded
opportunity to leave an already-selected clearing maneuver for an earlier
**clear audited nominal**. Keep its old H as the fallback when that one early
query is blocked or finite-infeasible; do not rerank every row or globally remove
progress. This is an optional early exit, not a requirement that every waypoint
prove a landing suffix. It targets the three demonstrated clear cases while
preserving the two blocked cases as honest remaining diagnostics.

A global handoff-gate change was **not** tested: it would change every row's
acceptance/ranking, not merely the query time of the same selected maneuver.
Likewise, successful continuation through the newly exposed conflicts in
`983/516` is untested. Resolve those separately after real early-exit session
execution and preservation checks. No acquisition redesign is justified first
by these five cases; no conclusion is made about the other 93 nominal stops or
154 clearing exhaustions. Further effort is worthwhile as a bounded timing fix,
not an open-ended solver/threshold search.
