# Bounded ballistic replanning — 2026-10-08

[Documentation home](README.md) · [Evaluation](evaluation.md) · [Prior diagnostic](ballistic_feedback_sweep_results.md)

## Verdict

The final opt-in `ballistic_feedback_v2_replan_r2` lands **385/1000** unchanged
procedural worlds: all 360 original candidate landings preserved, 25 gained,
zero lost. All four terrain recipes improve. There are 235 zero-handoff landings
and 150 landings after actual waypoint handoffs. The other 615 outcomes are
finite flying/in-progress stops; there are no physical crashes.

This is a useful feedback-loop correction, not a usable-planner acceptance.
The older early-exit/cap-24 experiment still lands 817/1000 on these worlds.
The candidate loses 449 of that experiment's successes and gains 17. Neither
ordinary policy 3, cap-6 defaults, accepted captures nor published navigation
were changed. The candidate's broader pack/testing-population acceptance campaign
remains deferred; default-planner preservation is a separate check below.

## Changed behavior

The shared blocked-route handler applies to a desired ballistic arc, an admitted
coast and pre-terminal short-command terrain prediction. It may replace an active
waypoint, rather than stop just because the goal is already a waypoint. It also
tries another local proposal when the first proposed waypoint's arc is blocked.

- Construct each proposal from the actual position, velocity, attitude, fuel and
  original mission clock. Proposals do not advance the plant or manufacture H.
- Keep the terrain-blind aim constructor and existing waypoint placement recipe.
  No terrain-aware apex search, waypoint stack or complete landing suffix was added.
- At most four proposals at one actual state. Reject an already-failed identical
  proposal within that decision, but permit a fresh correction to the active goal.
- Before accepting a proposal, check its desired arc and its first physical
  command using the unchanged 24-tick prediction horizon.
- Preserve source upright clearance, actual-state reserve checks, two-second
  waypoint continuation admission, terminal-controller ownership and existing
  mission/wall-time/handoff bounds. Query errors are not terrain obstructions.

Accepted goal revisions bind executed handoffs. Same-goal reacquisition retains
the revision; a replacement or rejected proposal is not another H. The common
rich report keeps all previous panels and adds a decision selector, previous-goal
marker, exact predicted conflict, recorded proposed-command trace and revision
labels. Short predictions can show a rejected command; they are not whole-route
certificates. The final common batch shows the previous-candidate comparison
separately from the older policy-3 experiment.

## Measured progression and retained attempts

| Build/check | Scope | Outcome |
| --- | --- | --- |
| Initial mechanism panel | 12 fixed missions + two repeats | Six selected positive controls land; six selected finite cases remain finite; all 14 verify |
| First frozen replan sweep | Original 1k + repeats 055/142 | 381 landings: 23 gains, two losses relative to 360 |
| Same-goal guard repair panel | Same panel + 308/349, with two repeats | All 16 verify; both regressions land again |
| Final frozen sweep | Original 1k + repeats 308/349 | 385 landings; preserves all 381 first-revision wins and all 360 original candidate wins |

The first sweep is retained, not overwritten. Its losses `308/349` had identical
commands up to the new guard stop. The guard incorrectly treated the active goal
as an already-failed proposal, even though a fresh correction from the current
state passed the checks. The repair allows that first query without weakening any
guard or changing aim/waypoint inputs. Final `308` has exact complete ordinary
flight and handoff parity with its old successful run. Both cases have exact
external feedback repeats. No thresholds or terrain inputs were tuned between
the two source-frozen campaigns.

Development `dev-01-055` reserved an empty root then failed on a wrong input path;
`dev-02-055` used the correct saved bytes. Two panel setup invocations failed
before any flight/root reservation: missing native-pack receipt and a rejected
output location. These setup
errors are not mission outcomes. Native dev evidence, both panels and both sweeps
are preserved. In total this pass measures two dev flights, 30 panel/repeat
attempts and 2,004 sweep attempts; their denominators are deliberately separate.

The panel controls include flat `v2_clear_845`, uphill
`fresh_clear_uphill_805` and downhill `fresh_clear_downhill_805`, all landing
directly without cutaways. `001/006/142` retain direct/one-H/two-H landings;
their complete ordinary flights and handoffs match the original candidate.
`715` keeps its exact two-H finite `no_ballistic_aim` result; its exit-state
problem was not hidden or solved by this change.

## What improved, and what did not

| Original candidate stop group | Cases | Final landings from this group |
| --- | ---: | ---: |
| Active waypoint arc blocked | 95 | 13 |
| First proposed waypoint arc blocked | 101 | 12 |
| Short-command prediction rejected | 187 | 0 |
| Waypoint continuation rejected | 111 | 0 |
| No ballistic aim | 130 | 0 |
| Waypoint aim construction miss | 16 | 0 |
| Original successful flights | 360 | 360 |

The 25 gains come from the two arc-blocked groups, not from relabeling finite
stops. Twelve initially blocked-proposal cases and thirteen active-waypoint
cases execute useful flight and actually land.

