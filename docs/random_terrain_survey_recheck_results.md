# Random terrain numerical-fix recheck — preservation stop

[Documentation home](README.md) · [Approved pass](random_terrain_survey_recheck_plan.md) · [Original stopped survey](random_terrain_survey_results.md)

## Verdict

The numerical geometry/source-rest fix is implemented and developer-validated.
The approved recheck stopped at its **first preservation control**, before any
random flights. That control landed on target with mission success, integrity
and final source replay passing, but its complete non-timing record differed
from the accepted control in one diagnostic clearance scalar. The collector
correctly retained it as `evidence_error`, not an accepted preservation result.

This is not another first-step crash or a failed flight proof. It is an exact
comparison failure after the intended geometry arithmetic change. It does not
establish the original failed procedural case's full-flight fix or broaden
random-terrain coverage. No retry or comparison relaxation was made.

## Attempt accounting

| Cohort | Frozen allowance | Attempted | Recorded under protocol | Unattempted |
| --- | ---: | ---: | ---: | ---: |
| Preservation sentinels | 3 | 1 | 0 | 2 |
| Random primary cases | 100 | 0 | 0 | 100 |
| Predeclared repeats | 5 | 0 | 0 | 5 |
| Total | 108 | 1 | 0 | 107 |

Only `v2_clear_685` ran. Native exit was zero; its verified landing projection
has zero corrections and a clear nominal audit. The campaign nevertheless
stopped with `v2_clear_685: sentinel numerical preservation failed` and a
nonzero collector exit. There are no random outcomes or landing-rate estimates
from this recheck. In particular, `random-001` was **not rerun**.

The original capture still has four attempted random cases: three verified
direct landings and one physical landing with a failed initial replay guard.
Those historical outcomes are not replaced or retroactively accepted.

## Exact difference and interpretation

After excluding only the three predeclared wall-clock timing fields, a complete
recursive comparison finds exactly one changed JSON value:

```text
cycles[0].audit.clearance_scan.minimum_airborne.clearance_m
accepted: 0.0005533556252945715
recheck:  0.0005533556252954597
delta:    8.881784197001252e-16 m
```

Its location remains physics step 1, phase `upright`, required clearance 0,
corridor `source_pad_transition`. Commands, states, contacts, selected plans,
segment boundaries, outcomes, integrity and replay flags, and every other
non-timing field are exactly identical. The mismatch is approximately
8.88e-13 millimetres, not a meaningful trajectory change.

The new calculation derives body extents from rotated local geometry rather
than subtracting rounded world coordinates. A diagnostic computed from that
geometry can therefore change in its final digits even when the whole flight
is unchanged. This was anticipated by the approved plan, which explicitly
required a stop and review rather than silently accepting such a difference.

The shared 1e-9 m admission tolerance remains limited to the exact admitted
source-rest state at global step zero. It is not a blanket airborne clearance
epsilon, and this first-airborne-step diagnostic is not clamped. Meaningful
penetration, positive airborne reserves, poststeps and handoffs remain subject
to the unchanged strict guard rules.

## Frozen inputs and evidence

The new create-only capture is
[`capture-random-20261006-v2-pad-rest`](../outputs/eval/planner_v2_random_terrain/capture-random-20261006-v2-pad-rest/survey.json).
Before any measured flight:

- All 100 raw terrain files and all 103 prepared scenarios matched the original
  reviewed capture byte-for-byte, including source/target pad geometry.
- The seed list and frozen plan matched byte-for-byte. All 100 native input-only
  preflights and the fresh-process reversed-order generation check passed.
- The source snapshots and release executable were frozen together; before/after
  identities match. The collector and exact comparison were unchanged.
- A separate preservation receipt recorded 163 original files: 48 accepted
  benchmark report/selector files, the old stopped report's 109 HTML bodies,
  its survey/receipt, and four frozen input contracts.

