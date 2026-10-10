# Ballistic mechanics V14: isolated negative/mixed result

[Documentation home](README.md) · [Frozen plan](ballistic_mechanics_plan.md) · [V13 1k](ballistic_terminal_coordination_sweep_results.md)

Verdict: **do not combine or promote these changes**. The waypoint exit check
demonstrates a useful correction but loses two prior landings; early piecewise
replanning loses one; earlier warning alone gains none. The declared admission
fails, so no combined focus or new 1k ran. The latest broad ballistic result is
still V13's **639/1000**, not a rate inferred from these selected worlds.

Subsequent scope: the separately user-authorized
[unchanged exit-check 1k](ballistic_exit_diagnostic_results.md) now records
706/1000 with 78 gains and eleven losses. It leaves this panel's admission and
closed conditional stages unchanged; the headline above describes this pass.

## Complete accounting

Four modes each ran the same 45 original worlds and three external repeats:
**192 native attempts**, all complete with integrity, original command replay
and deterministic decision reproduction. All twelve external repeats match
their complete mode feedback exactly. No retries, native exceptions, executed
crashes, off-target landings, source changes during flights, guard relaxation,
terrain edits or deadline extensions occurred.

| Mode | Landings / 45 | Gains vs V13 | Losses vs V13 | Admission |
| --- | ---: | --- | --- | --- |
| Same-source V13 | 25 | None | None | All 48 complete feedback records exact |
| Exit consistency | 26 | 035, 047, 094 | 142, 715 | Failed preservation |
| Early piecewise target | 24 | None | 715 | Failed gain and preservation |
| Recovery lead | 25 | None | None | Failed required subject gain |

Four prior zero-H direct controls preserve their complete ordinary flights in
every mode. The panel includes all four recipes and 21 prior waypoint landings,
but is selected development evidence, not fresh terrain coverage. Each subject
group is fixed before flights; unplanned gains cannot substitute for its gate.

## What the flight evidence adds

### Consistent exit checks help, but automatic lifting changes useful states

The incoming/exit mismatch was real. All four selected original waypoint-exit
stops disappear as that stop category, but only **047 lands**, at 70.25 s with
two actual H. 013 advances to a terminal reserve stop; 056 reaches two H then
has no destination aim; 061 stops later in terrain recovery. 035/094 also land.

The cost is different waypoint-entry/terminal states. 142's first proposal gets
height repairs of 25.86 m and then 15.99 m; it now stops beside the target under
body reserve. 715's later proposal gets a 2.48 m lift and its changed flight
stops at 35.217 s under reserve. These are not executed crashes or proof that
height repair itself is always wrong. They show that making the ideal two-second
coast clear is not synonymous with preserving a useful realized handoff.

### Earlier attempts reveal correction churn, not a solved next leg

All four late-H subjects attempt destination planning earlier, but stop on
waypoint construction instead of landing. In 044, the stop moves from a
destination miss at H/19.750 s to a waypoint miss at 16.867 s, with no actual H.

The new adapter repeatedly reacquires the same/local waypoint while the current
coast still satisfies that waypoint's positional acceptance. In 044 it records
244 blocked early destination attempts. Of 243 subsequent route acquisitions,
230 still have a pending turn and issue an engine-off turn command. At the next
update, accepting the existing waypoint coast can cancel that queued correction
and start another early attempt. Thus the adapter does not reliably finish the
energy-changing maneuver it keeps proposing. 050/062/081 show the same pattern.

715 now stops at 26.733 s on a waypoint-construction miss, losing its V13 landing.
This negative result is specific to the new adapter; it is not evidence against
the piecewise design or proof that the original 124 H states were unrecoverable.
Simply routing an earlier blocked target fit into local replanning is insufficient
when accepted-coast logic owns cancellation and optional queries can terminate
the active leg.

### Earlier warning is not a common recovery acceptance horizon

Recovery now starts earlier, including 020 at 3.117 s rather than the previous
immediate no-command stop at 3.767 s. It still stops at 3.767 s, after preserving
forward motion. The first bounded full-thrust choice is accepted on its own
turn-plus-0.2-second horizon; longer warning does not require that selected choice
to stay clear through the common warning interval.

