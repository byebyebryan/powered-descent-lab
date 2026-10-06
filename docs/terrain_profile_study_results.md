# Procedural terrain profile study — 2026-10-06

[Documentation home](README.md) · [Study contract and runner](../studies/terrain_profiles/README.md)

## Verdict

Keep **Pylander-inspired, scale-calibrated generation** as the preferred direction.
The refined arm produces useful hills, valleys and successive ridges at this
1.2 km scale without erosion, models, external height assets or planner feedback.
FastNoiseLite is a viable implementation option, not a demonstrated quality win.
Do not adopt another terrain engine or add erosion on this evidence.

This is a completed **shape and reproducibility study**, not a production-source
promotion, supported-pad check, planner evaluation or landing result. The accepted
44-case planner pack, flight workspace and published reports are unchanged.

## Frozen inputs and outputs

Six predeclared seeds (0, 1, 2, 7, 42, 12345), three arms, all **18 profiles retained**.
The [tracked plan](../studies/terrain_profiles/plan.json) records both complete
recipes, source hashes, source origin and descriptor/sampling thresholds before
generation. The unchanged Pylander class is evaluated directly from hash-bound
source; the two refined arms share one recipe and sparse-feature implementation.

The review span is 0…1200 m, with −160…1360 m raw halos and a fixed 137 m source
offset. Samples are saved at 4 m spacing plus a 1 m reference. Display alone
subtracts h(0); raw heights remain untouched. There is no per-profile rescaling.

The complete create-only capture is
`outputs/terrain-profile-study/capture-20261006-v1-reviewed`:

- [Offline gallery](../outputs/terrain-profile-study/capture-20261006-v1-reviewed/index.html)
- [Shared-scale overview](../outputs/terrain-profile-study/capture-20261006-v1-reviewed/overview.svg)
- [Refined-arm overlay](../outputs/terrain-profile-study/capture-20261006-v1-reviewed/refined.svg)
- [Metrics](../outputs/terrain-profile-study/capture-20261006-v1-reviewed/summary.json)
- [Source/artifact receipt](../outputs/terrain-profile-study/capture-20261006-v1-reviewed/receipt.json)

Plan SHA-256: `82665074d99180549a03ee5d4716ff29cac7379230eab6e10e01b4dace5f77f5`.
Fresh-process, reversed-seed/reversed-coordinate full-reference hash:
`b0dacbcdcec0c188e7ea6792218750ddea2171e9da096beeabb85b5b1ee319a2`.

## Measured comparison

Ranges below are across six seeds within each arm. They are not reliability
rates, population estimates or universal backend rankings.

| Arm | Relief (m) | Endpoint rise (m) | Maximum slope (m/m) | Worst 4 m interpolation error (m) |
| --- | --- | --- | --- | --- |
| Pylander defaults | 209.97…1188.01 | −1188.01…−99.69 | 1.215…2.346 | 0.838 |
| Pylander refined | 58.96…103.73 | −67.02…+56.16 | 0.774…0.880 | 1.135 |
| FastNoiseLite / Perlin | 62.77…118.88 | −73.05…+1.86 | 0.645…1.026 | 1.434 |

The baseline shows large relief and all six endpoints below their starts in
this fixed crop. That is a property of these samples, not proof that Pylander
cannot generate uphill terrain. The refined arm includes rising, falling and
nearly level endpoints with more appropriately scaled intermediate landforms.
The FastNoiseLite arm produces comparable relief but different shapes; the
same seed does not define the same world across noise implementations.

Scale and weighting were deliberately bundled. This comparison cannot isolate
their causal effects or establish that weighting alone improved the terrain.

## Remaining issues and simplification decisions

### Recurring pointed-ridge structure

Static inspection of the rendered refined overlays finds recurring ridge
positions across seeds. This is consistent with the recipe sampling the noise
at integer second coordinates and folding zero crossings with `abs`: gradient
noise vanishes at lattice intersections, encouraging phase-aligned ridge peaks.
The small warp moves them but does not remove the pattern. This is a recipe issue
shared by both backends, not evidence that another noise library will fix it.

A fixed non-integer slice offset is the simplest next hypothesis. Test it in a
separate, predeclared comparison without adding regional graphs or erosion. The
current 18 profiles and recipes must remain unchanged.