Base commit: `ffc59ec28613b35327c23edaeae0b586d4cdf085`, plus the saved source
snapshots of uncommitted changes. The commit alone is not the measured source.

- Executable SHA-256: `3cf291ada3f6771051b4508e172d334252192d0c88ec21bb127ce39754dd4585`
- Raw generation repeat SHA-256: `05a3b02abbea033d905c1e04757a073022ac923cf1ab897eae88470a36660b9c`
- Capture receipt SHA-256: `63cc00dfe25080c3f922fc511b42509e5d825e2baa22642ef0965650b6386422`

The capture retains all 108 accounted rows, one native bundle/log set, the
complete frozen inputs and controls, and authenticated receipts. Saved
verification executes no fresh flight or replay.

## Validation and presentation

Before source freeze, the maintained eleven-step developer gate passed,
including workspace/all-feature tests (214 evaluator tests passed, four
deliberately ignored), CLI boundaries, formatting, strict Clippy, maintained
JavaScript tests and local documentation checks. Seven survey report-adapter
tests, eight focused geometry/guard tests and all 36 terrain study/survey Python
tests pass.

The report adapter now supports one create-only `recheck-ID` child under the
existing development survey collection, with path/symlink/create-only tests.
It keeps the established common batch/rich-detail templates and displays
explicit attempted/verified/unattempted counts instead of a misleading planned
cohort landing percentage. Missing aggregates remain missing, not artificial
zero measurements. Available failed-row raw bundles/logs are linked without
fabricating accepted flight plots.

Open the separate
[recheck batch](../outputs/reports/eval/planner_v2_random_terrain/recheck-20261006-v2/index.html),
then Diagnostics and the first control's status page. Its
[raw flight JSON](../outputs/eval/planner_v2_random_terrain/capture-random-20261006-v2-pad-rest/runs/v2_clear_685/flight.json)
contains the passing native flight/proof tuple; campaign preservation acceptance
remains false. All 108 detail pages are status pages because there are no
protocol-accepted rows. There are no new waypoint annotations to inspect.

Normal navigation connects Root → Report home → Waypoint planning →
Numerical-fix terrain recheck. The original stopped report remains reachable,
and accepted benchmark bodies/selector are not regenerated or promoted.
Report/link checks are automated/static, not human visual acceptance.
Final read-only checks passed four report-navigation tests, 692 local report
links across 112 pages, and 594 local documentation links across 106 files
(54 generated-evidence links were separately skipped by the Markdown checker).
Both stopped captures still authenticate, current source/executable identities
match the new freeze, and all 163 protected original file hashes are unchanged.
The report server was found stopped and remains unchanged. No commit or push
was authorized or performed.

## Recommended next decision

Do not redesign the planner or restore the unstable geometry arithmetic to
satisfy an old diagnostic bit pattern. The immediate blocker is the preservation
contract, not evidence of a new flight defect.

Authorize one narrow comparator revision before another measured pass:

1. Keep commands, states, contacts, selections, handoffs, outcomes and proof
   flags exact. Keep all three accepted controls and the same 100 inputs.
2. Permit only explicitly named derived clearance scalar differences with a
   small, documented finite bound; log every exception. Require matching
   location/corridor/required-reserve metadata and unchanged safety disposition.
   Reject missing/non-finite fields, threshold crossings, other differences
   and larger deltas. Unit-test those rejection boundaries before freezing.
3. Keep same-source random repeats fully exact apart from the existing three
   timing exclusions. Do not reuse cross-source allowances for repeatability.
4. Freeze a new create-only capture and separately authorize the same bounded
   108-attempt sequence. Stop again on any unexpected difference or proof
   failure. Run `random-001` within the first random wave, not as an extra flight.

The exact permitted field inventory and bound need review before implementation;
this recommendation is not authorization to change comparisons or rerun.
The numerical fix remains worth validating: the saved issue is reproducible,
the fix is small, and this control preserves the actual flight exactly. Broad
procedural terrain reliability is still unmeasured.
