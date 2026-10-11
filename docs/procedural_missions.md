# Procedural missions and collections

The new distribution is declared here before inspecting any strategy's results.
It is separate from the frozen Pylander-derived studies and authored fixtures.
Those inputs and their outcomes keep their original identities.

The October 11 [challenge benchmark plan](procedural_challenge_plan.md) proposes
a new versioned distribution in which every qualification mission has a measured
terrain challenge. Its [input review](procedural_challenge_review.md) documents
the existing collections' milder geometry and pad-edge effects. The V1 contracts
below remain implemented and unchanged. The [V2 generator and bounded pads](procedural_generation_v2.md)
are implemented with explicit native/CLI/Python proposal frontdoors; V2 challenge
admission and collections are still under development. The neutral
[geometry contract](procedural_geometry.md) is implemented independently of V1
generation and provides physically bound, versioned coverage descriptors.

## Initial distribution: `pd-missions/value-noise-rest-v1`

The first tier is a complete source-pad-rest to target-pad mission with Earth
gravity, the current nominal 8 by 10 metre vehicle, 6,300 kg of fuel, 90 seconds,
120 Hz physics, 60 Hz commands and 10 Hz saved samples. All those fields appear
in the recipe, so future resource/start tiers need their own declared recipes.

The default recipe varies travel distance uniformly over 300–2,400 metres,
source position over -500–500 metres, and each pad width over 24–80 metres.
Four declared terrain strata use amplitude ranges 20–60, 60–140, 140–280 and
280–480 metres. Both travel directions are scheduled evenly by proposal index.
The stratum and direction schedule is part of the generator version; random
draws within a proposal depend only on recipe, collection seed, index and retry
number, not on iteration order or controller results.

Terrain uses seeded integer hashing and piecewise linear value noise at four
octaves with weights 8:4:2:1. The base wavelength varies over 320–960 metres.
Heights are quantized to millimetres; points are spaced eight metres apart with
at least 800 metres of terrain beyond each end of the transfer. Locally prepared
flat pads replace only points strictly within their own intervals and insert
flat endpoints. Terrain outside those intervals remains intact. No flight path,
waypoint or inter-pad corridor is authored or flattened.

Generation validates recipe bounds, finite terrain, distinct nonoverlapping
pads, physical scenario validity and clear source-pad rest geometry. Rejection
is structural only and has a finite declared proposal allowance. It does not
promise solvability. Rejected proposals and exhausted requests remain visible;
no controller result can admit or remove a mission.

## Layered family: `pd-missions/layered-perlin-rest-v1`

The second family keeps the physical distribution above and changes terrain
composition. Pinned FastNoiseLite 1.1.1 Perlin primitives produce macro relief,
three weighted regular/ridged octaves, domain warp and sparse pits, mesas and
depressions. Constants and seed arithmetic belong to this generator version;
the recipe's amplitude and wavelength scale the components. It is a native,
self-contained port of the earlier layered composition, with a distinct noise
backend identity. It does not claim exact Pylander noise equivalence.

Independent samples from the earlier input-only FastNoise study were recorded
before implementing this family. Negative and positive coordinate samples match
within 1e-10 metres before millimetre quantization. Ordinary tests use the new
recorded reference; generation never needs the study binary or a Pylander checkout.
The existing value-noise version retains a pinned resolved-byte/physical identity.

The October 10 learning launch characterized 25,000 requests per family and
repeated every result in reverse order. Both produced 25,000 distinct inputs,
zero exhausted requests and identical repeated receipts; value-noise had four
structural retries and layered terrain had six. Eight seed-disjoint collections
contain 4,096 unique physical inputs with no duplicates across cohorts. These
checks establish input reproducibility and separation, not flight feasibility.
The launch records are under `outputs/procedural-learning/run-20261010T190900Z/`.

## Collection contract

Collections contain resolved mission bytes, their SHA-256 identities, recipe
and proposal receipts, and a manifest with a declared usage role: training,
development, validation or held-out. Writers reserve a new directory and use
create-only files. Readers verify identities and physical scenario validity
without regenerating the terrain or consulting a solver.

Using a held-out collection for tuning consumes it. An append-only usage receipt
records that exposure rather than changing its original manifest or silently
reclassifying it. Recipe changes create new identities. Exact physical duplicate
missions are detected separately from seed/label differences.

The launch characterization used 10,000 scheduled requests (10,001 proposals,
including one structural retry) and a sealed
64-mission stratified development collection for integration. These are input
coverage and API/replay checks, not a landing-performance campaign.

## Local workflow

Run from the repository root; each output destination must be new and its parent
must already exist. These commands use neither saved flight data nor Pylander.

```sh
rtk proxy cargo run --release -p pd-missions --example missions -- generate --output outputs/my-missions --seed 1234 --count 64 --role development
rtk proxy cargo run --release -p pd-missions --example missions -- verify outputs/my-missions
rtk proxy cargo run --release -p pd-missions --example missions -- use outputs/my-missions --id integration-001 --exposure evaluation --purpose 'Consumed development integration inputs'
rtk proxy cargo run --release -p pd-missions --example missions -- characterize --output outputs/my-input-coverage.json --seed 4321 --count 10000
```

`--recipe PATH` accepts a complete serialized `MissionRecipeV1` in generation
and characterization. Collection manifests retain that recipe. Verification
reads the resolved bytes and receipts; it never reruns generation or flights.
`--family layered` selects the second default family; `--family value-noise`
selects the original. A family selector and a recipe file are mutually exclusive.

The first input characterization used seed `1723102026`. It produced 10,000
distinct physical inputs, zero exhausted requests and one structural retry,
then reproduced every result in reverse index order. Each of the eight
stratum/direction groups contained 1,250 inputs. Observed travel was 300.057–
2,399.956 metres; source and target widths covered approximately 24–80 metres.
Target elevations relative to source ranged from -487.945 to 544.177 metres,
and the largest inter-pad crest above the higher pad was 528.362 metres.
These ranges describe this sample; they do not establish flight feasibility.

The resolved 64-mission development collection uses seed `2026101091`. The
characterization and collection are retained under
`outputs/restructure/run-20261010T170648Z/` and will be consumed for integration.
