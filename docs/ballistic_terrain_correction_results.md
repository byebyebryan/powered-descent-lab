# Ballistic terrain-aware correction results

Date: 2026-10-08. Status: bounded opt-in pass complete; not accepted or promoted.
The [plan](ballistic_terrain_correction_plan.md) follows the
[385/1000 bounded-replan checkpoint](ballistic_replan_results.md).

## Verdict

The new candidate lands **401/1000** on the same frozen worlds: **16 gains,
zero losses** relative to 385. All 385 previous successes retain exact complete
ordinary flight, command/update and handoff parity. There are 240 zero-H landings
and 161 after actual handoffs; the 599 other outcomes are finite flying stops,
not physical crashes or proof of impossible routes. All four recipes improve.

This supports separating route selection from immediate clearance protection.
All 16 gains execute same-goal recovery; five land without a waypoint handoff and
eleven after H. It does **not** establish a usable replacement: the older
policy-3/cap-24 experiment still lands 817/1000. The full candidate acceptance
campaign, external game-loop integration and default promotion remain deferred.

## What changed

The terrain-blind constructor still supplies the ballistic aim and finite burn
estimate toward the destination or active waypoint. A blocked desired/coast arc
can select or replace a waypoint using the unchanged placement recipe and four
bounded proposals. A rejected immediate command no longer chooses another goal.

For source-cleared, pre-terminal flight, terrain protection tries at most four
full-thrust commands: support the requested turn, support current attitude,
upright lift, and 30-degree braking lift. Identical commands are skipped. Each
query advances a clone of the actual ordinary plant, including rotation, gravity,
fuel/mass and contact. The required reserve must hold for turn time plus 24 ticks,
capped at 240 ticks. The original mandatory 24-tick guard remains a subset of
that check. These checks admit the next command, not a whole-leg/landing suffix.

Recovery keeps the exact goal and revision and updates commands every two ticks.
It ends when a fresh same-goal command passes the response check; old burn/arrival
clocks are then discarded and normal acquisition restarts from actual state.
Each episode is bounded at 600 physics ticks, alongside original mission, fuel
and wall bounds. No admitted command is a finite controller miss. No desired
velocity is assigned to the plant and no serialized state is restored.

Source upright clearance, terminal-controller ownership, actual reserve and the
two-second waypoint continuation/handoff guard are unchanged. This is basic
pre-terminal command protection, not a new general terminal avoidance controller,
terrain-following policy or terrain-aware apex search.

The common rich report retains all plots/presets and adds recovery start/resume
entries to **Jump to decision**. Recovery summaries show rejected and selected
commands, checked horizons and the retained goal. Pale purple is the selected
command's first 24-tick prediction; any red warning on the recovery-start record
belongs to the rejected command. Neither prediction is an executed crash or H.
The original-source action ledger records every actual override; periodic review
records do not replace that ledger.

## Measured results

| Terrain recipe, 250 worlds each | Previous candidate | Terrain correction |
| --- | ---: | ---: |
| Mountains 4x | 123 | 128 |
| Mountains 8x | 93 | 96 |
| Broad massifs 8x | 106 | 107 |
| Successive ridges 8x | 63 | 70 |

The gains are `114/116/124/139/164/381/395/462/685/751/785/805/907/929/977/993`.
Ten were previously waypoint construction misses and six duplicate-goal stops.
No case from the old missing-aim or continuation-rejection cohorts becomes a
landing. Existing successful trajectories are unchanged, not just still landed.

There are 120 worlds with executed recovery, 289 episodes, 205 resumes and
1,643 recovery command updates. Sixteen of those worlds land; 89 end with no
admitted recovery command, seven with continuation rejection, seven with no aim
and one with no local waypoint. Episode/resume counts are not landing counts.

| Remaining finite stop | Cases |
| --- | ---: |
| No admitted terrain-recovery command | 205 |
| Waypoint continuation rejected | 148 |
| No ballistic aim | 136 |
| Waypoint aim construction miss | 78 |
| Local proposal budget | 30 |
| No local waypoint | 1 |
| Terminal short-command rejection | 1 |

The duplicate-goal stop disappears as a label, but only six of its 53 old cases
land. Removing that label is not evidence that all those flights are solved.
No case hits the actual-handoff cap or five-second recovery cap; maximum H is
three. The remaining failures are not correction-count truncations.

## Missions to inspect

The local [common batch](../outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1/index.html)
is reachable on the existing server at
`http://192.168.1.110:8000/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1/`.
It is not the published/accepted site and is not selected by the report-home
navigation. No server lifecycle or report publication was performed.

- **114:** recovery at 4.050 s, resume at 4.233 s toward destination revision 0;
  verified landing without inserting a waypoint.
- **139:** recovery at 10.017 s, resume at 11.233 s toward waypoint 1 revision 1;
  executes H and lands. This demonstrates correction flight toward the same W.
- **034:** same W near (50.61, 100.44) m, upright recovery begins at 4.167 s;
  no admitted command at 4.333 s, still flying with zero H. It no longer enters
  a repeated waypoint proposal loop, but is not recovered.
- **055:** retains W near (244.27, 157.47) m. It executes one recovery and
  resumes ordinary acquisition, then a later recovery near x = 91 m ends at
  7.383 s without an admitted command. Zero H; it has not cleared that feature.
