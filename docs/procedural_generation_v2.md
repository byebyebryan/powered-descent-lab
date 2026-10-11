# Configurable procedural generation V2

[Mission workflow](procedural_missions.md) · [Geometry](procedural_geometry.md) ·
[Implementation loop](procedural_challenge_execution.md)

`pd-missions/configurable-rest-v2` is a native, explicit schema-2 physical
generator. It produces complete source-rest missions, with no authored routes,
morphology schedule or outcome input. A returned proposal establishes structural
validity only. Challenge admission and collection management are separate layers;
this generator alone is not a qualified benchmark or a solvability claim.

## Default physical distribution

The default keeps the nominal 8x10 m vehicle, 6,300 kg fuel, Earth gravity,
90 s horizon, 120/60 Hz physics/commands and 10 Hz saved samples. Draw source x
uniformly over -500–500 m, transfer distance over 300–2,400 m, and both pad widths
independently over 24–80 m. Even proposal indices travel right; odd indices travel
left. Direction does not define a morphology or a controller selection.

Each request has at most eight structural attempts. Seed, index and attempt
determine local random draws, so reversal/parallel execution retains identical
results. Every failed structural attempt is returned, including complete
exhaustion. No retry depends on terrain challenge, classification or a flight.
Changing geometry specifications leaves the physical input unchanged.

Layered Perlin is the default backend; `value_noise` is an explicitly selected
second primitive backend using the same composition controls. V2 value noise is
a new composition, not the frozen four-octave V1 value-noise generator. Perlin
uses pinned FastNoiseLite 1.1.1 primitives, and does not reproduce Pylander seeds.
The backend is provenance, not a qualification or morphology label.

| Continuously drawn control | Default range |
| --- | --- |
| Macro amplitude | 240–960 m |
| Macro wavelength | 1,200–3,600 m |
| Structure amplitude | 420–1,680 m |
| Structure wavelength | 300–1,000 m |
| Ridge mixing | 0.10–0.85 |
| Octave gain | 0.25–0.55 |
| Lacunarity | 1.60–2.40 |
| Weighted strength | 0–0.80 |
| Warp amplitude | 0–120 m |
| Warp wavelength | 500–2,000 m |
| Local-feature amplitude scale | 40–240 m |
| Local-feature cell size | 180–720 m |
| Local-feature density | 0.05–0.60 |

The default has three octaves; recipes can explicitly select one through four,
within sampling limits. Macro and structure scales vary independently. Local
features are sparse pits, mesas and depressions; their peak heights are fractions
of the amplitude scale, not guaranteed feature heights. Value noise interpolates
seeded hash values linearly along each component's slice; Perlin uses grid
gradients. Both have regular/ridged weighted octaves, warp and local features.
Measure actual geometry rather than interpreting control values as difficulty.

Heights are quantized to millimetres; mission positions, widths and composition
controls are continuous floating-point draws. Resolve a 4 m global-grid polyline
with at least 800 m context beyond both pads. It is the authoritative physical
terrain. Recipe validation requires the smallest nominal octave wavelength and
feature radius to span at least four samples; that guard does not prove absence
of interpolation error or preserve every warped detail.

## Raw terrain and bounded local edits

Retain the full raw polyline and its serialized native SHA-256. Pad elevation is
the millimetre-rounded raw height at its center. Replace each flat shelf plus a
24 m transition on each side; transitions connect linearly to raw boundary
heights. Retain every raw vertex and the exact polyline geometry outside the
recorded edit intervals. Do not clear an inter-pad corridor.

An outer edit boundary within 1e-6 m of a raw vertex snaps to that vertex and the
snapped interval is recorded. This avoids accidental tiny connections caused
by floating-point grid alignment. Preparation rejects domain overrun and
overlapping edit intervals. Recipes bound pad widths, transition size and total
sampling work; the default intervals remain small relative to the mission.

Resolved records retain recipe/identity, proposal, realized controls, terrain
seed, raw polyline/identity, raw geometry, prepared physical scenario/identity
and a [geometry receipt](procedural_geometry.md). Readers verify the declared
raw domain/grid, physical tier, bounded preparation and both metric projections.
They do not regenerate noise or consult a controller. Immutable file/manifest
bindings belong to the collection layer; these hashes are not signatures.

`ResolvedMissionV2::raw_reference(1.0)` explicitly regenerates a finer input-QA
sample from the recorded recipe/controls/terrain seed over the same domain.
It retains the sealed 4 m physical input. Compare interpolation errors and
raw/prepared measurements during calibration; sampled agreement is not a
continuous-terrain proof. QA has a 100,001-point limit and fails explicitly when
a requested finer domain exceeds that bound.

## Versioned frontdoors

`VersionedMissionRecipe::from_json` and `VersionedResolvedMission::from_json`
dispatch on explicit integer `schema_version`. Both versions keep strict schemas;
V1 fields are not reinterpreted as V2 controls. Serialized versioned results keep
the original per-version JSON shape. V1 byte identities and aliases are pinned.

From the repository root:

```sh
rtk proxy cargo run --release -p pd-missions --example missions -- recipe --family layered-v2
rtk proxy cargo run --release -p pd-missions --example missions -- propose --family layered-v2 --seed 1234 --index 0
rtk proxy cargo run --release -p pd-missions --example missions -- propose --recipe /path/to/recipe-v2.json --seed 1234 --index 1
```

`recipe` and `propose` print JSON to stdout without creating collections or
writing files. Aliases `layered` and `value-noise` retain V1; `layered-v2` and
`value-noise-v2` select V2. `propose --geometry-spec PATH` supplies a complete
`GeometrySpecV1` for V2. V1 rejects that option rather than ignoring it.
Existing `generate`/`characterize` collection commands still consume V1; V2
collection frontdoors are the next implementation slice.

The thin Python bridge retains `generate_mission(family, seed, index)` with
unchanged V1 aliases/results and explicit new V2 aliases. Its new
`generate_mission_from_recipe(recipe_json, seed, index, geometry_spec_json=None)`
uses the same native dispatch. Learning callers own sampling and rewards;
the bridge introduces no incumbent fallback or control policy.

Validation includes actual Python calls matching native V1/V2 generation JSON,
and captured native/Python episode equality under identical V2 scenarios and
commands. Those small command-script tests establish interface parity, not
controller performance, benchmark qualification or a training allocation.
