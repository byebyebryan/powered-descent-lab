# Ballistic planner mechanics: bounded V14 pass

[Documentation home](README.md) · [V13 full diagnostic](ballistic_terminal_coordination_sweep_results.md)

Status: closed after the control and three independent modes: 192 native
attempts, failed admissions, no combined focus or new 1k. See the
[measured result and reader-only recovery](ballistic_mechanics_results.md).
No default promotion, accepted-site publication, server operation, commit,
push or delegation.

## Questions and candidate

Keep the V13 terminal behavior and terrain-blind finite constructor unchanged.
Three independent opt-ins test mechanical interfaces, without new search knobs:

1. `exit-consistency`: audit every proposed waypoint's existing two-second ideal
   continuation, not just repaired proposals. Feed violations through the existing
   fixed-x height repair and four-proposal bound. Actual H admission stays unchanged.
2. `piecewise-early-target`: when the existing negative-braking-room trigger finds
   a destination fit whose ideal arc is obstructed, use normal local replanning
   from the actual state instead of requiring a clear destination suffix. Keep
   real H separate from goal revisions and preserve the immediate command guard.
3. `recovery-lead`: use the maximum existing four-command turn/response horizon
   for earlier warning and recovery release, capped at the existing two seconds.
   Keep the mandatory short guard and the same four commands; record rejected
   recovery choices so no-safe-command stops are explainable.

`mechanics-combined` combines only these fixed changes. No flights tune arc
profiles, waypoint heights, attitude choices, reserves, clocks, fuel or terrain.
The seven destination-coast losses and thirty finite waypoint-construction misses
remain separate diagnoses; do not add a solver or terminal fallback here.

## Frozen comparison and allowance

Reference: `capture-terminal-coordination-diagnostic-1k-20261009-v1`, receipt
`49a9dd87d8df9725dd594c9979470d8984446aac31e3aecfec483db64910ccfe`, results
`7f66120ba2851da54c490937591c4fdc3e9ff370738cb6216dc3d78f87fc6121`.
Use exact retained scenario bytes; all random worlds are observed development
inputs, not a new held-out test set.

The fixed 45-world focus contains:

- Exit subjects: 013/047/056/061.
- Late-H subjects: 044/050/062/081.
- Recovery subjects: 012/016/017/020/021.
- Other diagnostic watches: 000/055/035/094/896 and terminal stops 048/755.
- Four prior H landings per recipe: 006/008/010/015, 250/251/252/253,
  500/501/504/505, 750/756/757/759.
- Prior zero-H landings: 001/264/503/753.
- Previously reviewed landings: 084/142/349/715/974.

Each mode has three external exact-feedback repeats, 047/044/020: 48 attempts.
Run one same-source V13 control plus the three independent modes (192 attempts).
Each independent mechanism needs at least one new target landing in its subject
group and no loss of the 25 focus reference landings before combination is
admitted. If any fails, close this pass with all measured negatives; do not run
the combined candidate or conditional sweep. Unit/static work may finish, but
there is no flight-driven retuning or extra allowance.

If all three pass, run the fixed combined focus (48). It must gain at least one
target landing, preserve all 25 prior focus landings, preserve complete ordinary
flights for four zero-H controls and verify every record/repeat. Only then run
the unchanged combined source on the original 1,000 worlds plus five external
repeats (044/047/020/349/715): 1,005 attempts. Total ceiling: **1,245**.

Freeze source and binary before measured flights. Source changes invalidate
admission. Record every invocation, with no retries, hidden exceptions or
replacement inputs. Four workers, 60-second case ceiling, original flight
deadline and cap-24 opt-in. A catastrophic physical/proof/source failure closes
the next stage; honest finite stops remain complete evidence.

## Acceptance and handoff

Require target physical touchdown, mission success, integrity, exact source
command replay and decision reproduction for a landing. Retain full captures,
receipts, comparison feedback, rich common details and the batch tree. Preserve
the accepted selector/site, existing navigation, historical captures and ordinary
release binary. No report redesign or root publication is part of this pass.

Before measurement: focused Rust tests, collector tests, formatting and the
maintained development gate. Afterward: independent saved verifier, exact
repeats, paired gains/losses, source/protected seals and report-link checks.
Document which mechanism worked, regressed or remained inconclusive; a failed
admission is a completed negative experiment, not permission to retune or run 1k.
