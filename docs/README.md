# Documentation home

Start with the current contracts and workflows. The many protocols and result
files are retained development records, not a list of unfinished prerequisites.
The accepted flight evidence remains the October 5 session/CLI replacement;
later code and documentation housekeeping does not create a new flight capture.

## Daily workflows

| Task | Entry point |
| --- | --- |
| Project overview and quick start | [Root README](../README.md) |
| Build, validate and make safe changes | [Development workflow](development.md) |
| Planner flights, batch evaluation and controller caches | [Evaluation workflow](evaluation.md) |
| Browse, serve, refresh and check saved reports | [Report workflow](reports.md) |
| Identify maintained scripts and their side effects | [Tooling inventory](../scripts/README.md) |
| Understand frozen inputs and archived pack metadata | [Fixture ownership](../fixtures/README.md) |

## Current contracts and ownership

- [Architecture](architecture.md): plant, scenario, controller and persisted
  artifact boundaries.
- [Guidance](guidance.md): terminal, direct-transfer, authored-waypoint and V2
  planning/execution ownership; supported envelope and source layout.
- [Waypoint planning](waypoint_planning.md): current V2 status first, followed
  by explicitly historical V1 and research checkpoints.
- [Terminal suite](terminal_suite.md) and [transfer suite](transfer_suite.md):
  maintained corpus shape, selectors and controller evidence interpretation.
