# Challenging procedural mission benchmark

[Documentation home](README.md) · [Review and research](procedural_challenge_review.md) ·
[Mission contracts](procedural_missions.md) · [Learning follow-up](procedural_learning_next.md)

Status: October 11, 2026 reviewed roadmap, revised after the owner's coverage
question. The [execution record](procedural_challenge_execution.md) owns the
authorized implementation loop. The saved-input audit is complete; native V2
implementation and input-only calibration are underway. Flights and training
remain separately scoped work.

## Decision and intended behavior

The procedural qualification benchmark must contain a meaningful terrain
challenge in every admitted mission. Synthetic terminal and simple-transfer
cases own the basic physical-control diagnostics. Gentle procedural inputs may
serve training curricula and integration checks in separately named collections;
they do not contribute to the challenging benchmark's headline denominator.

Keep both existing generators and all their saved inputs. Use layered terrain
as the primary foundation for the new benchmark. Value noise remains available
as an additional backend when it fills a measured geometry gap or supplies a
declared generalization check. Backend counts follow coverage needs, without an
automatic equal split. Mix missions from backends and retain each mission's
provenance; blending both noises within every terrain is unnecessary.

Qualification here means complete missions sampled from a declared challenging
2D heightfield, vehicle, gravity and resource distribution. A high rate supports
performance on that distribution, with measured sampling uncertainty. Neither
named categories nor a filled coverage map establish complete coverage of the
envelope. Unmeasured geometry, interactions and generator biases remain possible.
This does not establish arbitrary vehicles, overhangs, 3D flight or every
physically possible mission.

## What the previous populations represent