Recorded exhausted-query evidence now shows why each distinct choice fails.
For 020 at the final state, the two support choices violate reserve at tick +23,
upright at +51 and braking lift at +48. This is no longer an unexplained label.
All five recovery subjects remain stopped; 000/055 also end as recovery stops
rather than the original waypoint-construction misses. No new landing follows.

## Reader-only recovery, not flight retries

The initial collector did not register V14 identities in inherited countdown,
braking, centering and coast validators, or the new blocked-destination estimate.
All 144 new-mode native flights were complete and internally proved, but the
collector labeled them `evidence_error` and sealed unsuccessful collection views.
Those original captures, errors, receipts, results and pages remain unchanged.

The corrected reader rechecks every native record and creates separate `-reader`
captures. All native artifacts are byte-for-byte copies; original collector
receipt/results/index and frozen source are retained alongside explicit repair
provenance and the corrected reader. **Zero additional native attempts** occurred.
Independent verification authenticates both original seals and repaired views,
every copied original file, input/source bindings, physical/mission/replay tuple,
decision evidence, complete repeats and four direct-control flights. Reader
repair does not pass any flight admission or open the combined/sweep stages.

## Validation and reports

- 52 focused ballistic Rust tests pass, including new isolated-option, fresh
  waypoint-exit, blocked early-preview and rejected-choice evidence checks.
- All 174 terrain-tool Python tests pass, including inherited reader-contract
  coverage, fixed denominators and admission-bypass rejection.
- The maintained eleven-step development gate passes, including strict Clippy,
  formatting, workspace/CLI boundaries, 65 Node tests and documentation links.
- Static report checks pass for all 192 rich details: 17,610 embedded planning
  cycles match their sidecar data and recorded refresh counts; 65 recovery-query
  summaries are present; all 1,360 relative links resolve. The captures retain
  162 actual handoffs, distinct from refreshes. This is not browser visual QA.
- Explicit numerical parity is **44/44 exact** against the retained October-7
  `capture-early-exit-20261007-native`. An earlier attempt against October-5
  session-repair evidence fails on an 8.88e-16 m derived-clearance difference.
  Neither historical data nor the comparator was modified; do not claim exact
  October-5 parity or relabel the newer reference as the accepted site.

Create-only common batch trees and rich details:

- [Same-source control](../outputs/eval/planner_v2_random_terrain/capture-ballistic-mechanics-terminal-coordination-focus-20261009-v1/index.html)
- [Exit consistency, reader-verified](../outputs/eval/planner_v2_random_terrain/capture-ballistic-mechanics-exit-consistency-focus-20261009-v1-reader/index.html)
- [Early piecewise target, reader-verified](../outputs/eval/planner_v2_random_terrain/capture-ballistic-mechanics-piecewise-early-target-focus-20261009-v1-reader/index.html)
- [Recovery lead, reader-verified](../outputs/eval/planner_v2_random_terrain/capture-ballistic-mechanics-recovery-lead-focus-20261009-v1-reader/index.html)

Start with exit 047/142, early 044/715 and recovery 020. Recovery summaries list
each evaluated command, its horizon and first rejection tick; those predictions
are distinct from actual flight. Existing plots and waypoint/coast annotations
remain. No browser visual acceptance or root-navigation publication is claimed.

Read-only reader verification, repeated for each independent mode:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_mechanics_review.py verify --mode exit-consistency
```

## Next bounded question

Do not raise more waypoints, enlarge search or launch another broad campaign.
First make an optional early attempt a transaction: retain the checked active
leg unless a replacement is admitted, respect the existing planning cadence,
and do not discard a still-valid queued turn/burn merely because its positional
waypoint coast is acceptable. Separately compare recovery choices over a common
existing response horizon rather than letting the first short-safe choice win.
These are hypotheses supported by recorded execution, not implemented fixes or
permission for more flights. Keep 044/715 and 020 as focused subjects, with the
successful preservation panel before any next 1k.

Ordinary policy 3, controller defaults, accepted selector/site, root navigation,
historical captures and server state are unchanged. No commit or push occurred.
