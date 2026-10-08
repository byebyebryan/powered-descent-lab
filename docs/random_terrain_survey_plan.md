# Random procedural-terrain survey — frozen plan

[Documentation home](README.md) · [Terrain refinement](terrain_ridge_refinement_results.md)

The bounded pass reached its prescribed stop condition. See
[results and numerical guard diagnosis](random_terrain_survey_results.md): seven
attempts retained; the 100-case campaign is incomplete. The plan below remains
the original frozen contract, not permission to resume it.

## Scope

The approved next pass replaces the inspected six-shape flight recommendation
with 100 fresh procedural cases. It uses the current refined Pylander-inspired
composition and ridge slice, the supported vehicle/Earth/120–60 Hz setup,
90 s mission horizon and 1200 m separation. Planner/controller policy is unchanged.
The [machine-readable plan](../studies/terrain_profiles/survey_plan.json) fixes
the master seed, seed range, exclusions, attempt budget and stop rules before
terrain generation. No balancing, seed replacement, amplitude search or corridor
clearing after viewing outcomes. More samples expand coverage within this one
distribution; they do not establish coverage of absent terrain regimes.

## Implementation and preflight gate

Reuse the hash-bound Pylander loader, weighted composition, ridge wrapper and
local pad compiler. Save all seeds before sampling, raw 4 m polylines separately
from prepared scenarios, exact patch geometry and input-only native preflights.
Check a fresh-process reversed-order generation repeat. Keep the original study
plans and receipts untouched. Add only a thin Python collector invoking the
existing native single-flight command and a presentation-only Rust adapter to
the common batch/detail templates. Do not relax the sealed 44-case benchmark.

Before measured flights: review deterministic expansion, create-only outputs,
partial failures, typed outcomes, summaries and navigation; run focused tests and
the maintained development gate. Freeze input/source/executable hashes. Preflight
support means a valid request, not a feasible trajectory or guaranteed landing.

## Execution allowance

At most 108 measured attempts: three existing direct/single/multiple-correction
sentinels (`v2_clear_685`, `v2_ridge_early`, `v2_plateau_reference_900`), 100 new
random cases, and repeats of the first five predeclared random cases. Sentinels
must preserve their complete non-timing flight records. First ten random cases
are the runner checkpoint, not another sample or permission to tune.
Use at most four workers, a 300 s per-process wall limit and a 10800 s campaign
ceiling. Preflights and ordinary developer tests are outside this measured count.

Retain ordinary crashes, mission timeouts, off-target landings and finite stops;
continue collection without replacing seeds. Stop new launches for implementation
errors, source/input drift or applicable integrity/replay failures. Drain existing
workers and preserve raw logs/partial roots. A runner timeout is not a simulated
mission timeout. Budget exhaustion leaves every unattempted case explicit.

## Interpretation and reports

Keep all 100 identities in the primary denominator. Report input support,
initial nominal clear/blocked/not-established classification, zero/one/multiple
completed corrections, planning stop, physical/mission outcomes, integrity,
source replay, fuel, clearance and timings. Derive classification from actual
cycle/audit evidence, not a default-false blocked flag. Keep sentinels and repeats
separate. No landing-percentage acceptance gate or waypoint quota for this survey.

Use the rich common detail and batch templates with actual H annotations. Publish
only a separate development collection at `/reports/eval/planner_v2_random_terrain/`,
reachable through Report home → Waypoint planning → Procedural terrain survey.
Do not replace the accepted benchmark selector/site or change server state.
Complete means a trustworthy accounted sample and documented coverage/failure
verdict, not 100/100 landings. Follow-up tuning, broader distributions, extra
campaigns, commits and pushes require another decision.