### Sampling warnings, not hidden physics failures

Against the 1 m reference, the predeclared 0.5 m warning is exceeded by all six
baseline profiles, all six refined profiles and five FastNoiseLite profiles.
At 8 m spacing, worst errors rise to 1.519, 2.594 and 2.643 m respectively. These
warnings are retained; no seed is discarded and no threshold is changed.

The 1 m reference is itself sampled, not a mathematical continuous-error bound.
Before flights, explicitly decide whether the 4 m **resolved polyline is the
authoritative game terrain**. If so, planner, physics and renderer must all use
that same line; a finer visual/query surface cannot silently add missing peaks.
That is simpler than adaptive meshing and is plausible for a gamified source.
If fidelity to the continuous generator is required instead, sampling needs a
separately justified denser or feature-aware strategy. No decision here proves
that the chosen vehicle can fly these routes.

### Feature and descriptor limits

Additive plateau offsets still do not flatten the underlying surface. That
feature is unchanged, not marketed as a true flat plateau. Local turn counts
use adjacent-extrema prominence and half-width thresholds; a zero count does
not mean no hills, no terrain conflicts or an easy flight. Do not use those
descriptors to choose nominal arcs or label planner obstacles.

## Validation and review

- 12 Python tests pass: sampling/metrics, feature-width descriptors, display-only
  translation, create-only output, independent plan/inventory binding and
  derived-result tampering (including a rehashed forged summary).
- Three Rust tests pass; format and strict all-target Clippy pass for the isolated
  helper. The helper is not a member or dependency of the flight workspace.
- All 18 profiles are finite and grid-bound. Their full reference arrays match
  a fresh Python process with reversed seed and coordinate order; FastNoiseLite
  is a fresh subprocess on each pass. Every Rust sparse-feature sample matches
  the actual Pylander feature function to 1e-10 m.
- Saved verification reconstructs metrics, both SVG review sheets and the HTML
  gallery from raw samples, checks current generator source and artifact hashes,
  and binds the independently supplied tracked plan. This read-only check is
  not itself another fresh generation.
- Both SVG review sheets were rasterised with librsvg and visually inspected
  by the agent. This is static figure QA, not browser interaction or user visual
  acceptance. The page uses offline inline SVG and no network resources.

The first capture stopped at the cross-language sparse-feature check because
the initial Rust hash translation omitted an additive constant. A golden hash
regression test and per-sample comparison close that mechanical defect. The
incomplete root `outputs/terrain-profile-study/capture-20261006-v1` is retained;
it has input snapshots but no accepted profile summary. No recipe was tuned in
response, and the final capture is separate/create-only.

Final verifier review also hardened exact source-key and required-artifact
inventories, including binding source snapshots to current generator files.
The pre-review `capture-20261006-v1-final` root is retained. The reviewed capture
regenerated the same plan, all 18 identities and repeats: plan, summary, HTML
and both SVGs are byte-identical. This was a guard-only integrity change, not a
new terrain recipe or outcome-dependent retry. Thirteen relevant Node docs/gate
tests and the documentation link/whitespace checks also pass. No full flight
workspace gate or numerical flight recapture was run because flight code is
unchanged and this goal excluded flight execution.

## Recommended next bounded pass

This recommendation was subsequently executed as the separate
[ridge-slice refinement](terrain_ridge_refinement_results.md). Its evidence and
input-only preflights do not alter this original study's results or create flights.

1. Keep this study as the frozen control. Test only a non-integer slice offset
   on the two refined arms: six fixed seeds each, same physical coefficients,
   no backend search or outcome-based seed/crop selection.
2. Review whether the repeated ridge motif is reduced. Keep source selection
   provisional until that review; Pylander-inspired composition remains the base.
3. Define the authoritative resolved-polyline rule and narrow, explicitly
   recorded flat-pad preparation. Do not clear the flight corridor or reshape
   terrain around a nominal trajectory.
4. Only after that input review, propose a separate small flight pack with
   ordinary controls, uphill/downhill/compound profiles, explicit finite-stop
   expectations and the current vehicle/gravity/rates. Preserve every miss and
   the existing 44-case pack. No flight run, planner tuning, report publication,
   commit or push is authorised by this result record.