| Terrain recipe, 250 worlds each | Original candidate | Final replan |
| --- | ---: | ---: |
| Mountains 4x | 122 | 123 |
| Mountains 8x | 86 | 93 |
| Broad massifs 8x | 103 | 106 |
| Successive ridges 8x | 49 | 63 |

Remaining stops: 260 waypoint construction misses, 141 continuation rejections,
129 destination/current-goal aim misses, 53 repeated failed proposals, 30 local
proposal-budget stops, one missing local waypoint and one terminal short-command
rejection. No case hits the actual-handoff cap; maximum actual H is three.

Of the 260 waypoint construction misses, 180 follow a short-command trigger and
80 follow an ideal-arc trigger. There are 234 before any H and 26 after H.
For the 180 short-triggered misses, warning time is 0.092–0.200 s (median 0.200 s).
The new waypoint is 13.93–37.61 m ahead (median 17.83 m); distance divided by
current horizontal speed gives a median 0.685 s. That ratio is a diagnostic,
not a maneuver-feasibility certificate. This supports investigating warning and
correction capability, not assuming more retries or a physically impossible world.

### 055: replacement works, recovery is still missing

At 2.917 s the desired arc toward the waypoint near x = 976.23 m is blocked
near x = 231.46 m. The final candidate replaces the goal with x = 244.27 m,
reconstructs the correction from the actual state and continues.

At 4.583 s the craft is at approximately (35.24, -95.20) m. Its next held-command
prediction loses reserve near (38.86, -91.80) m. The proposed nearer waypoint
at approximately (51.67, -64.90) m has no admitted correction in the existing
constructor, so the flight stops honestly with zero H. It has not cleared the
early obstacle or landed. The new report's decision selector exposes both events.

## Evidence, source seals and validation

All original scenario bytes, seeds, recipe assignments and baseline results are
receipt-bound. Both campaigns have exactly 1,000 primary worlds and two separate
repeats. Every native attempt repeats its decisions and replays its complete
original-source command ledger. Both sweeps and panels pass saved verification;
the original 360/1000 capture still verifies through the extended reader.

Final source/binary/report identities:

```text
candidate: ballistic_feedback_v2_replan_r2
Rust tree/manifests: ce3c648363296cc9e4c793d7602f643f9e42249d48e7357dc19f0b8b7ff2fe9b
binary: 1e654cd3670e6b0792d6a18b80bd51848dd60aadd9ecb186c16908e2956d7b67
planning JS: c4e5d57d843427392ffcfb78b599f13fc0e47decebaf650c2420019bc183240a
final sweep receipt: 25ce8b67762e5032894375e84860c8f37d2306413aff9268590ed8cc594d78ea
final sweep results: 1da91de0d5ffb40efed8c883fc8558b9f7f54aace3df02ade42f3420937dc6c7
```

Final [collection contract](../studies/terrain_profiles/ballistic_replan_final_sweep_plan.json),
prior [contract](../studies/terrain_profiles/ballistic_replan_sweep_plan.json) and
[panel runner](../studies/terrain_profiles/ballistic_replan_panel.py) retain scope
and source identities. Their completed flight allowances are closed.

Local generated evidence:

- [Final batch index](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2/index.html),
  [paired results](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2/ballistic-feedback-sweep.json)
  and [055 detail](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2/runs/random-055/report.html).
- [Recovered 084](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2/runs/random-084/report.html),
  [two-H recovery 268](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2/runs/random-268/report.html),
  [restored 308](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2/runs/random-308/report.html).
- [First frozen sweep](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-v1/index.html),
  [final panel](../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-panel-20261008-r2/panel.json)
  and [dev evidence](../outputs/research/ballistic-replan-20261008/).

The maintained eleven-check developer gate passes, including 60 Node tests;
130 terrain-study Python tests pass. There are fourteen focused Rust feedback/
report tests, including active-waypoint replacement, same-goal reacquisition,
bounded failed proposals, query-error handling and executed revision binding.
The explicit default-planner comparison to
`capture-early-exit-20261007-native` passes 44/44 exact complete non-timing
records. It is not a candidate-pack acceptance or a new published capture.
Rich report/data and HTTP checks are automated; browser pixel/user acceptance
was not performed. No browser executable was available locally.
The final 1,002 rich details bind 427 handoff annotations and 59,879 cycle
origins/goals exactly to their raw feedback. The common batch has 2,008 checked
local links and no missing targets. Its batch and 055 detail return HTTP 200 on
the existing LAN server; no server start/stop or publication operation occurred.

## Recommended next decision

Keep the shared replan mechanism, finite guards and correct report provenance.
Do not promote this candidate or increase retry/profile/cap budgets to chase
the remaining denominator.

The next bounded diagnostic should explain why a short-triggered local goal
cannot be acquired: compare warning distance/time with turn and velocity-change
room, and separate an inadequate correction family from a genuinely late query.
Then evaluate one cheap earlier acquisition preview or response-room estimate,
not a complete optimizer or landing-suffix search. Use 055 plus cross-recipe
contrasts and preserved successes; freeze any successor before paired validation.
Continuation admission and post-H aim construction remain distinct mechanisms.
