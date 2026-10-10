# Procedural terrain profile study

[Documentation home](../../docs/README.md) · [Frozen plan](plan.json)

The completed [results and next-pass recommendation](../../docs/terrain_profile_study_results.md)
record all measured outcomes and limitations. The incomplete first capture is
retained separately; the final root is `capture-20261006-v1-reviewed`.

An explicit, bounded shape study, not a new production terrain API or planner
pack. It compares 18 raw 1D profiles: six predeclared seeds for each of three arms.
The planner, flight workspace dependencies, accepted evidence and report site
are unchanged. No flights or landing claims belong to this study.

## Frozen comparison

- **Baseline:** the actual `LayeredTerrainGenerator` defaults from Pylander
  revision `41c4be26c3bb209a495c725bdb9d618378a24202`. Two source hashes bind
  the class and its actual local noise implementation. The loader evaluates
  only these inspected class definitions; it does not import the game engine.
- **Refined:** retain Pylander's composition and sparse-feature functions;
  calibrate macro/structure scales to 1800/600 m, amplitudes to 80/140 m, and
  use three octaves with gain 0.35 and lacunarity 2. Warp amplitude is 25 m;
  feature cells are 360 m. These are pre-generation, gamified study choices,
  not flight-calibrated values. Details are in the plan.
- **FastNoiseLite:** use the same refined recipe and cell hash with the pinned
  Rust `fastnoise-lite` 1.1.1 **Perlin primitive**, not a differently configured
  built-in fractal. Coordinates and recipe arithmetic are f64; primitive samples
  are f32. Gradients, permutations and internal normalisation differ from
  Pylander; equal seed numbers do not identify equivalent worlds.

The one weighting refinement uses the previous octave's signal to attenuate
subsequent regular/ridged contributions. Strength 0.65 blends constant weighting
with `(n+1)/2` for ordinary noise and `(1-|n|)^2` for ridges, clamped to [0,1].
Normalisation uses the unweighted geometric amplitude sum, not a varying local
divisor. Macro, warp and sparse features are otherwise unchanged in form.