The current value-noise and layered-Perlin collections vary noise amplitude
over 20–60, 60–140, 140–280 and 280–480 m. These strata are generator parameters;
they do not guarantee significant intervening terrain. The [input audit and
review](procedural_challenge_review.md#saved-input-audit) measures the difference.

The previous 1k used Pylander's hash-bound layered composition, refined weighting
and a ridge-slice adjustment. It contained four recipes, 250 worlds each:
mountains at 4x and 8x vertical scale, broad massifs at 8x/1.5x vertical/horizontal
scale, and successive ridges at 8x/0.6x. Its noise primitive differs from the
current FastNoiseLite Perlin backend. Reuse its physical challenge range and
retained worlds as comparison evidence, without treating equal seeds or the
current layered backend as exact reproductions.

Both earlier 1k populations and the October 10 learning collections are already
observed. New generalization claims need new, disjoint inputs after the new
recipe and coverage definitions are frozen.

## Ownership and data flow

```mermaid
flowchart LR
  R[Versioned terrain and mission recipe] --> G[Generate raw terrain]
  G --> P[Prepare bounded local pads]
  P --> V[Validate physical structure]
  V --> M[Measure resolved mission geometry]
  M --> C[Admit by neutral challenge rule]
  C --> I[Seal immutable mission collection]
  M --> A[Audit joint coverage and assign display labels]
  I --> E[Evaluate any independent approach]
  E --> Q[Verify commands and native replay]
  A --> D[Mission previews and coverage views]
  Q --> D
```

| Owner | Responsibility |
| --- | --- |
| `pd-missions` | Versioned generation, pad preparation, neutral geometry descriptors, challenge admission, coverage audits and immutable collections |
| `pd-core` / `pd-run` | Existing plant, physical mission view and complete episode execution |
| `pd-eval` | Common scheduling, dispositions, evidence integrity and replay |
| `pd-report` | Saved geometry/outcome projections through maintained rich views |
| `pd-python` | Explicit version dispatch and thin access to native recipes/collections for independent learning callers |
| `pd-benchmark` | Explicit unchanged raw/reflected incumbent comparison adapters |
| `experiments/rl` | Its own training sampler, curriculum and agent; no incumbent dependency |

Physical input generation and membership must never consult a controller,
planner, reward, flight outcome or saved trajectory. Agents continue to receive
physical missions without seeds, coverage labels or collection membership.

V1 uses strict schemas and a pinned index-to-stratum schedule. Introduce explicit
V2 contracts/readers for new recipe controls and coverage receipts; do not insert
new semantics into `terrain_stratum`, broaden the V1 validator or alter existing
resolved identities. Report descriptors can be separately versioned, bound to
the physical input and metric definition, without changing scenario physics.

## Coverage specification

Separate provenance, physical challenge membership, measured coverage and
navigation labels. Generate a broad continuous proposal population before
classification. Do not generate only enough worlds to fill named families, or
discard a challenging world because its morphology has no familiar label.

**Mountain crossings**, **broad massifs**, **successive ridges**, and **large
elevation transitions** remain useful candidate navigation labels. They can
overlap. Assign them from measured geometry after challenge admission; retain
mixed and unclassified challenging missions in full scored denominators. A
single deterministic display group may avoid duplicate rows, but it must not
erase secondary labels or control inclusion. Recipe names cannot certify shape.

Optional substantial/severe/extreme geometry bands are display summaries whose
numeric definitions require input-only calibration. They describe selected
physical demands, not universal difficulty or predicted controller success.
Do not make a four-family by three-band grid the admission or coverage gate.

Audit continuous descriptors and selected joint distributions: distance, signed
pad rise, both pad widths, obstacle width/count/separation, first obstacle
distance from departure, last obstacle distance from destination, and local
terrain on both sides of both pads. For example, moving the last ridge next to a
narrow target changes approach demands without changing ridge count or relief.
Marginal distributions cannot establish coverage of these interactions.

Define diagnostic bins and important interaction panels before examining the
pilot: distance x barrier width; signed rise x target width; departure obstacle
proximity x source width; final obstacle proximity x target width; crest spacing
x sustained barrier height. Record occupancy, sample ranges, correlations,
near-duplicate concentration and holes. A full Cartesian grid is neither
necessary nor sufficient; many combinations are physically constrained. Explain
unsupported or unattainable combinations rather than manufacturing missions.
Keep normalized positions and physical distances together so relative placement
does not hide a narrow landing approach at vehicle scale.

Balance left/right proposals independently of morphology; report achieved
direction counts and conditionally imbalanced coverage. Direction remains an
optional facet. Do not author a route, waypoint list or flight corridor.

### Geometry measurements

Measure the authoritative prepared polyline, clipped exactly to the source-to-
target interval, with interpolated boundary samples. Also retain raw-terrain
measurements and the pad-edit intervals so their effects are visible.

| Descriptor | Meaning and use |
| --- | --- |
| Relief | Maximum minus minimum height between pads; useful scale, insufficient alone |
| Pad rise | Signed destination minus source elevation; separate uphill/downhill demands |
| Chord barrier | Maximum terrain height above the straight pad-to-pad line; geometric obstruction, not a ballistic-path or feasibility certificate |
| Sustained barrier width | Horizontal length of each connected interval above a declared chord-relative threshold; distinguishes a massif from a narrow spike |
| Significant crests | Declared prominence, width and spacing of terrain crests after explicit pad-edit exclusion; retain positions, departure/approach distances and gaps as well as counts |
| Slope distribution | Horizontal-length-weighted quantiles, with raw/prepared and pad-transition values reported separately |
| Endpoint surroundings | Terrain rise and sustained obstacles within declared physical neighborhoods of both pads, examined on both sides |
| Mission geometry | Distance, pad widths, resource envelope, ratios to hull dimensions and terrain scales |

Crest definition, plateau handling, endpoint conventions, interpolation, ties,
quantiles and tolerances are part of a versioned metric specification. Validate
them on hand-calculated geometry before they govern selection. The present audit
implements relief, rise, chord barriers, sustained width, slopes and pad-edge
diagnostics; it does not yet establish the morphology classifier.

Input-only calibration starts with diagnostic barrier heights of 50, 100 and
200 m and a sustained-width diagnostic of 80 m. They are sensitivity probes,
not accepted qualification cutoffs. A final challenge rule must also recognize
large endpoint elevation demands and repeated meaningful ridges. Challenge
membership is independent of the display taxonomy. Publish the exact predicate,
the qualifying mechanism(s), and cases it cannot recognize; do not mistake an
OR of a few metrics for exhaustive terrain coverage.

### Admission and accounting

Every admitted qualification mission must satisfy a frozen neutral challenge
predicate on the **prepared** physical geometry, plus structural validity. The
broad evaluation collection keeps all unique qualifying proposals in its
declared bounded stream, regardless of classification or coverage occupancy.
Its distribution is the declared proposal distribution conditional on that
predicate, not a claim to sample all challenging missions uniformly.

Record structural invalidity, insufficient challenge, exact duplicates and
bounded generation exhaustion separately. Every proposal has exactly one
terminal disposition. Stress-panel selection adds its own explicit dispositions
and weights; it must not alter the broad population's membership or receipts.
Coverage holes are audit findings, not hidden rejection reasons.

Use a declared deterministic proposal order and bounded work. Preserve every
proposal identity, diagnostic descriptor and disposition in an immutable
selection receipt. Report acceptance yield, unclassified counts and holes. Never
silently fill a quota with mild terrain, replace a failed flight, or expand
budgets after controller results. Content identities detect exact duplicates;
spatial descriptors expose concentration without pretending to prove uniqueness
of physical demands. Counterexample and failure-driven diagnostic additions
belong to separately exposed development collections, not a rewritten test set.

Structural validity and challenge membership do not establish solvability. Keep
feasibility unknowns visible. Any later witness-based solvability study uses a
separate collection and label; the incumbent's success is not an admission rule.

## Terrain and mission recipe changes

Expose layered composition controls independently: macro/structure amplitude,
horizontal scale, ridge mix, octave composition, warp and local-feature density
and scale. Begin calibration around the old 4x/8x and broad/compressed regimes,
including intermediate combinations, rather than increasing one amplitude only.
Characterize backend differences by measured geometry, not matching seed numbers.

Use an authoritative resolved 4 m polyline for the first V2 pilot, matching the
old challenge's resolution. Compare it with 1 m reference samples during input
QA and report interpolation differences; sampled agreement is not a continuous-
terrain proof. Avoid interpreting 4 m versus 8 m sampling differences as backend
quality. Physics continues to consume only the sealed resolved polyline.

Freeze the first envelope to the current 8x10 m vehicle, Earth gravity,
6,300 kg fuel, 90 s scenario horizon and 120/60 Hz plant/commands. Retain complete
source-rest starts, locally prepared source/target pads and no authored route.
Explore distance and width jointly with terrain, beginning from the previous
1,200 m / 36 m anchors and current 300–2,400 m / 24–80 m variation. New fuel,
gravity, vehicle or airborne-start tiers are separate future recipes.

V2 pad preparation needs a declared finite transition outside each flat shelf,
with retained raw/prepared samples, edit intervals and nonoverlap checks. The
current flattened shelf connects directly to the next 8 m sample and can create
very steep short edges. Fix this as a versioned physical-input change; measure
the terrain after edits and preserve all terrain outside the declared local
intervals. Transitions cannot extend into a cleared inter-pad flight corridor.

## Collection design and reporting

Training, development, validation and held-out qualification collections have
separate recipe/seed declarations and physical-duplicate checks. Inspecting a
specific candidate makes it development/validation exposure; freeze final
held-out inputs after calibration and do not use their previews to tune recipes.

Keep seed-disjoint same-recipe evaluation separate from recipe/backend-shift
evaluation. Claim backend transfer only when the training exposure actually
excludes that backend. Procedural variation within one recipe is a narrower
claim than coverage of a new morphology or resource envelope.

Maintain three separately declared populations:

- **Broad challenging evaluation:** all unique qualifying proposals from a
  bounded continuous recipe mixture. Equal morphology counts are not required.
- **Targeted stress panels:** deliberate boundary and interaction coverage, with
  overlapping membership identified and separate scores/weights. These diagnose
  gaps rather than estimate the broad population's average success.
- **Distribution-shift evaluation:** unseen recipe settings, compositions or
  backends excluded from development/training. New seeds alone are not a shift.

Choose sample counts from yield, sampling precision and the later flight budget.
The former 3,072-mission twelve-cell target is withdrawn: filling that grid could
still miss obstacle placement, geometry interactions and novel shapes. Record
achieved counts and distribution limits rather than claiming completeness from
mission count. Backend weights belong to the declared proposal mixture; pilot
changes require a new declaration before the final cohort is generated.

Lead reports with coverage occupancy, relief/barrier/rise distributions and
representative physical previews. Preserve the review hierarchy:
**mission collection -> morphology family -> geometry band -> mission**.
Retain one open band at a time, twelve missions per page, lazy preview loading,
stable detail/return context, normalized direction and rich flight plots.
Backend, direction and mission facets remain secondary filters. Show physical
units and view bounds; optional terrain-only zoom must be clearly marked.

Outcome views lead with each population's scheduled unique-mission rate and
honest denominator, then labels, bands and interaction panels. Multi-label
subgroup counts overlap and must not be summed. Do not average broad, stress and
shift populations into an undefined headline. Retain each RL training seed and
unsupported dispositions. Sampling intervals refer to the declared generator
distribution, not unmeasured terrain; training-seed variation is separate.
Mirror pairs, retries and repeated executions are not independent missions.
Coverage must remain readable before any controller evaluation exists.

## Implementation slices and acceptance

| Slice | Concrete result | Required acceptance |
| --- | --- | --- |
| 1. Neutral geometry | Versioned descriptors and read-only audits in `pd-missions` | Hand-calculated cases, translation/reflection and resampling invariance, plateau/endpoints, finite/bounded work, no strategy dependencies |
| 2. V2 recipes and pads | Configurable layered composition, bounded pad transitions, unchanged V1 reader/generator | Pinned V1 identities, exact outside-edit preservation, structural/source-rest validity, deterministic reversed/parallel generation |
| 3. Coverage collections | Broad/stress/shift declarations, taxonomy-independent challenge admission and complete receipts | Every admitted mission qualifies; challenging unclassified cases retained; all dispositions accounted; repeat/order equivalence; visible yield/exhaustion; no outcome input |
| 4. Input pilot and views | Input-only distributions, fixed-scale previews and maintained report projections | Raw/prepared agreement, challenge sensitivity, overlapping labels, joint occupancy, near-duplicate and pad-artifact checks; declared gaps and bounded browsing |
| 5. Freeze and compare | New cohort declarations and a separately scoped unchanged-incumbent campaign | Frozen definitions before flight outcomes; identical common inputs, raw/reflected support explicit, complete native integrity/replay tuple |

Extend the CLI examples and `pd-python` recipe/collection frontdoors with explicit
version dispatch as part of slices 2/3. Existing V1 calls keep their exact
behavior and serialization. Identical physical inputs/commands through the
native and Python paths must produce identical episode results; no learning
sampler or approach fallback belongs in the binding.

The input pilot evaluates 10,000 scheduled proposals per participating backend,
with a fixed structural retry limit and balanced direction schedule. Freeze its
seeds, proposal mixture, limits and diagnostic interaction bins before running.
These are exposed development inputs, not final held-out qualification.
Select a bounded preview sample across geometry quantiles, interaction boundaries
and mixed/unclassified cases; no all-mission expansion. Adjust recipes only from
input geometry, under a new pilot declaration, and publish acceptance yield and
remaining gaps. Final release criteria require documented coverage limitations
and known blind-spot examples, not only a filled map.

Slice 5 needs its own flight budget and capture/storage estimate. The previous
held-out campaign retained about 51.7 GB across multiple approaches; a larger
comparison needs an explicit estimate instead of automatically repeating every
training seed and backend. Controller tuning and another RL allocation follow
the mission-platform acceptance, with separately declared hypotheses/budgets.

Bind the incumbent comparison to its parked combined acquisition/terminal-safety
candidate, correction cap 24 and explicit global-reflection adapter. Retain the
raw adapter's unsupported directions in its full scheduled denominator and show
supported-input rates separately. Keep the scenario's 90 s horizon distinct from
the incumbent's existing internal deadline/reserve policy; do not extend that
policy to improve a comparison. Any later agent change has a new descriptor.

## Readiness and stop conditions

The [review record](procedural_challenge_review.md) supports staged implementation
but does not validate taxonomy completeness. The [execution record](procedural_challenge_execution.md)
tracks the authorized loop; the [neutral native geometry contract](procedural_geometry.md)
and [V2 generation/pads](procedural_generation_v2.md) are implemented. Challenge
thresholds, display labels/bands, backend
weights and observed coverage gaps require input-only calibration. A qualified
release needs a frozen distribution and explicit limits; it is not a complete-
coverage or solvability certificate.

Stop input promotion for unexplained pad artifacts, unexplained interaction gaps,
unaccounted proposals, changed frozen inputs, inconsistent measurements or
source/identity drift. A finite controller miss is retained evidence and does
not authorize changing mission membership, deadlines, fuel or guard policies.

