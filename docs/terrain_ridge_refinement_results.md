# Ridge-slice terrain refinement — 2026-10-06

[Documentation home](README.md) · [Original profile study](terrain_profile_study_results.md) · [Runner](../studies/terrain_profiles/README.md#ridge-only-refinement-and-offline-pad-inputs)

## Verdict

Keep the Pylander-inspired recipe and the **ridge-only fractional slice** for
the next development flight pass. Changing ridge-noise y from 67.0 to 67.37
substantially reduces the shared ridge placement in both tested backends while
retaining hills, valleys and endpoint elevation differences at the intended
1.2 km scale. No erosion, new noise algorithm, region graph or parameter search
is warranted by this result. FastNoiseLite remains an implementation option,
not a demonstrated universal quality winner.

This closes the bounded **shape/input refinement**, not random-terrain flight
coverage or runtime-source promotion. There were **zero flights**. Native
preflight acceptance means the inputs meet the supported request contract;
it does not establish a feasible nominal, a clearing maneuver or a landing.
The existing 44-case pack, planner/controller code and report site are unchanged.

## Frozen experiment and retained evidence

The [refinement plan](../studies/terrain_profiles/refinement_plan.json) binds
the unchanged original recipe, the reviewed control receipt and an exact saved
scenario template. It fixes the ridge offset at 0.37, both existing refined
backends, all six original seeds, the two peak landmarks and the pad dimensions
before generation. Macro/ordinary noise, warp, sparse features, weighting,
amplitudes, frequencies, crop, source offset and all grids remain unchanged.
The Python arm wraps only ridge queries and reuses the original weighted formula.
The isolated Rust helper defaults an omitted ridge offset to zero.

Twelve new profiles are compared to twelve independently bound original refined
controls. The original 18-profile capture is preserved. All twelve regenerated
zero-offset full-reference profiles match their controls exactly; the new twelve
match a fresh-process repeat with reversed seed/coordinate ordering exactly.
Every Rust sparse-feature sample still matches the actual Pylander function to
1e-10 m. No profile or seed was discarded, rescaled or selected by flight outcome.

Canonical shape capture:
`outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final`:

- [Offline before/after gallery](../outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final/index.html)
- [Shared-scale review sheet](../outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final/overview.svg)
- [All metrics and peak positions](../outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final/summary.json)
- [Source/artifact receipt](../outputs/terrain-profile-study/capture-20261006-ridge-slice-v1-final/receipt.json)

Plan SHA-256:
`7c658f421809c0d34f135220f2a7437713706f1df167d4be55cecc0fc103ae0b`.
New full-reference repeat SHA-256:
`67024109da292c1bf6a7582a224bdd390f55aa6a7505353fa1028548a678d50e`.

The pre-review `capture-20261006-ridge-slice-v1` and
`capture-20261006-ridge-slice-v1-reviewed` roots are retained. Static figure QA
found a clipped final axis label; the first text-matching fix missed the renderer's
two-decimal formatting, which its added unit test caught. The corrected final
capture keeps exactly the same plan, all twelve profile files and summary bytes.
Only presentation/guard source and derived figure/page provenance changed. These
were not recipe retries. No earlier source receipt is relabelled as final source.

## Shape results

The predeclared descriptor counts a local peak on the unchanged 1 m reference
within 25 m of each landmark. It has no prominence filter and is diagnostic,
not a planner obstacle label or automated aesthetic gate.

| Backend | Near 463 m, before → after | Near 1063 m, before → after |
| --- | --- | --- |
| Pylander primitive | 6/6 → 2/6 | 6/6 → 0/6 |
| FastNoiseLite / Perlin | 6/6 → 1/6 | 6/6 → 0/6 |

The original integer slices force gradient-noise zeros at lattice crossings;
absolute-value ridging turns those into peaks, and harmonic octaves reinforce
their alignment. Prior point diagnostics matched the predicted warped crossings
to the saved original peaks within the 1 m grid. The intervention's results are
consistent with that mechanism. They do not prove all directional/lattice
artifacts have disappeared or that an arbitrary fractional offset is optimal.

| Backend | New relief (m) | New endpoint rise (m) | Worst 4 m loss, before → after (m) | 0.5 m warnings, before → after |
| --- | --- | --- | --- | --- |
| Pylander primitive | 63.98…86.70 | −77.79…+61.32 | 1.135 → 0.721 | 6/6 → 2/6 |
| FastNoiseLite / Perlin | 67.03…108.54 | −79.81…+18.89 | 1.434 → 0.613 | 5/6 → 1/6 |

New maximum-slope ranges are 0.463…0.966 and 0.460…0.886 m/m respectively.
Worst 8 m losses are 1.756 and 1.546 m. Sharp ridge creases still exist; the
offset addresses placement, not smoothness. The unchanged warning remains
descriptive, not permission to drop seeds or tune resolution. The 1 m reference
is itself sampled and supplies no mathematical continuous-error bound.

Primary-agent static review of the final rendered sheet finds less repeated
pointed structure and adequate visible variety for the proposed gamified study.
It is not user visual acceptance or evidence of physical flyability. This fixed
six-seed crop is not an unbiased terrain-population sample; in particular, five
Pylander profiles end downhill. No seeds were replaced to balance the gallery.

## Authoritative surface and fixed local pads

The actual game/evaluation surface is the **resolved piecewise-linear polyline**,
not a hidden finer generator query. The 1 m arrays remain generation diagnostics.
The existing `pd-core` heightfield/query implementation and report adapters
already consume resolved vertices. No flight-consumer implementation changed.

The offline compiler makes twelve candidate scenario files, one for every new
profile, in a separate create-only input capture:
`outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1`:

- [Input summary and exact patch geometry](../outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1/summary.json)
- [Input/preflight receipt](../outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1/receipt.json)
- Example [raw profile](../outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1/raw/refined-seed-42.json),
  [prepared scenario](../outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1/scenarios/refined-seed-42.json)
  and [native preflight](../outputs/terrain-profile-study/prepared-20261006-ridge-slice-v1/preflight/refined-seed-42.json).

Source/target centres are 0/1200 m. Each shelf is 36 m wide, at the original
resolved height at its centre. Each side has a fixed 24 m linear transition
to the original surface: edit intervals are **[−42,42] and [1158,1242] m**.
Original vertices outside those patches are preserved exactly; inserted outer
anchors are collinear with the original line, preserving its geometry outside.
No trajectory is considered and no flight corridor is cleared. Patch preparation
can raise or lower terrain locally; the changes are explicit, not a hidden cutaway.
The largest recorded local vertex change is 7.490 m across all twelve inputs.

Each prepared line retains the 381 original 4 m-grid vertices and inserts eight
exact shelf/patch boundaries: 389 ordered finite points over the same halo domain.
Those extra boundary vertices are part of the authoritative line. Raw and
prepared data remain distinct and individually hashed. The compiler keeps the
tested vehicle, Earth gravity, 120/60 Hz clocks, 90 s scenario horizon and
route-free landing mission; it sets upright source rest at the new pad height.

All **12/12 real CLI preflights** are supported. The runner can only invoke
`--preflight-only`, supplies no output directory, and requires policy 3, zero
corrections and null planning/physical/mission/integrity/replay outcomes.
The native preflight binary is hashed before/after validation. No simulator,
flight bundle, rich flight report or accepted batch publication was created.

## Validation and scope

- 25 Python tests pass, including original-study checks, ridge-only/default
  behavior, strict polyline queries, exact shelves, unchanged outside vertices,
  template preservation, flight-free preflight guards and forged/rehashed
  comparison/scenario rejection. Synthetic tests are not native flight evidence.
- Four isolated Rust tests, formatting and strict all-target Clippy pass.
- Current-source saved verification reconstructs both the comparison and fixed
  pad scenarios; this is not fresh generation or repeated native preflight.
- The original study still verifies with its preserved archived tool. Its old
  receipt correctly does not match the helper's newly changed source.
- The final SVG was rasterised and inspected; this is static QA, not browser
  interaction or user acceptance. All twelve final profile/summary bytes match
  the first numerical capture despite the axis-label correction.
- Thirteen relevant Node documentation/gate tests pass. The documentation check
  covers 102 files and 560 local links, with 48 generated-evidence links skipped
  and no issues; diff and new-file whitespace checks pass.
- Cargo metadata still reports the six ordinary flight crates. Original plan
  and control-receipt hashes are unchanged; the Pylander checkout remains clean.

No full flight gate, parity campaign or planner run is part of this goal. The
isolated helper remains outside the ordinary six-crate flight workspace.

## Next bounded flight recommendation — not executed

Stop refining noise for now. First review the shape gallery, then propose a
separate development flight pack using **all six Pylander-offset scenarios**,
not only attractive or apparently easy ones. Keep the FastNoiseLite scenarios
as implementation comparisons rather than doubling the initial flight matrix.

Include three uncut simple 1200 m controls (flat, uphill and downhill) to separate
the new span/endpoint-height envelope from procedural obstructions. Preserve a
small set of accepted direct/corrected/multi-handoff sentinels before the new
attempts. Freeze every input, expectation, attempt limit and stop rule before
flight execution; keep the accepted 44-case pack unchanged.

Report nominal terrain-audit decisions, actual correction counts, planning stop,
physical/mission outcomes, integrity, final-source replay and time separately.
Predeclare that a crash or proof/integrity failure stops the pass for diagnosis;
a typed finite planning stop is coverage information, not physical impossibility.
No in-pass recipe/policy tuning, seed substitution or terrain-aware higher
nominal retry. The six shapes are already inspected development inputs, not
held-out random-terrain reliability evidence.

The current amplitude may yield mostly direct landings; do not force waypoints
by rescaling these cases after results. If so, that is useful clear-terrain
coverage, not expanded correction coverage. Any harder procedural regime needs
a separately frozen follow-up. Ordinary sharp crests and imperfect landings do
not by themselves justify another noise/erosion research loop.
