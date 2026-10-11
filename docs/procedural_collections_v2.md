# Challenging procedural collections V2

[Mission workflow](procedural_missions.md) · [V2 generation](procedural_generation_v2.md) ·
[Geometry](procedural_geometry.md) · [Implementation loop](procedural_challenge_execution.md)

V2 collections select complete configurable proposals without a controller,
route, checkpoint, reward or outcome input. This is an implemented input contract.
The default geometric rule remains a development candidate after the
[completed input pilot](procedural_challenge_calibration.md); freeze is separate
work. Meeting its thresholds does not prove solvability, universal
difficulty or complete terrain coverage.

## Admission and labels

Default `pd-missions/challenge-v1` admission requires at least one path to hold
in **both raw and prepared geometry**:

| Path | Required measured demand |
| --- | --- |
| Sustained obstruction | Interval at least 80 m wide at 100 m above the pad-center chord |
| Elevation transition | Absolute target minus source pad rise at least 300 m |
| Repeated crests | At least two crests, each at least 50 m above the chord, 80 m prominent and 40 m wide at half prominence, spaced at least 80 m apart |

The same path must survive preparation. A prepared-only feature cannot qualify
a mild raw proposal. This remains a proxy: a sloped downhill transfer can meet
the elevation path without an inter-pad obstruction; its dynamics need separate
flight evidence. Calibration must expose pad edits and raw/prepared differences.

Crests follow the [geometry contract](procedural_geometry.md); prepared crests
whose half-prominence supports overlap a pad edit are already excluded. Filter
by height, prominence and width, then scan source-relative order with the declared
spacing. Earliest-first spacing maximizes the retained count among those filtered
crests. Equality passes thresholds. Validation requires observable metrics and
positive minimum demands; threshold changes create new declarations/identities.

Separate `pd-missions/morphology-v1` labels use prepared geometry:

| Label | Default condition |
| --- | --- |
| Mountain crossing | Chord barrier at least 200 m |
| Broad massif | Interval at least 300 m wide at 100 m above the chord |
| Successive ridges | At least three above-chord crests with prominence at least 80 m |
| Elevation transition | Absolute pad rise at least 400 m |

Keep all matching labels in stable order. Empty labels mean unclassified;
several labels mean mixed. Neither removes a qualifying proposal. Changing only
this projection changes the declaration and labels, but leaves broad accepted
physical membership unchanged. Labels do not reach the agent decision interface.

## Three declared populations

- **Broad** retains every unique qualifying proposal in the bounded stream,
  without morphology or geometry-bin quotas. Admission changes the continuous
  proposal distribution; the pilot must expose that change.
- **Stress** declares up to sixteen named panels with inclusive neutral geometry
  constraints and quotas. Qualifying unique proposals fill matching panels in
  canonical index order. One mission can fill several quotas; the union is the
  unique denominator. Receipts distinguish matching and selected memberships,
  including unmatched/capacity exclusions. Missing obstacle/spacing values never
  match numeric constraints.
- **Shift** binds a complete reference recipe and a hypothesis. Its recipe must
  use another backend or a strictly disjoint travel/width/terrain control range.
  Fresh seeds alone are insufficient. Admission retains all unique qualifying
  proposals, as in broad sampling. The declaration establishes a recipe difference,
  not that a policy avoided exposure to that distribution.

Keep these populations in separate collections and performance denominators.
Reference recipes cannot certify an agent's exposure history. Overlapping panel
and morphology counts are not additive mission counts.

## Immutable accounting

`CollectionRequestV2` retains role, full recipe, metric spec, challenge rule,
morphology spec, population, seed and indices. Writers sort schedules before
reserving output; sealed schedules have 1–20,000 ascending distinct indices.
Duplicate winners and stress decisions use that order independently of workers;
generation itself is index-local.

Reserve a fresh directory, write `request.json`, retain every structurally resolved
input as `proposal-<20-digit-index>.json`, then seal `manifest.json`. Keep mild,
duplicate and stress-excluded inputs for audits. Each index has one terminal receipt:

- accepted;
- insufficient challenge;
- physical duplicate, naming the first qualifying index;
- structural exhaustion, retaining every bounded failed attempt;
- stress exclusion.

Receipts bind structural attempt prefixes, file/byte and physical identities,
raw/prepared challenge measurements, labels and stress memberships. The manifest
reconciles counts and the exact accepted entry list. Deduplication follows
qualification and precedes stress capacity selection; repeats cannot fill another
quota. Write failure leaves an incomplete reserved directory and returns an error;
the writer does not overwrite it or present it as sealed.

`read_collection_manifest_v2` checks the bounded declaration, canonical schedule,
attempt prefixes, complete selection/duplicate/quota accounting and entry list.
`load_collection_proposal_v2` verifies an admitted **or excluded** input, physical
scenario, raw/pad preparation and derived geometry/selection projections.
`verify_collection_v2` does this for every resolved proposal without regenerating
noise. A manifest read alone is not full input verification. Regular-file limits,
SHA-256 bindings and create-only writes preserve evidence; hashes are not
signatures or independent generator provenance.

V1 writer, schemas, scheduling and duplicate-reporting semantics remain unchanged.
Explicit `read_versioned_collection_manifest`, `load_versioned_collection_mission`
and `verify_versioned_collection` dispatch on integer schema headers. Usage
receipts keep their schema and create-only semantics for both versions. Recorded
inspection/tuning does not silently restore a collection's held-out role.

## Frontdoors and consumption

Serialize a complete `CollectionRequestV2`. In Rust,
`CollectionRequestV2::broad(recipe, seed, count)` supplies development defaults;
edit and validate the declaration before saving it. From the root:

```sh
rtk proxy cargo run --release -p pd-missions --example missions -- collect --request /path/to/request-v2.json --output outputs/my-challenging-collection
rtk proxy cargo run --release -p pd-missions --example missions -- verify outputs/my-challenging-collection
rtk proxy cargo run --release -p pd-missions --example missions -- use outputs/my-challenging-collection --id inspection-001 --exposure inspection --purpose 'Development input review'
```

`collect` is create-only and prints terminal counts; `verify` performs full V1/V2
input checks. The common evaluator accepts both versions, passes the loaded
physical scenario to its executor and keeps the existing attempt/proof/replay
contract. It binds **all** V2 proposal files, including exclusions, once before
and after a campaign; each attempt binds only its selected input. Strategy
behavior, commands and clocks remain unchanged.

Python `load_collection_missions(path, indices, usage_id, exposure)` uses the same
native reader and returns each version's original resolved JSON shape. Only
admitted entries can be consumed; explicit repeated attempt indices are allowed.
Existing receipts record tuning/evaluation exposure. Actual Python-call tests
verify byte and schema agreement.

The incumbent `pd-benchmark` composition/report frontdoor still consumes V1;
V2 presentation integration is a later task. No flight or learning campaign is
needed to verify this input-management milestone.