These ideas are supported by [FastNoiseLite's controls](https://github.com/Auburn/FastNoiseLite/wiki/Documentation)
and [coarse-to-fine procedural extensions](https://www.decarpentier.nl/scape-procedural-extensions).
The implementation is an explicitly described study recipe, not a claim of
exact Swiss/Jordan turbulence, erosion or physical landform simulation.

The two refinements (scale and weighting) are bundled for this bounded comparison.
It cannot establish their separate causal effects or rank noise backends universally.
The additive plateau-offset feature remains unchanged: it does not flatten
underlying noise. No pad preparation, regional system, erosion or geology is added.

## Sampling and review contract

The review span is 0…1200 m, with a raw −160…1360 m domain for both endpoint halos.
Primary polyline spacing is 4 m. A 1 m reference also measures 4 m and 8 m linear
interpolation loss. An error above 0.5 m is a warning, not permission to discard
the profile or change the recipe. The reference is still sampled, so it is not
a continuous-clearance bound or proof of flight suitability.

All arms sample at world coordinate `x + 137 m`, recorded before generation.
The non-lattice origin avoids forcing every source onto the exact zero-noise
grid intersection; it is a fixed translation, not seed-dependent crop selection.

All 18 profiles remain in the gallery. Heights are translated by h(0) **only
for display**. No per-profile rescaling, min/max normalisation, terrain-aware
arc shaping, source selection by planner outcome, or flight corridor clearing.
Three-arm plots share one vertical scale. Refined-only overlays share a second,
explicitly labelled scale to permit inspection without hiding baseline differences.

Metrics describe core-span relief, endpoint rise, slopes and sampling loss.
Significant turns are local extrema with at least 8 m excursion to both adjacent
extrema and at least 40 m width on each side at half that local prominence.
They are not full topographic prominence or planner obstacle labels.
A zero turn count can mean a sustained slope, not an uninteresting or safe route.

Before interpreting the gallery, require finite increasing raw grids, exact
recipe/source binding, 18 retained identities, and a fresh-process repeat with
reversed seed and coordinate order. Review variety and shape at physical scale.
Do not infer vehicle feasibility or reliable landings from these descriptors.
Every Rust sparse-feature sample is also checked against the actual Pylander
feature function to 1e-10 m; backend comparisons change noise, not feature hashes.

## Explicit build, capture and verification

The Rust helper is a separate `[workspace]`, not a member/dependency of the flight
workspace. Its dependency lock is tracked. Cargo may fetch ordinary small source
dependencies; no model weights, terrain assets or global package installation.
The capture uses Python standard-library code and an explicitly supplied
Pylander checkout, whose bound source is copied into the create-only capture.

```sh
rtk proxy cargo build --release --locked \
  --manifest-path studies/terrain_profiles/fastnoise/Cargo.toml \
  --target-dir target/terrain-profile-study
rtk proxy python3 -m unittest discover -s studies/terrain_profiles
rtk proxy cargo test --locked \
  --manifest-path studies/terrain_profiles/fastnoise/Cargo.toml \
  --target-dir target/terrain-profile-study
rtk proxy python3 studies/terrain_profiles/study.py capture \
  --pylander-root /home/bryan/code/pylander \
  --fastnoise-bin target/terrain-profile-study/release/pd-terrain-profile-fastnoise \
  --output outputs/terrain-profile-study/capture-20261006-v1-reviewed
rtk proxy python3 -B \
  outputs/terrain-profile-study/capture-20261006-v1-reviewed/inputs/study/study.py verify \
  outputs/terrain-profile-study/capture-20261006-v1-reviewed \
  --plan studies/terrain_profiles/plan.json
```

Capture refuses an existing destination. Each JSON contains the raw primary
polyline and reference grid. `plan.json`, source snapshots, `summary.json`, the
offline `index.html` gallery and an artifact-hash receipt preserve the experiment.
`overview.svg` and `refined.svg` are static review sheets derived from the same data.
Verification binds the independently supplied tracked plan and reconstructs
metrics/gallery from raw samples; it is not another generation or fresh repeat.
Generation source must still match the recorded hashes; changing it needs a new
capture, never an overwrite or relabel of old evidence.

The page is a profile gallery, not a replacement batch/detail flight template.
It stays outside the accepted report site; publishing navigation/server changes
needs a separate request. The stopping point is a reviewed source/recipe verdict
and a separately bounded next-flight-pass recommendation.

## Ridge-only refinement and offline pad inputs

The completed [refinement results](../../docs/terrain_ridge_refinement_results.md)
record the subsequent twelve-profile experiment. The original `plan.json`,
Python composition and all original capture bytes remain unchanged. The isolated
Rust helper now accepts an optional ridge slice offset, defaulting to zero; its
new source deliberately does not match the old receipt. Use the archived verifier
above for that historical capture, not a weakened current-source check.

The separate [refinement plan](refinement_plan.json) changes **only ridge-noise y**
from 67.0 to 67.37 for the same six seeds and two refined backends. It pins the
original control receipt, checks exact zero-offset compatibility before new
generation, requires a reversed-order fresh-process repeat and retains all
profiles. Landmark peak counts are diagnostic; no seed or recipe is tuned.

`refinement.py` reuses the original loader, composition, descriptors and plotting
helpers. Its local pad compiler uses the resolved 4 m polyline as authoritative,
with 36 m shelves and 24 m transitions at 0/1200 m. It modifies only
[−42,42]/[1158,1242] m, inserts exact boundaries and preserves outside vertices.
The [template](scenario_template.json) is an exact copy of a saved supported
scenario, not an accepted result for the new terrain. Vehicle, gravity, clocks,
mission and fuel defaults are unchanged. Raw/prepared data remain separate.

Build/check the helper with the commands above. Explicit profile capture:

```sh
rtk proxy python3 -B studies/terrain_profiles/refinement.py capture \
  --control-capture outputs/terrain-profile-study/capture-20261006-v1-reviewed \
  --pylander-root /home/bryan/code/pylander \
  --fastnoise-bin target/terrain-profile-study/release/pd-terrain-profile-fastnoise \
  --output outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final
rtk proxy python3 -B studies/terrain_profiles/refinement.py verify \
  outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final
```

Capture is create-only; an existing reviewed root must never be overwritten.
The old control capture is authenticated before executing its archived verifier.
New evidence includes captured generator sources, bound controls, 1/4 m arrays,
metrics, peak positions, before/after HTML/SVG and independent input receipts.

Review shapes before the separate create-only preparation step:

```sh
rtk proxy cargo build --release -p pd-cli --features planner-v2
rtk proxy python3 -B studies/terrain_profiles/refinement.py prepare \
  --capture outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final \
  --preflight-bin target/release/pd-cli \
  --output outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1
rtk proxy python3 -B studies/terrain_profiles/refinement.py verify-prepared \
  outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1 \
  --capture outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final
```

Preparation invokes only the existing CLI's `--preflight-only` mode. It creates
no simulator or flight bundle. It saves all twelve scenarios, raw inputs, fixed
patch geometry, native preflight JSON and a binary/artifact-hash receipt. Saved
verification reconstructs geometry and checks recorded preflights; it does not
execute generation or native validation again. Tests use synthetic data where
marked and do not require local captures/Pylander or establish native evidence.

No flight, native generator port, new controller, report-site publication or
server changes belong to this runner. The ordinary flight workspace and rich
flight report templates remain unchanged.

## Separate random-terrain flight survey

The [survey plan](../../docs/random_terrain_survey_plan.md) and
[frozen inputs/budget](survey_plan.json) define the explicit development
campaign. Its [stopped results](../../docs/random_terrain_survey_results.md)
retain seven measured attempts and a diagnosed initial replay-envelope roundoff
failure; 96 random cases remain unattempted. A new campaign requires a separate
decision, not rerunning the saved root. The [numerical fix/recheck plan](../../docs/random_terrain_survey_recheck_plan.md)
retains the same frozen sample and allowance. `survey.py` reuses this study's composition and local pad compiler,
but does not alter either historical study plan. It persists all 100 fresh seeds
before sampling, checks a fresh-process reversed generation repeat, validates
100 native input-only preflights, then freezes source/executable identities.

The [numerical-fix recheck results](../../docs/random_terrain_survey_recheck_results.md)
retain one measured control with a passing landing/integrity/replay tuple but
an exact-preservation mismatch in one clearance scalar. All 100 random cases
remain unattempted in that new capture; its create-only root must not be resumed.

The historical [comparison-aware follow-up](../../docs/random_terrain_survey_recheck_plan.md#comparison-aware-follow-up-authorized-2026-10-06)
uses a new source-bound [sentinel comparison contract](sentinel_comparison.json).
Only two named cycle-audit clearance scalars allow cross-source roundoff within
1e-12 m, with matching metadata and unchanged zero/required-reserve decisions.
Every exception is logged and verified in Python and Rust against shared
conformance cases. Historical captures retain their exact rule, and same-source
repeats remain exact apart from the three declared flight wall timings.
Its [stopped comparison-aware results](../../docs/random_terrain_survey_comparison_results.md)
retain two measured controls and zero random attempts. Selected local trajectory
diagnostics and their content-hash references remained outside that V1 rule;
do not resume that stopped root or silently broaden its captured contract.

The [2026-10-07 repair-and-retry continuation](../../docs/random_terrain_full_sweep_plan.md)
completed the [full frozen population](../../docs/random_terrain_full_sweep_results.md):
100/100 random verified direct landings, three controls (including actual one/three
handoffs), and five exact non-timing repeats. All random routes were clear, so
procedural blocked-path coverage remains a separate follow-up decision.
New preparation uses [comparison V2](sentinel_comparison_v2.json), which includes
selected-trajectory body-clearance diagnostics and independently authenticated
proposal/segment hash references. The read-only native `compare-terrain-flights`
command is shared by collection and report verification; runtime guards are not
relaxed. Historical absent/V1 contracts retain their original meaning.
The new owner authorization permits routine repair and retry in new source-frozen
captures; it does not permit restarting a stopped root or tuning frozen inputs.

Explicit input preparation (no flights):

```sh
rtk proxy cargo build --release -p pd-eval
rtk proxy python3 -B studies/terrain_profiles/survey.py prepare \
  --pylander-root /home/bryan/code/pylander --binary target/release/pd-eval \
  --sentinels outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  --output outputs/eval/planner_v2_random_terrain/NEW_CAPTURE
```

After source review and the maintained developer gate, execute that prepared
root once (at most 108 measured attempts, not another permission to run a batch):

```sh
rtk proxy python3 -B studies/terrain_profiles/survey.py run \
  --capture outputs/eval/planner_v2_random_terrain/NEW_CAPTURE \
  --binary target/release/pd-eval
rtk proxy python3 -B studies/terrain_profiles/survey.py verify \
  outputs/eval/planner_v2_random_terrain/NEW_CAPTURE
```

`run-start.json` is create-only, so the collector cannot silently restart an
existing campaign. Native full records, compact summaries, logs and per-attempt
ledgers are retained. All 100 primary identities remain accounted even on stop;
sentinels/repeats are separate. Saved verification is read-only and does not
rerun generation, planning or replay. Physical misses are data, not permission
to resample; implementation/integrity/replay/source failures pause new waves in
that capture. Under the continuation policy, repair routine defects and retry
in a new capture rather than abandoning the goal; unrecoverable safety or evidence
failures still require an explicit stop.
The presentation-only `pd-eval render-terrain-survey` command creates a separate
common batch/rich-detail site, never changes the accepted benchmark selector,
and never executes a mission. Site publication/navigation are explicit writes.

## Harder procedural-terrain challenge

The [challenge plan](../../docs/terrain_challenge_plan.md) and
[completed results](../../docs/terrain_challenge_results.md) extend the same
collector without changing the original sanity population. Four global recipes
scale vertical relief by 4/8 and, for the broad/successive variants, horizontal
dimensions by 1.5/0.6. `challenge.py` retains the Pylander composition, weighting,
ridge slice and local pad preparation; no per-seed obstacle placement or filtering.

The source-bound [calibration plan](challenge_calibration_plan.json) has 24 cases,
six per recipe, plus three controls. The separate [held-out plan](challenge_plan.json)
has 100 cases, 25 per recipe, plus three controls and five explicit repeats at
indices 0, 25, 50, 75 and 1. Both pools exclude sanity/shape seeds; held-out seeds
also exclude calibration. All recipes were retained unchanged after calibration.
This is independent held-out flight coverage, not a calibrated solvability quota.

For a separately authorized new capture, select the plan explicitly:

```sh
rtk proxy python3 -B studies/terrain_profiles/survey.py prepare \
  --plan studies/terrain_profiles/challenge_plan.json \
  --pylander-root /home/bryan/code/pylander --binary target/release/pd-eval \
  --sentinels outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  --output outputs/eval/planner_v2_random_terrain/NEW_CHALLENGE_CAPTURE
```

Use `challenge_calibration_plan.json` only for a separately authorized calibration
capture. Run/verify use the same commands above. Source binding, create-only
roots, strict proofs, four-process ceiling and repair-in-new-capture rule remain
unchanged. The native common report adds per-recipe conditional blocked/clear
outcomes; old captures retain their original plan/denominator semantics.

## Closed intervention-timing comparison

The [bounded timing plan](../../docs/intervention_timing_plan.md) reused 16
saved challenge scenarios for a logging-only baseline and one timing candidate.
The [results](../../docs/intervention_timing_results.md) reject the replacement:
four new verified landings came with a lost successful control and a CLI
regression. Previous timing is restored; compact query diagnostics remain.
No conditional 44-case capture, full challenge recheck or repeat was launched.

`intervention_pass.py` is a create-only saved-input collector, not a new terrain
generator or an active campaign instruction. Its frozen identities and allowance
are in [intervention_timing_plan.json](intervention_timing_plan.json). Read-only
verification of the two completed captures:

```sh
rtk proxy python3 -B studies/terrain_profiles/intervention_pass.py verify \
  outputs/eval/planner_v2_random_terrain/capture-intervention-20261007-diagnostics
rtk proxy python3 -B studies/terrain_profiles/intervention_pass.py verify \
  outputs/eval/planner_v2_random_terrain/capture-intervention-20261007-focus
```

These checks authenticate saved source, inputs, results and receipts; they do
not execute flights or claim that the captures came from the restored final
source. The normal report topic links both common-template comparison sites.
The subsequently executed failure-only fallback is a separate follow-up; the
closed replacement allowance does not permit invoking its conditional challenge.

## Completed failure-only timing fallback

The separate [fallback plan](../../docs/intervention_fallback_plan.md) and
[results](../../docs/intervention_fallback_results.md) close a 17-case focus,
44-case unpublished benchmark, unchanged 100-case challenge and two repeats.
The current search keeps successful primary choices and invokes the four-clock
fallback only on finite exhaustion. Challenge landings improve from 57 to 65
with no lost successes; the benchmark passes without replacing its accepted site.

[fallback_pass.py](fallback_pass.py) reuses the create-only saved-input collector;
its [frozen contract](intervention_fallback_plan.json) is separate from the closed
replacement experiment. The benchmark audit uses the existing sealed diagnostic
comparison contract and separate normalized copies, never edits raw captures.
Read-only verification commands live in the [evaluation workflow](../../docs/evaluation.md).
No new capture, report publication or reopening of this completed allowance is
implied by these historical instructions.

## Completed handoff braking-room preference

The [bounded plan](../../docs/handoff_room_plan.md) and
[results](../../docs/handoff_room_results.md) close eight logging-only shadow
cases, a 17-case preference focus, 44 unpublished benchmark cases, the unchanged
100-world challenge and two repeats. A negative-room original winner may be
replaced by the original-rank-best already accepted nonnegative-room row; entry
stages, family and guards stay unchanged. Challenge landings increase from 65
to 68, with all prior successes preserved. This is a scalar ranking heuristic,
not a landing suffix check or general recovery solver.

[handoff_room_pass.py](handoff_room_pass.py) reuses the create-only collector and
binds original input receipts separately from the current motion baseline in its
[frozen contract](handoff_room_plan.json). The benchmark audit needs exact
complete-flight preservation, without historical numerical exceptions. Saved
verification commands are in the [evaluation workflow](../../docs/evaluation.md).
No further campaign or publication is implied by the closed allowance.

## Fresh 1,000-case validation

The separate [validation plan](../../docs/terrain_validation_1k_plan.md) and
[source-bound contract](challenge_validation_1k_plan.json) retain the four
challenge recipes and current planner, with 250 fresh seeds per recipe. The seed
pool excludes all shape, sanity, calibration and development seeds. Three
preservation controls and five repeats are outside the 1000-case denominator.
This is a one-shot coverage check, not tuning or a paired planner comparison.

The [completed results](../../docs/terrain_validation_1k_results.md) record
733/1000 verified landings and 346/613 blocked-route landings. All 1008 attempts
pass verification; the allowance is closed. The retained commands below are
reproducibility instructions, not authorization for another run.

Preparation/run require explicit campaign authorization and a new capture:

```sh
rtk proxy python3 -B studies/terrain_profiles/survey.py prepare \
  --plan studies/terrain_profiles/challenge_validation_1k_plan.json \
  --pylander-root outputs/eval/planner_v2_random_terrain/capture-challenge-20261007-main-v2/inputs/reference \
  --binary target/release/pd-eval \
  --sentinels outputs/eval/planner_v2_lab_suite/capture-handoff-room-20261007-benchmark \
  --output outputs/eval/planner_v2_random_terrain/capture-validation-1k-20261007-v1
rtk proxy python3 -B studies/terrain_profiles/survey.py run \
  --capture outputs/eval/planner_v2_random_terrain/capture-validation-1k-20261007-v1 \
  --binary target/release/pd-eval
```

Use the [evaluation workflow](../../docs/evaluation.md#fresh-1000-case-terrain-validation)
for read-only Python/native saved verification. Native `check-terrain-survey`
checks the same evidence contract as rendering without publishing a site.
Finite misses remain data; source drift, collection errors or failed proofs stop
with evidence retained. No mid-campaign repair, replacement seeds or automatic
retry is authorized. Preserve earlier captures, accepted selectors and reports.

## Procedural-terrain capability diagnostics

The separate [selection plan](../../docs/terrain_diagnostics_plan.md) and
[frozen contract](diagnostics_plan.json) select ten exact saved 1k worlds:
two failures and a successful comparison each for near-pad departure, late
airborne acquisition and repeated short corrections, plus one clear-direct
control. The [checked-in inputs](../../fixtures/research/terrain_diagnostics_v1/README.md)
preserve original bytes/IDs and need no procedural generator or large saved
survey to use. This is deliberately selected development data, not a held-out
test or a population landing-rate estimate.

[diagnostics.py](diagnostics.py) reuses native preflight, full-flight collection,
common rich detail reports, complete landing projection and exact non-timing
repeat comparison. Baseline mode characterizes existing behavior with a fixed
13-attempt allowance. Explicit future candidate mode can accept repaired failures
without relaxing successful-comparison landing/integrity/replay requirements.
Run/verify instructions are maintained in the
[evaluation workflow](../../docs/evaluation.md#procedural-terrain-diagnostic-pack).
Neither mode publishes reports or changes the accepted selector. Pure/synthetic
tests run through the [development workflow](../../docs/development.md).

## Isolated correction-cap probe

The [bounded follow-up plan](../../docs/terrain_cap_probe_plan.md) and
[contract](cap_probe_plan.json) distinguish cap-truncated unfinished flights from
physical failures. Subjects `030/280` are extended with cap 12 and, only if that
cap still binds, cap 24. Each stage includes direct/six-handoff controls and two
exact repeats; maximum 12 measured attempts.
The [completed results](../../docs/terrain_cap_probe_results.md) record six cap-12
attempts: `030` lands after seven corrections, while `280` reaches a later
`NoClearing` after eight. No cap-24 stage ran and production stays at six.

[cap_probe.py](cap_probe.py) creates source-frozen diagnostic build copies,
changing only the copied cap and explicit probe identity. It does not relax the
production policy validator, alter tracked Rust/defaults, restore arbitrary
airborne state, publish reports or change prior captures. Exact prefix comparisons
prove the original commands, H states, samples, segments and next nominal audit
unchanged before the old cap boundary. Collection/read-only verification commands
are in the [evaluation workflow](../../docs/evaluation.md#isolated-correction-cap-diagnostic).

## Paired 1,000-world relaxed-cap sweep

[cap_sweep.py](cap_sweep.py) reuses isolated cap-24 construction, strict native
probe admission and exact prefix/control comparison from [cap_probe.py](cap_probe.py).
Its [separate plan](../../docs/terrain_cap_sweep_plan.md) and
[contract](cap_sweep_plan.json) rerun every original 1k input, three controls and
seven repeats. Original flights stay in their authenticated baseline capture;
the verifier requires that retained dependency. No generation, tuning, retry,
production-policy relaxation or publication is part of the sweep.
Collection and saved-verification commands are in the
[evaluation workflow](../../docs/evaluation.md#paired-1000-world-relaxed-cap-sweep).
The [completed results](../../docs/terrain_cap_sweep_results.md) record 748/1000
landings, fifteen added without losing any previous success, and no cap-bound
case. All 1010 attempts verify; production defaults and accepted reports remain
unchanged. This is paired development evidence, not fresh held-out validation.

## Bounded handoff timing/selection probes

[handoff_probe.py](handoff_probe.py) collects at most 19 fixed counterfactual
queries under the [plan](../../docs/terrain_handoff_probe_plan.md) and
[source/input/clock contract](handoff_probe_plan.json). The native diagnostic
reconstructs every query by forward execution from the original source and
reproves its consumed command/action prefix. It never restores saved snapshots
or changes the production flight session. Earlier query points are not relabeled
as executed/accepted waypoints; a clear nominal audit is not a new mission landing.
The [evaluation workflow](../../docs/evaluation.md#bounded-handoff-timingselection-diagnostic)
documents isolated build, explicit collection and read-only saved verification.
The ordinary release binary, accepted reports and current report site stay intact.
The [completed results](../../docs/terrain_handoff_probe_results.md) close all 19
queries and exact repeats: earlier same-maneuver states produce three clear and
two terrain-blocked nominals, while max-room alternatives produce two clear and
three missing nominals. No complete modified flight session is executed. The
[bounded early-exit pass](../../docs/terrain_early_exit_results.md) subsequently
executes three earlier target landings, preserves two blocked-query fallbacks
and passes the normal 44-case acceptance pack. This is selected-case execution,
not a rerun of the random population.

## Completed bounded early-exit execution

[early_exit.py](early_exit.py) owns the closed nine-flight allowance under the
[frozen contract](early_exit_plan.json). Its isolated cap-24 executable has a
distinct identity; the separately built acceptance executable uses normal cap 6.
The collector's saved-verifier fix did not change Rust, inputs or captured bytes
and did not rerun flights. See the [results](../../docs/terrain_early_exit_results.md)
for source and comparison boundaries.

Read-only verification of the original closed matrix:

```sh
rtk proxy python3 -B studies/terrain_profiles/early_exit.py verify \
  outputs/eval/planner_v2_random_terrain/capture-early-exit-20261007-v1
```

Do not invoke `run` again under this completed allowance. Paired 1k/fresh-100
campaigns, cap promotion and publication require separate authorization.

## Completed paired early-exit 1,000-world rerun

[early_exit_sweep.py](early_exit_sweep.py) uses a separate
[frozen contract](early_exit_sweep_plan.json) and the exact admitted cap-24
executable, without a build or planner/controller changes. The
[completed results](../../docs/terrain_early_exit_sweep_results.md) record
817/1000 landings, up from 748 with 69 gains and no losses; all four recipes
improve and all 387 clear direct flights remain unchanged. All 1010 unique
attempts, including three controls and seven repeats, verify. The initial
collector serialization error is retained separately: the final capture imports
seven verified executions without rerunning them, then executes the remaining
1003. Complete original input bytes stay pinned; native output scenario JSON is
checked by all parsed values without numeric tolerance.

Read-only saved verification launches no flights or publication:

```sh
rtk proxy python3 -B studies/terrain_profiles/early_exit_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/capture-early-exit-sweep-20261008-v2
```

## Fresh validation and rejected departure probe

The [completed pass](../../docs/terrain_departure_probe_results.md) follows its
[bounded plan](../../docs/terrain_departure_probe_plan.md): the exact retained
early-exit executable lands 77/100 genuinely new worlds, with all 35 clear routes,
42/65 blocked routes, three preserved controls and five exact repeats. The
[fresh collector](fresh_validation.py) and [contract](fresh_validation_plan.json)
keep this denominator separate from the old 1k development population. The
sample is now observed, not untouched for a later candidate.

The [departure experiment](departure_probe.py) and
[contract](departure_probe_plan.json) retain a source-sealed 13-mission negative
result: neither `327` nor `791` gains an admitted H from a fixed half-lift/
half-forward split. Other diagnostic records remain exact except root experiment
identity/timings. The rejected core runtime was removed; only saved verification
is maintained. Both verifiers are documented in the
[evaluation workflow](../../docs/evaluation.md#planner-evaluation-v2-default).
No broader campaign, production promotion or report-site publication followed.

The subsequent [departure clearance design](../../docs/terrain_departure_clearance_design.md)
and [proposed implementation](../../docs/terrain_departure_clearance_plan.md)
use the complete 46-world source-bridge failure cohort. The read-only
[analytical helper](departure_design.py) prints receipt-bound saved-geometry and
kinematic-screen results; it writes no capture and launches no native process.
These estimates are not accepted handoffs or landings. Its tests are included
in the ordinary terrain-study unittest discovery.

The authenticated previous cap-24 capture remains a dependency. Complete fallback
flights, original selected proposals/early executed prefixes, exact queued
nominals, repeat evidence and imported original ledgers are checked. This campaign
is closed; a fresh sample, default promotion or publication needs a separate
decision. No source or historical outcome is rewritten by saved verification.

## Ballistic feedback paired diagnostic

The completed opt-in [phase-transition pass](../../docs/ballistic_phase_transition_results.md)
records 797/1000 versus 786 (twelve gains, one loss), with exact repeats, focused
overlap and original inputs. Its [protocol](../../docs/ballistic_phase_transition_plan.md)
uses [a frozen collector](ballistic_phase_transition_panel.py) for a behavior-identical
probe, separate coast/entry/pad ablations, their combination and the original
1000 worlds. Every focused stage has 29 primary worlds and three repeats; the
full diagnostic has 1000 primary worlds and five repeats. Use `run` only when
authorized; `verify` reads saved evidence without publishing or rerunning flights.

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_phase_transition_panel.py verify full
```

Previous completed reference: the [finite-correction probe and 1k](../../docs/ballistic_finite_correction_results.md)
records 786/1000 versus 706 (109 gains, 29 losses). The
[collector](ballistic_finite_correction_panel.py) freezes one native candidate,
checks the actual powered acquisition, preserves the failed reader-only focus
capture, and verifies repeats, focused overlap and all original inputs. Its
[pass](../../docs/ballistic_finite_correction_plan.md) is complete; no promotion,
waypoint-ranking/recovery changes or new campaign starts automatically.

Read-only saved verification:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_finite_correction_panel.py verify full
```

Preceding separate diagnostic: the [frozen exit-check 1k result](../../docs/ballistic_exit_diagnostic_results.md)
records 706/1000 versus 639 (78 gains, eleven losses), with 1,005 complete records
and exact panel overlap. [Its collector](ballistic_exit_diagnostic.py) adds
diagnostic authority without weakening the old mechanics gate or changing native
flight behavior. `verify` is read-only; `run` creates the fixed new capture and
refuses an existing root. No publication or default change is included.
The [preceding design](../../docs/ballistic_correction_ownership_design.md) includes a
[read-only mathematical screen](ballistic_waypoint_energy_screen.py) of existing
waypoint fits. That tool never launches flights or writes files; its alternatives
are not terrain-audited/realized landing evidence.

The [results](../../docs/ballistic_feedback_sweep_results.md) retain 360/1000
landings from the unchanged opt-in construction candidate on the exact original
1k worlds, including 125 after waypoint handoffs. All 640 misses are finite
flying stops, not physical crashes. This is not acceptance or a new untouched
population. The original 817/1000 evidence, accepted site and selectors are
unchanged; the separate 100-world validation was not run.

The [collector](ballistic_feedback_sweep.py) and
[frozen contract](ballistic_feedback_sweep_plan.json) authenticate old inputs,
pin the existing candidate binary/source and record 1000 primary plus two exact
external repeats. Each native attempt also repeats decisions and replays commands.
The saved-only Rust example uses the common batch shell/tree/actual-flight preview
and rich detail links; it creates a local capture index without site publication.

```sh
rtk proxy python3 -B ballistic_feedback_sweep.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1
```

Run this command from `studies/terrain_profiles/`, or use the root-relative form
in the evaluation workflow. It verifies the sealed 1002-attempt capture without
running flights or changing artifacts. A new collection, candidate change or
publication requires a separate request; it is not an automatic next task.

### Completed bounded replan follow-up

The [bounded replan results](../../docs/ballistic_replan_results.md) retain the
first 381/1000 sweep and final 385/1000 guard-repaired sweep. The final source
preserves all 360 construction-candidate landings and gains 25. The
[first contract](ballistic_replan_sweep_plan.json),
[final contract](ballistic_replan_final_sweep_plan.json) and
[fixed panel runner](ballistic_replan_panel.py) are completed protocols, not new
flight allowances. Goal replacements/reacquisitions remain separate from actual
H; full candidate acceptance and publication remain deferred.

The same saved verifier recognizes each explicitly sealed contract without
weakening the original one:

```sh
rtk proxy python3 -B ballistic_feedback_sweep.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2
```

### Completed terrain-aware correction follow-up

The [terrain-correction results](../../docs/ballistic_terrain_correction_results.md)
record 401/1000 with all 385 previous successes exactly preserved. The
[sealed contract](ballistic_terrain_correction_sweep_plan.json) and
`ballistic_replan_panel.py --terrain-correction` are completed protocols, not new
flight allowances. Same-goal command recovery is separate from arc-blocked
waypoint selection and actual H. The full candidate remains unaccepted.

Saved verification is read-only:

```sh
rtk proxy python3 -B ballistic_feedback_sweep.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1
rtk proxy python3 -B ../../scripts/resume-ballistic-feedback-sweep.py --verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1
```

The capture retains four extra interrupted native invocations and their independent
exact recovery copies outside the 1k denominator. A separately sealed reader-only
same-clock H/recovery correction leaves all native source/binary/flight evidence
unchanged. See the results for the original error wave and append-only provenance.

### Completed waypoint-clearance mechanism panel

The [waypoint-clearance results](../../docs/ballistic_waypoint_clearance_results.md)
record 13/19 selected primary landings and two complete repeats. 807/928 pass
their original proposal-budget stops but still stop later; 484 now lands.
This is not a new 1k result. The dated create-only runner/finalizer retains the
display-only underscore-ID repair without rerunning any collected flight.
Its protocol is complete, not another active campaign allowance.

Read-only verification from this directory:

```sh
rtk proxy python3 -B waypoint_clearance_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-waypoint-clearance-panel-20261008-v1
```

### Completed ridge-placement experiment

The [pinned plan](../../docs/ballistic_ridge_waypoint_plan.md) and
[ridge-waypoint results](../../docs/ballistic_ridge_waypoint_results.md) complete
the two-flight probe and 21-record preservation panel. 807 lands with one actual
H, but three old random landings are lost: 8/16 versus 10/16, plus three exact
direct controls (11/19 versus 13/19 primary missions). No full 1k rerun, tuning,
retry or promotion follows. The geometry heuristic needs pass-through timing
diagnosis, not more terrain thresholds. This is a closed flight allowance.

The saved verifier needs neither current executable nor external captures:

```sh
rtk proxy python3 -B ridge_waypoint_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-ridge-waypoint-panel-20261008-v1
```

### Completed waypoint-entry ablations

The [pinned plan](../../docs/ballistic_waypoint_entry_plan.md) and
[waypoint-entry results](../../docs/ballistic_waypoint_entry_results.md) retain
four eight-record ablations and one 21-record combined preservation panel.
Effort and recovery independently land 349/006 without an apex constraint.
Combined remains 8/16 random landings, with three gains and three losses versus
V5; three direct controls stay exact. All 53 records verify, with no tuning,
retry, 1k campaign, default change or accepted-site publication.

The native experimental command exposes `--waypoint-experiment` with explicit
`ridge` (default), `effort`, `recovery` and `combined` identities. This does not
select a maintained planner policy. The common rich reports add the estimated
coast after burn cutoff without removing existing views.

Read-only verification from this directory, using copied sources/evidence:

```sh
rtk proxy python3 -B waypoint_entry_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-waypoint-entry-preservation-20261008-v1
```

### Completed local-height / early-target ablations

The [pinned plan](../../docs/ballistic_local_waypoint_plan.md) and
[results](../../docs/ballistic_local_waypoint_results.md) retain four eleven-record
comparisons and a 21-record combined preservation panel: all 65 records verify.
`early-target` alone lands 715; `local-height` alone and
`local-height-early-target` do not. The combined random panel falls from 8/16 to
6/16, losing 084/142, with three exact direct controls. Early-only has not run
the full preservation panel. The flight allowance is closed, with no tuning,
retry, 1k campaign, default change or accepted-site publication.

These V7 modes inherit V6 combined entry behavior. `combined` is the exact
same-source V6 control; experimental CLI `ridge` and maintained policy 3 remain
unchanged. The additive rich report identifies early destination goal changes
separately from actual H, and records local height versus incoming-corridor max.
`waypoint_entry_panel.py` supplies the shared proof/report runner through a frozen
`PanelSpec`; the earlier V6 runner retains its original default contract.

Read-only verification from this directory needs neither current executable nor
the previous capture:

```sh
rtk proxy python3 -B waypoint_local_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-waypoint-local-preservation-20261008-v1
```

### Completed failure-only landing duration

The [pinned protocol](../../docs/ballistic_landing_duration_plan.md) and
[results](../../docs/ballistic_landing_duration_results.md) close all 44 records:
nine same-source V7 controls, nine focused V8 records, five early-only controls
and a 21-record combined preservation pack. The new explicit `landing-duration`
mode opens terminal entry in 715/084/142 but they stop later under unchanged
guards. It gains 268 without losses, moving from 6/16 to 7/16 random landings.
All previous successful ordinary flights and direct controls stay exact.
`early-target-landing-duration` preserves successful early-only 715 exactly.
Neither maintained policy 3 nor experimental default `ridge` is changed.

The shared fallback queries exactly one finite 3–14 s duration only after all
existing latest-safe fits fail; it is used by entry and live descent. It does
not establish sustained braking when repeatedly recomputed with zero initial
net vertical acceleration. The countdown follow-up below tests that behavior,
not guard relaxation. No tuning, retry, broad campaign or publication occurred.

Read-only verification from this directory (no current native binary required):

```sh
rtk proxy python3 -B waypoint_landing_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-landing-duration-preservation-20261008-v1
rtk proxy python3 -B waypoint_landing_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-landing-duration-early-preservation-20261008-v1 \
  --mode early-target-landing-duration
```

### Completed retained landing countdown

The [protocol](../../docs/ballistic_landing_countdown_plan.md) and
[results](../../docs/ballistic_landing_countdown_results.md) close 48 records:
11 exact same-source V8 controls, 11 focused V9 records, five early-only
preservation records and the 21-record combined panel. The combined panel stays
7/16, with no new/lost landings. Twelve random ordinary flights, direct controls
and early-only successful 715 stay exact. 715/084/142/268 change; 268 still lands.

V9 retains the first selected fallback arrival and releases once on infeasibility
or expiry. It works as implemented, but 084 requires excess thrust later;
715 releases on tilt after the desired terminal-speed policy changes. Initial
acceleration admission is not complete-profile feasibility. Inspect that profile
on saved states before authorizing more flights. No tuning, retries, default
promotion, full 1k sweep or publication occurred.

Read-only verification from this directory (no current native binary required):

```sh
rtk proxy python3 -B waypoint_countdown_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-landing-countdown-preservation-20261009-v1
rtk proxy python3 -B waypoint_countdown_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-landing-countdown-early-preservation-20261009-v1 \
  --mode early-target-landing-countdown
```

### Completed automatic coast-to-terminal comparison

The [plan](../../docs/ballistic_coast_terminal_plan.md) and
[results](../../docs/ballistic_coast_terminal_results.md) close 22 fixed records:
11 exact same-source V9 controls and 11 explicit `coast-terminal` comparisons.
084 lands with H1 only, automatically preserving motion to a body-clear crest
checkpoint and entering standalone terminal control. No saved clock or W2 is
used. The selected random panel moves from 3/6 to 4/6 without losses; all other
ordinary flights and the three direct controls remain exact. Full 16-world/1k
coverage is not measured. Defaults and accepted site remain unchanged.

`coast_terminal_panel.py` reuses the sealed common panel runner. Its layout is
six selected random worlds, three direct controls and repeats of 715/084; it
rejects a wider panel. Admission checks current coast and bounded actual terminal
feedback, with configured controller terrain handling and live guards retained.
The two-second preview is not a complete landing suffix.

Read-only verification from this directory needs neither the current binary nor
the earlier capture:

```sh
rtk proxy python3 -B coast_terminal_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-coast-terminal-focus-20261009-v1
```

### Completed full preservation and paired coast-terminal sweep

The separate [validation plan](../../docs/ballistic_coast_terminal_validation_plan.md)
and [results](../../docs/ballistic_coast_terminal_validation_results.md) close
1023 records with the exact focused native binary: 21 full-panel records and
1002 conditional-sweep records. The panel lands 8/16 versus V9's 7/16 without
losses, permitting the sweep. The full candidate lands 562/1000 versus the last
complete ballistic sweep's 401, with 200 gains and 39 losses. This measures the
combined newer refinements, not an isolated coast increment or fresh sample.
Eight of nine worlds selecting coast land; 900 stops later under terrain reserve.

`coast_terminal_validation.py` owns the full frozen inventory and conditional
gate; the earlier focused runner remains unchanged. The
[receipt-pinned sweep contract](ballistic_coast_terminal_sweep_plan.json) reuses
the common sweep collector, copies authenticated gate evidence for portable
checking, and retains the original worlds, deadlines and cap 24. Native source,
guards, renderer, report navigation and accepted defaults/site are unchanged.
No retry, tuning or additional flight follows the closed allowance.

Read-only verification from this directory:

```sh
rtk proxy python3 -B coast_terminal_validation.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-coast-terminal-preservation-20261009-v1
rtk proxy python3 -B ballistic_feedback_sweep.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-coast-terminal-sweep-20261009-v1
```

### Closed focused terminal braking guard

The [plan](../../docs/ballistic_terminal_braking_plan.md) and
[results](../../docs/ballistic_terminal_braking_results.md) close four focus
records: 142 and 974 plus exact external repeats. The opt-in candidate lands 974;
142 slows substantially but stops beside the pad under unchanged body reserve.
The focus gate fails, so the conditional 21-record preservation panel did not run.
No new 1k, default promotion or accepted-site publication follows this result.

`terminal_braking_panel.py` reuses the common sealed collector and rich templates,
with a separate candidate identity and isolated native target. It refuses panel
execution after a failed focus or a source/binary/renderer change. Its completed
capture remains portable and read-only verifiable:

```sh
rtk proxy python3 -B terminal_braking_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-terminal-braking-focus-20261009-v1
```

The returned `passed: false` is acceptance failure, not an evidence failure.
The subsequent body-aware result below addresses that centering defect.

### Closed body-aware terminal centering pass

The [plan](../../docs/ballistic_terminal_centering_plan.md) and
[results](../../docs/ballistic_terminal_centering_results.md) close 25 records.
142 and 974 land with exact external repeats. The preservation panel improves
from 8/16 to 10/16 without previous-candidate losses; all three direct controls
land, and 084 retains its identical one-H flight. Their changed braking commands
nonetheless fail the declared exact direct-flight criterion. No new full 1k ran.

`terminal_centering_panel.py` reuses the sealed common collectors and rich report
templates. Its conditional full-sweep admission binds the focus and preservation
receipts to one native/source/renderer identity and refuses this failed panel.
Do not weaken the captured criterion, tune or retry it. Separate read-only
authentication verifies all records and receipts without promoting acceptance.

```sh
rtk proxy python3 -B terminal_centering_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-terminal-centering-focus-20261009-v1
rtk proxy python3 -B terminal_centering_panel.py verify \
  ../../outputs/eval/planner_v2_random_terrain/capture-terminal-centering-preservation-20261009-v1 --panel
```

The second command is expected to raise the exact-control mismatch. The practical
next decision is a separate outcome-based control contract and full-1k test of
the same frozen native candidate, not a new controller tweak. No defaults,
accepted-site navigation or server lifecycle changed.
