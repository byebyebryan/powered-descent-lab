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