- **716:** H1 at tick 828 precedes selection/recovery of W2 at the same tick.
  Those are different goal revisions, not a handoff during recovery.

## Capture boundaries and interruptions

- Two development native attempts, 034/055; both retained finite results with
  original command replay and decision reproduction.
- Fixed focused panel: 19 primary missions plus two external repeats, **21/21**
  verified. All ten selected positive controls land, including flat/uphill/downhill
  without cutaways, direct/one-/two-H controls and the 308/349 regressions.
- Paired sweep: **1,000 primary worlds plus two external repeats**, all **1,002**
  admitted records verified. Native attempts additionally repeat their entire
  decisions and replay their full command ledgers internally.

The foreground collector disappeared after 340 verified wave records. Four
native runs, 340–343, had complete feedback/replay/report evidence but no captured
collector exit/stdout. They remain untouched and are not assigned invented exit
codes. Four explicitly separate `recovery-random-*` invocations reproduce their
complete feedback exactly and supply observed process/log evidence. The remaining
worlds were continued, not restarted. There are therefore **1,006 sweep native
invocations**, including four additional interrupted invocations outside the
original allowance and outside the primary denominator. This exception is
explicit in `collector-recovery.json`; the frozen no-retry plan is not rewritten.

The common batch's logical-case links for those four open the retained original
native reports. Their complete feedback is identical to the admitted independent
recovery run; the results rows' `output_dir` and `process_attempt_id` identify the
observed recovery invocation. Both copies and logs are receipt-bound.

Collection later stopped on a **reader-only false positive**, not a flight
failure: the verifier used clock intervals alone and rejected 716's previous-goal
H at the next recovery's start tick. It now uses the active goal revision and
same-tick ordering. The original error wave remains unchanged; a create-only
verification-recheck record admits the unchanged native result. No rerun of 716
or controller/parameter change occurred. The sole changed verifier file has a
separate source seal in `collector-verifier-repair.json`; the original source
snapshot and exact native Rust/binary/report-script seals are retained.

Append-only recovery/continuation helpers and their exact source copies are
retained. Their source/protected-state checks reject changes beyond the explicitly
sealed reader repair. No cases, fixture geometry, profiles, recipes, guards or
old outcomes were tuned or rewritten.

## Evidence and validation

```text
candidate: ballistic_feedback_v3_terrain_correction
Rust tree/manifests: 9b9ab115b816aea4f127f9d6e7f445c4398b2ad17b4c8dc08002cc12235025c6
native binary: 96b08b2e605a318a8d3d68b41ba37e344e203c4cbcb0331aa2310bce051e5376
planning script: 4b2d9a6fc9e0edcf116008b24e1777352d23204bff62fdb623c7e3803363282b
collection plan: 7e10a214b2a4b135718be5e79f8da94512c517a7c96688750cf5ed5df9a5c1f0
sweep receipt: 20c474ff79bf722d9bbd6a0e46fab5fbd867f510c88ac7b593df08cd39110afe
sweep results: 9f39ff37b57767b0d2efa6af3f05da63882be8c68a4515942c16adcfb9c15621
panel receipt: cc9a739bbdc1bfb551ab1b8098b8679be89e56f6cfd81c615f51d7db6f256fd0
```

The [sealed collection contract](../studies/terrain_profiles/ballistic_terrain_correction_sweep_plan.json),
focused panel, original-source replay and independent repeat proofs bind all
results. The maintained 11-check development gate passes (including 61 Node
tests, formatting and strict Clippy), 18 focused Rust tests and 133 study tests
pass, and two added same-clock reader tests preserve the valid previous-goal H
while rejecting a recovering-goal H. All **44 default planner cases** have exact
complete non-timing parity with the retained October 7 capture.

Saved verification passes for the new sweep, collector recovery, focused panel
and old 385 capture. A separate static rich-report audit checks all 1,002 admitted
records: 61,041 embedded cycles match their saved decisions, current-state fields
and goals; 454 actual H annotations bind to their executed goal revisions. All
1,006 distinct common-batch links resolve, and the four interrupted originals'
planning data matches their admitted recovery copies. LAN responses are checked
separately; there is no local browser, so no human/pixel-level visual acceptance
is claimed.
The accepted selector/site, ordinary release binary and protected pages remain
unchanged. No commits, pushes or delegation were performed.

## Next direction

Keep this control/route separation. Do not replace short-command danger with
another waypoint request or infer success from a vanished stop label.

Of the 205 no-command cases, 121 stop on an immediate warning at the final state
(five had completed an earlier recovery); 84 stop inside an active recovery.
For the 121 immediate warnings, predicted reserve loss is only 0.033–0.200 s
away, median 0.192 s. This supports investigating **earlier response-room
protection**, but does not prove every miss is late detection or physically
unrecoverable; the four-command family and conservative held-command forecast
also limit admission.

Next compare earlier actual states from these frozen flights against the same
bounded command family, separating too-late warning from inadequate command
choices and construction/continuation limits. Keep the policy small and
state-derived. No more retry/cap/profile budget or end-to-end solver is implied.
That diagnostic/design pass needs a new request; this collection allowance is
closed and no broader acceptance or publication is scheduled automatically.