- [Roadmap current status](roadmap.md#11-current-status) and
  [next product decision](roadmap.md#7-recommended-immediate-next-step): closed
  work versus procedural-terrain or host-integration choices.
- [Project direction and scope](project_direction.md): lab/game split and reboot
  rationale.

## Accepted checkpoints and completed housekeeping

- [Session/CLI replacement results](waypoint_v2_session_repair_results.md): the
  accepted native, CLI and repeat checkpoint; 36 core landings, eight separate
  diagnostics and 42 saved-source replays.
- [Common report templates](planner_v2_common_report_templates_results.md):
  shared batch/detail templates and actual handoff visualization.
- [Core-loop cleanup](waypoint_v2_core_cleanup_results.md): completed structural
  work and its preservation gates.
- [Retirement and housekeeping](planner_retirement_cleanup_results.md): sole
  current policy, removed experiments, historical decoding boundaries and final
  cleanup validation. Its [plan](planner_retirement_cleanup_plan.md) is closed.
- [Documentation/repository hygiene review](docs_repo_hygiene_results.md): current
  workflow reconciliation, output-free validation and intentionally preserved
  worktrees/history; reviewed docs/tooling checkpoint, not a new flight capture.

These are measured or completed records, not commands to generate new evidence.
For current developer validation, use the development workflow above.

## Terrain source study

The latest [independent acquisition and terminal-safety pass](ballistic_acquisition_safety_results.md)
lands **863/1000 (86.3%)** with acquisition-only: 37 gains, three protected
deadline losses and every recipe improved versus 829. Terminal-only lands
**833/1000**, with four gains and no losses. All 2,328 records, exact controls,
repeats, overlaps and independent full inventories verify. The ordinary developer
gate and later-source 44-case exact parity pass; the older published-capture
clearance discrepancy stays explicit. The [frozen pass](ballistic_acquisition_safety_plan.md)
is complete, not promoted or combined. Use acquisition-only as the opt-in
development reference; review deadline accounting and the unchanged 69 recovery
stops before another tuning pass. A combination and held-out test need a separate
decision. Existing navigation work and server lifecycle remain unchanged.

The preceding [bounded recovery probe, controls and paired 1k](ballistic_recovery_consistency_results.md)
lands **829/1000 (82.9%)**, versus 797: 33 gains and one loss (138), with every
recipe improving. The 48-case diagnostic probe preserves complete reference
flights; all final records, repeats, focused overlap and rich report data verify.
Recovery commands/goals/guards stay bounded and unchanged in family; earlier
response and common-horizon selection bring 33 more worlds into terminal control.
The ordinary developer gate passes, but a separate exact retained-parity failure
also reproduces on untouched HEAD and remains explicit. The
[bounded pass](ballistic_recovery_consistency_plan.md) is complete, not promoted.

Its destination-acquisition diagnostic and authorized independent changes are
completed above. Mathematical fits are not safe terrain prefixes. 407, 715 and
small resume-churn cases remain separate. Defaults and accepted site are unchanged.

The preceding [phase-transition probe, ablations and paired 1k](ballistic_phase_transition_results.md)
lands **797/1000 (79.7%)**, versus 786: twelve gains and one loss. Every recipe
improves; 407 is the routing regression. All final records, repeats, focused
overlap and rich controller metrics authenticate. Recovery remains the largest
failure group; common-window native queries support an earlier-response experiment,
not a claim that those failures can already land. The
[bounded pass](ballistic_phase_transition_plan.md) is complete, not promoted.

The preceding [finite-correction native probe and paired 1k](ballistic_finite_correction_results.md)
lands **786/1000 (78.6%)**, versus 706: 109 gains, 29 losses, every recipe
improved. The probe confirms that the instantaneous ideal arc can reject a safe
finite acquisition. All final records, repeats and focused overlap verify.
The [bounded pass](ballistic_finite_correction_plan.md) is complete, not promoted.

Its recommended bounded recovery experiment is completed above; the earlier
warning counts were finite diagnostics, not promised landing gains. Pad recentering
and 715's distinct aim-admission boundary remain separate; waypoint-energy ranking
stays deferred. Defaults/site/navigation are unchanged.

The separately authorized [unchanged exit-check 1k](ballistic_exit_diagnostic_results.md)
lands **706/1000 (70.6%)**, versus V13's 639: 78 gains, eleven losses and all four
recipes improved. All 1,005 records, five repeats and 45 panel overlaps verify;
46 of the original 70 exit failures now land. The failed local admission stays
failed; this is a diagnostic, not promotion. The
[ownership/waypoint-energy design](ballistic_correction_ownership_design.md)
preceded the native finite-correction pass above. Its softer waypoint-ranking
proposal is deferred; slower-fit screens are mathematical, not landing proof.
Defaults/site/navigation remain unchanged.

The [isolated V14 mechanics result](ballistic_mechanics_results.md) closes the
[bounded pass](ballistic_mechanics_plan.md): exit consistency is 26/45 versus
25 (three gains, two losses), early piecewise planning is 24/45 and recovery
lead remains 25/45. All 192 native records/repeats prove; separate reader-only
captures repair collector identity recognition without another flight. All
admissions fail, so no combination or new 1k ran. The next question is queued
correction retention and consistent recovery response checks, not more height
or search tuning. Defaults/site/navigation remain unchanged.

The separately authorized [frozen terminal-coordination 1k](ballistic_terminal_coordination_sweep_results.md)
now records **639/1000 (63.9%)**, versus 592: 49 gains and two losses (048/755).
All 1,005 records/repeats verify with no native exceptions or executed crashes.
All four recipes improve, but 335/361 failures stop before terminal entry with
unchanged flights. The [diagnostic contract](ballistic_terminal_coordination_sweep_plan.md)
is closed; the earlier panel gate stays failed. Defaults, accepted site and
root navigation stay unchanged. Inspect the large preterminal cohorts before
another tuning batch; perfect terminal completion alone would reach only 665/1000.

The [terminal coordination result](ballistic_terminal_coordination_results.md)
lands 115/121 selected random worlds versus V12's 75, recovering 41/45 lost-success
cases and all three upright-settlement cases. It loses 048/755, failing the
129-attempt admission; the conditional full sweep did not run. The next question
is lateral capture alongside required lift, not waypoint placement or a longer
deadline. Its [plan](ballistic_terminal_coordination_plan.md) is closed. The full
diagnostic above subsequently ran under separate user authority without tuning;
it does not retroactively pass this panel admission.

The [unchanged-centering full 1k](ballistic_terminal_centering_sweep_results.md)
now records **592/1000 (59.2%)**, versus 562: 75 gains and 45 former landings
not reproduced (33 verified stops and 12 unverified domain errors). The fix lands
75 of the original 102 terminal short-command failures, but all 16 domain errors
stay in the denominator. All 1002 original invocations completed, with 986 full
proof records and no retries. The separate
[initial contract](ballistic_terminal_centering_sweep_plan.md) and
[contained-error continuation](ballistic_terminal_centering_continuation_plan.md)
are closed; the earlier exact-control admission stays failed. Next diagnose
terminal regressions and evidence loss before changing waypoint logic or tuning.
Defaults and accepted site remain unchanged; the older policy-3/cap-24 result
remains stronger at 817/1000. The sealed batch's recorded-only loss headline
omits 12 unverified former wins; use the complete 45-loss comparison above.

The [body-aware terminal centering result](ballistic_terminal_centering_results.md)
lands 142/974 and improves selected preservation from 8/16 to 10/16 without
losing previous-candidate landings. All three direct controls land, but changed
braking commands fail the exact-flight criterion, so the conditional 1k did not
run under that contract. Its [plan](ballistic_terminal_centering_plan.md) is closed
after 25 records; the separately scoped full sweep above uses outcome-based
control preservation without tuning. Defaults and accepted site remain unchanged.

The [focused terminal braking result](ballistic_terminal_braking_results.md)
lands 974 but leaves 142 at a correctly rejected pad-edge body-reserve stop.
All four records verify and repeat exactly. Its
[plan](ballistic_terminal_braking_plan.md) is closed: the preservation gate failed,
so no wider flights ran. The subsequent centering result above addresses that
defect without relaxing reserve or promoting this experimental candidate.

The [completed preservation and 1k validation](ballistic_coast_terminal_validation_results.md)
records 8/16 panel landings without losses versus V9, then **562/1000** versus the
last complete ballistic candidate's 401: 200 gains, 39 losses. All 1023 declared
records verify, with no actual crashes. The coast branch lands eight of nine
selected worlds; 900 exposes later terrain beyond its preview. This is broad
progress across the combined refinements, not an isolated branch effect or
replacement acceptance for policy 3's retained 817/1000. Next review the paired
losses by flight phase and 900's ownership boundary, without more tuning/flights
in this closed pass. Defaults and accepted site remain unchanged.

The [automatic coast-to-terminal result](ballistic_coast_terminal_results.md)
lands 084 with H1 only, without W2 or a saved takeover clock. All 22 fixed
comparisons verify: 4/6 selected random landings versus 3/6, no losses, exact
ordinary flights elsewhere and three exact direct controls. This is a focused
mechanism result, not a new 1k rate. The subsequent validation above completes
full preservation and the paired sweep; defaults and accepted site remain unchanged.
Its [plan](ballistic_coast_terminal_plan.md) is closed.

The [084 coast-through capability probe](ballistic_terminal_coast_results.md)
lands with H1 only by preserving the existing coast and handing to the ordinary
standalone terminal controller at a predeclared checkpoint. Transfer-configured
comparisons remain airborne at the mission budget. All four records verify;
this is a continuation witness, not an automatic H1 selector or a new pass rate.
The automatic result above now tests coast-to-terminal ownership before adding
an unnecessary clearing waypoint. Defaults and accepted site remain unchanged.

The [completed retained landing countdown](ballistic_landing_countdown_results.md)
closes all 48 planned records. The clock is retained correctly, but combined
084/715 still stop and the panel stays at 7/16, with no gained/lost landings.
084 runs out of thrust; 715 releases on tilt after a terminal-speed change.
An initially admissible fallback is not a feasible complete braking profile.
That profile limitation remains measured context; the later coast probe above
tests a separate ownership alternative. Defaults/site remain unchanged.
Its [protocol](ballistic_landing_countdown_plan.md) is closed.

The [completed landing-duration experiment](ballistic_landing_duration_results.md)
opens terminal ownership in 715/084/142, but they stop later during descent.
It gains 268 without lost landings: 7/16 versus the combined V7 panel's 6/16.
All 44 records verify; the allowance is closed and defaults/site are unchanged.
The sampling gap is real, but repeated zero-net-vertical fits waste braking room
in 084/142. The countdown follow-up above does not solve landing; keep 715's
edge issue separate. This is not permission for another campaign or adoption.

The preceding [local waypoint experiment](ballistic_local_waypoint_results.md)
lands 715 with early destination reacquisition alone, before its late second
waypoint. Local height lowers that waypoint but does not land; the combination
also stops later and loses 084/142, falling from 8/16 to 6/16 random landings.
All 65 records verify, with exact direct controls and unchanged defaults/site.
The follow-up above diagnoses the ownership gap and tests it without completing
landing recovery; early-only still needs full preservation before a wider
campaign. This allowance is closed.

The preceding [waypoint-entry experiment](ballistic_waypoint_entry_results.md)
demonstrates lower-effort entry and higher-energy recovery in 349: both land
without an apex constraint. The combined panel remains 8/16 random landings,
with three gains and three losses versus V5 and three exact direct controls.
All 53 planned records verify. This mixed result is not adopted or a new 1k rate;
downstream-state tradeoffs need review before another campaign.

The preceding [ridge-aware waypoint panel](ballistic_ridge_waypoint_results.md)
targets meaningful crests and lands 807 with one H, but loses three prior selected
landings: 8/16 random worlds versus 10/16, with three exact direct controls.
The [pinned pass](ballistic_ridge_waypoint_plan.md) is complete and is not ready
for adoption or a wider campaign. Its pass-through timing/overshoot diagnosis
motivated the subsequent waypoint-entry experiment above.

The preceding opt-in [waypoint-clearance repair](ballistic_waypoint_clearance_results.md)
removes the original proposal-budget stops in 807/928 and adds a landing in 484.
All twelve selected positive controls remain successful; 807/928 still stop later.
Its 21-record mechanism panel is not a new 1k result or accepted planner revision.

The [procedural profile study](terrain_profile_study_results.md) compares 18
Pylander-based and FastNoiseLite profiles, with raw data, shared-scale plots and
repeatability/sampling checks. It is a shape-only checkpoint, not a new flight
capture or planner-coverage claim. Its [runner and frozen contract](../studies/terrain_profiles/README.md)
are explicit opt-in; the flight workspace and accepted report site are unchanged.

The [ridge-slice refinement](terrain_ridge_refinement_results.md) adds twelve
fixed-offset before/after profiles and twelve locally pad-prepared inputs. The
shared ridge motif is reduced and 12/12 native input-only preflights pass; no
procedural flight or landing result is claimed. Its offline galleries remain
separate from the accepted common-template flight report site.

The [random-terrain survey plan](random_terrain_survey_plan.md) freezes 100 fresh
cases with separate preservation controls/repeats and common rich flight reports.
It is exploratory coverage, not a replacement for the accepted 44-case benchmark.
The [completed full-sweep results](random_terrain_full_sweep_results.md) record
100/100 verified random direct landings, three preserved controls (including
one- and three-handoff landings), and five exact non-timing repeats. All random
routes were clear: this remains the separate sanity baseline.
The subsequent [harder-terrain coverage plan](terrain_challenge_plan.md) and
[completed challenge results](terrain_challenge_results.md) retain a separate
24-case calibration and independent 100-case population. The latter has 64
blocked routes, 21 corrected landings and 36/36 clear direct landings, with 43
honest finite stops. This exposes planner coverage limits without changing the
accepted benchmark or rewriting the sanity sweep.
The [intervention-timing comparison](intervention_timing_results.md) retains a
16-case logging-only baseline and timing candidate: three versus six landings,
but a successful control and the existing ridge CLI test regress. The replacement
is not adopted; previous timing is restored and compact query diagnostics remain.
The conditional full recheck was not run. This is development reuse, not a new
held-out estimate.
The subsequent [failure-only fallback](intervention_fallback_results.md) preserves
all 57 prior challenge landings and adds eight, for 65/100 on the unchanged worlds.
Its fresh benchmark passes all 36 core landings; the accepted selector/site remain
unchanged. The current source tries extra timings only after finite primary
search exhaustion, with unchanged guards and actual-handoff replanning.
The completed [handoff braking-room preference](handoff_room_results.md) adds
three landings for 68/100, preserving all 65 previous successes. It changes
only a negative-room winner when an already accepted nonnegative-room alternative
exists. The benchmark and accepted site remain unchanged; this is a cheap
development heuristic, not a landing-feasibility certificate.
The completed [fresh 1,000-case validation](terrain_validation_1k_results.md)
lands 733/1000 unseen worlds, including 346/613 blocked routes; all 387 clear
routes land directly. Its [plan](terrain_validation_1k_plan.md) retains the same
four recipes with 250 worlds each and separate controls/repeats. All 1008
attempts verify without tuning or publication; the previous 100 worlds remain
development evidence, not part of its fresh denominator.
The subsequent [capability diagnostic results](terrain_diagnostics_results.md)
close a [ten-world selection/build plan](terrain_diagnostics_plan.md): six known
finite failures, three corrected-landing comparisons and one direct control,
plus three exact repeats. All 13 attempts reproduce their complete baseline
records except wall timings. These portable frozen missions establish focused
departure/acquisition/progress tests, not new coverage or a planner fix.
The follow-up [correction-cap diagnostic](terrain_cap_probe_results.md), under
its [bounded plan](terrain_cap_probe_plan.md), preserves the complete H6 prefixes:
`030` lands with seven corrections under isolated cap 12; `280` reaches a later
`NoClearing` after eight. Controls/repeats verify. Production remains cap 6;
these paired worlds do not establish a new sweep rate or faulty short-hop logic.
The subsequent [paired 1k cap sweep](terrain_cap_sweep_results.md), under its
[fixed plan](terrain_cap_sweep_plan.md), reruns every original world with cap 24:
748/1000 landings, all 733 previous successes preserved, maximum 13 corrections
and no cap-bound cases. All 1010 attempts verify. The remaining 252 stops are
clearing/nominal exhaustion; production cap 6 is not silently promoted.
The [bounded handoff diagnostic plan](terrain_handoff_probe_plan.md) compares
original, earlier same-maneuver and preselected alternative query states in five
nominal stops, with two controls and two repeats. It tests timing versus selection
versus acquisition, without changing flight behavior or claiming new landings.
Its [completed results](terrain_handoff_probe_results.md) find nominals at all
five earlier states: three clear audits and two terrain-blocked, versus only two
clear max-room alternatives. The [bounded early-exit pass](terrain_early_exit_plan.md)
now has [executed results](terrain_early_exit_results.md): three selected stops
land, two blocked queries preserve their old-flight fallbacks, and the normal
44-case acceptance pack passes. Those selected checks did not establish a new
1k rate. The separately authorized [paired early-exit rerun](terrain_early_exit_sweep_results.md)
now records 817/1000 landings: 69 gained, all 748 previous successes preserved,
and all 387 clear routes unchanged. All 1010 attempts verify; the remaining
148 clearing and 35 nominal stops are finite, not crashes. Its
[fixed plan](terrain_early_exit_sweep_plan.md) leaves production and reports unchanged.
The subsequent [fresh checkpoint and departure probe](terrain_departure_probe_results.md)
records 77/100 new-world landings (35/35 clear, 42/65 blocked), with all controls,
repeats and replays passing. Its fixed lift-then-forward candidate admits no H
on `327/791`; that negative experiment is retained, not adopted. The core edits
were removed and no broader campaign or default promotion followed.
The subsequent [departure clearance design review](terrain_departure_clearance_design.md)
uses the complete 46-world source-bridge failure cohort. Its analytical screen
supports one bounded lift/advance probe, not new flight or landing evidence.
The [implementation and validation plan](terrain_departure_clearance_plan.md)
is reviewed but unexecuted and now parked. The
[planning-cycle review](terrain_planning_cycle_review.md) adds recorded,
state-aware nominal overlays at each actual handoff to the common rich reports,
and identifies later approach/acquisition and unsupported-family mechanisms.
Review of `715` exposes a separate nominal-construction gap: its ballistic coast
can miss the destination while a long powered landing segment finishes the
transfer. The [updated ballistic aim and correction plan](ballistic_aim_correction_plan.md)
selects state-based aiming, waypoint targeting and rechecking during powered
correction, with simple landing safeguards and the full 1k/testing-set regression.
The [bounded construction results](ballistic_feedback_results.md) now record an
opt-in candidate: verified neutral correction from exact `715` H1, direct flat
and uphill landings without cutaways, and two executed H in original `715`.
Waypoint coverage remains insufficient; the full paired campaign and default
promotion are deferred. This does not replace the retained 817/1000 checkpoint.
The subsequent [unchanged-candidate 1k diagnostic](ballistic_feedback_sweep_results.md)
records 360/1000 landings, including 125 waypoint landings; the 640 other outcomes
are finite flying stops, not crashes. It provides broad failure groups and a
common-template local batch index, not acceptance or default promotion.
The subsequent [bounded replan pass](ballistic_replan_results.md) lands 385/1000:
all 360 original candidate successes preserved, 25 gained, and no physical crashes.
It adds active-waypoint replacement and same-goal reacquisition with explicit
report provenance. Short-command-triggered recovery remains unproven; the broader
candidate acceptance campaign is still deferred.
The subsequent [terrain-aware correction pass](ballistic_terrain_correction_results.md)
keeps goals during immediate clearance recovery and lands 401/1000: 16 gains,
all 385 earlier successes exactly preserved. It is opt-in and unaccepted; the
original-source sweep, collection recovery and reader-only repair are explicit.
The [original stopped results and diagnosis](random_terrain_survey_results.md) record three
verified random direct landings and an initial replay-envelope roundoff failure;
96 random cases were unattempted in that retained original capture. No
arbitrary-terrain reliability claim is made.
The [numerical fix and recheck plan](random_terrain_survey_recheck_plan.md)
keep the same 100 inputs and 108-attempt ceiling. The [recheck results](random_terrain_survey_recheck_results.md)
stop at the first preservation control: its flight/proofs pass, but one clearance
scalar differs by 8.88e-16 m. No random flight was launched in the recheck.
The subsequent [comparison-aware results](random_terrain_survey_comparison_results.md)
accept the direct control but stop at the corrected control's diagnostic-derived
proposal hash. Its maneuver/flight/proofs are unchanged; the comparator needs
selected-trajectory/hash-binding coverage, supplied by the subsequent full sweep.
The [full-sweep continuation](random_terrain_full_sweep_plan.md) supersedes the
earlier stop-on-first-error/no-retry execution policy: repair routine defects,
retain every source-frozen attempt and complete the unchanged full population.

## Research and history

[Development history and archive](history.md) groups the earlier trajectory,
V1, V2, alternative-planner and presentation records. It preserves their
original filenames, dated results and local evidence links. Restored links for
retired sources point to archival revisions, not live implementations. [Progress](progress.md)
is append-only checkpoint history; [early design](early_design.md) is exploratory.

## Reading evidence correctly

- A planning stop or successful collection exit is not a landing. Keep planning,
  physical outcome, mission outcome, integrity and replay evidence separate.
- Do not combine the 36 core planner cases with the eight diagnostics, or mix
  planner denominators with terminal/transfer controller matrices.
- `outputs/` contains local generated artifacts, not files supplied by Git.
  Missing local captures do not justify inventing results or deleting history.
- A cleanup check can preserve an accepted capture without making that capture
  evidence from the newer source. Per-tick host integration and arbitrary-terrain
  reliability are not established by the current lab checkpoint.
