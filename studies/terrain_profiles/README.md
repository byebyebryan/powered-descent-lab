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
