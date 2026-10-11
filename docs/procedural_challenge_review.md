# Procedural challenge planning review

[Documentation home](README.md) · [Implementation plan](procedural_challenge_plan.md) ·
[Retained learning results](procedural_learning_results.md)

Date: October 11, 2026. Primary-source research, code review and saved-input
validation are complete. A subsequent owner review revised the coverage design
before implementation. The [execution record](procedural_challenge_execution.md)
tracks that authorized loop. Challenge cutoffs, display labels/bands and backend
proportions require input-only calibration; no new benchmark has been qualified
or flown by this review.

## Review outcome

The proposed direction is supported by the saved physical inputs. The gentle
strata dominate the existing procedural population, and even its strongest
stratum is milder than the earlier mountain/ridge challenge. More seeds drawn
from those same recipes would improve sampling, not repair this coverage gap.

Use layered composition as the main generation foundation; retain value noise
as a second available backend and admit it according to measured coverage. Every
qualification mission must provide a declared terrain challenge. Synthetic
terminal/direct-transfer tasks retain their diagnostic role. Gentle procedural
curricula and integration checks stay separate from qualification scoring.

### Coverage correction after owner review

The saved-input audit validates its geometry calculations and the observed
distribution gap. It does not validate the completeness of four morphology
families or three bands. Equal counts within those twelve groups can miss
departure/approach obstacle placement, narrow-pad interactions and unfamiliar
combinations. A finite test population cannot establish every continuous terrain
and flight condition, even within a declared operating envelope.

The revised plan generates a broad challenging population independently of
classification, retains mixed/unclassified challenging missions, and assigns
overlapping navigation labels afterward. Admission uses a frozen neutral
physical predicate, without requiring a familiar family or an unfilled cell.
Broad evaluation, deliberate stress panels and distribution-shift tests have
separate declarations and denominators. Joint coverage maps expose selected
interactions and limitations; filling their bins is not completeness evidence.
The former 3,072-mission equal-cell target is withdrawn.

## Saved input audit

The [read-only audit](../studies/mission_coverage/audit.py) inspected **4,048**
manifest-bound saved physical inputs: 1,024 value-noise missions, 1,024 layered-
Perlin missions, the original 1k, and the fresh seed-disjoint 1k. Every selected
file matched its saved SHA-256. Repeated auditing reproduced the full result
exactly. Vehicle, gravity and simulation clocks/resources are identical across
these populations; terrain/mission geometry and pad preparation differ.

Measurements use the authoritative **prepared** terrain between pad centers,
with exact interpolated interval endpoints. Chord barrier is height above the
straight pad-to-pad line, not above a chosen flight. Slope quantiles weight
horizontal segment lengths, so denser sampling cannot dominate by vertex count.

| Population | Count | Median relief, m | Median chord barrier, m | Median absolute pad rise, m |
| --- | --- | --- | --- | --- |
| Value noise | 1,024 | 79.3 | 26.3 | 30.7 |
| Layered Perlin | 1,024 | 80.7 | 32.1 | 26.7 |
| Original challenge | 1,000 | 632.0 | 279.2 | 216.4 |
| Fresh challenge | 1,000 | 637.7 | 270.0 | 208.7 |

| New stratum | Value / Perlin median relief, m | Value / Perlin median barrier, m |
| --- | --- | --- |
| 0 | 23.1 / 26.2 | 7.6 / 11.4 |
| 1 | 60.7 / 61.8 | 21.4 / 27.3 |
| 2 | 124.5 / 135.5 | 45.8 / 66.5 |
| 3 | 254.3 / 232.4 | 84.9 / 106.3 |

The mildest old recipe, mountains at 4x, has 346.9 m median relief and a 175.6 m
median barrier. The difference also remains in new transfers restricted to
1,000–1,400 m: median barriers are 38.4 m for value noise and 36.6 m for Perlin,
versus 279.2 m in the old fixed 1,200 m population. This is a geometry comparison,
not a controlled attribution of controller success-rate changes; pad widths,
backend and preparation still differ.

### Sustained obstacles and sensitivity

A diagnostic obstacle is a connected interval at least 80 m wide above a given
chord-relative height. Intersections and interval widths are interpolated
exactly on the saved polyline. The values below test whether the finding depends
on one arbitrary height; they are not qualification rules or flight certificates.

| Population | At least 50 m high | At least 100 m high | At least 200 m high |
| --- | --- | --- | --- |
| Value noise | 283/1,024 (27.6%) | 130/1,024 (12.7%) | 40/1,024 (3.9%) |
| Layered Perlin | 320/1,024 (31.3%) | 148/1,024 (14.5%) | 31/1,024 (3.0%) |
| Original challenge | 816/1,000 (81.6%) | 713/1,000 (71.3%) | 502/1,000 (50.2%) |
| Fresh challenge | 811/1,000 (81.1%) | 687/1,000 (68.7%) | 482/1,000 (48.2%) |

No stratum-0 mission in either new family has an 80 m wide interval at least
50 m above its pad chord. Stratum 2 does have real rolling terrain; calling all
of it literally flat would be inaccurate. The broad coverage gap holds across
the tested diagnostics, but none alone recognizes every valuable terrain demand.

### Pad preparation and measurement traps

In 974/1,024 value-noise missions and 941/1,024 Perlin missions, a maximum-slope
segment touches a prepared pad edge. The median maximum slope falls from 1.89
to 0.37 for value noise and 2.37 to 0.58 for Perlin when those adjacent segments
are excluded. This diagnostic isolates immediate edge connections, not an entire
pad neighborhood or every edited slope.

The code flattens each shelf at its center elevation and connects the flat
endpoint straight to the next retained sample. That sample may be very close,
creating steep short connections. The terrain remains a continuous polyline;
this is not a claim of an actual vertical discontinuity. The finding justifies
versioned bounded transitions and separate raw/prepared metrics. It does not
prove these edges caused the PPO failures or any particular controller miss.

The review caught an additional diagnostic defect: choosing the *first* maximum-
slope segment made its pad-edge flag depend on reflection when several segments
tied on a linear transition. The final audit checks all slopes within declared
tolerance of the maximum, and flat terrain has no edge-peak flag. A hand-computed
regression and sampled saved-world symmetry checks validate that correction.

The chord metric also has a limitation: a straight 400 m uphill terrain profile
has 400 m relief and zero chord barrier. Qualification therefore needs distinct
crossing, ridge and endpoint-demand criteria, rather than a single obstacle
height. A hand-calculated test preserves that distinction.

## Source and contract review

| Finding | Required plan adjustment |
| --- | --- |
| V1 recipes/resolved records reject unknown fields and pin stratum to index modulo four | Introduce explicit V2 contracts; preserve every V1 identity and reader |
| Existing collection rejection accounting means structural validity only | Give geometry admission, quota mismatch and exhaustion separate versioned receipts |
| Current report groups use backend plus numeric amplitude stratum | Add semantic morphology/band descriptors through maintained views; preserve old groups, detail IDs and links |
| Current pad flattening can manufacture steep short edges | Prepare bounded local transitions, retain raw terrain/edit intervals, assess the final physical polyline |
| New 8 m versus old 4 m sampling is another physical difference | Begin V2 with resolved 4 m terrain and independent 1 m input-QA samples |
| Old and current layered implementations are not identical backends | Use source identity and measured geometry; matching seeds cannot establish equivalence |
| A steep elevation transition can have zero chord barrier | Recognize endpoint demands independently of chord barriers and display labels |
| Named families can conceal placement/mission interactions and unfamiliar shapes | Use taxonomy-independent admission, retain unclassified cases and audit selected joint distributions |
| Python generation/collection frontdoors currently accept V1 inputs | Add explicit version dispatch while preserving existing calls and physical execution |
| Previous held-out populations have already been inspected and evaluated | Use them as observed comparisons; freeze new unexposed inputs after calibration |
| Another full multi-approach campaign can retain substantial data | Estimate attempts and storage before allocating flights/training |

The old source's primitive class is named `SimplexNoise`, but its inspected
`noise2` uses square-grid gradients and quintic interpolation, a Perlin-style
construction. Calling the old 1k either today's value noise or today's exact
FastNoiseLite Perlin would lose that source distinction. The relevant preserved
composition is [Pylander's layered generator](../outputs/eval/planner_v2_random_terrain/capture-ballistic-combined-validation-inputs-20261010-v1/reference/game/core/terrain.py)
and the [study loader/refinement](../studies/terrain_profiles/study.py).

## Primary-source research and application

[Cobbe et al., Procgen (2020)](https://arxiv.org/abs/1912.01588) studies learning
and generalization across diverse procedural environment distributions. Its
[reference implementation](https://github.com/openai/procgen) separates the level
seed range from the distribution mode. For this lab, the proposed application
is to distinguish unseen seeds within a frozen recipe from new recipe/backend
coverage. More seed samples cannot establish missing terrain morphologies.
This paper does not prescribe our terrain thresholds or an all-challenging set.

[Mouret and Clune, MAP-Elites (2015)](https://arxiv.org/abs/1504.04909) organizes
variation using explicit user-chosen descriptors. The relevant design lesson is
to expose occupancy and holes in measured geometry space. The plan adopts that
coverage-map idea only; it does not add evolutionary optimization, rank terrain
by controller failures or select one controller's preferred environments. The
chosen axes limit what the map can reveal; a full map is not full terrain coverage.

[Agarwal et al. (2021/2022)](https://arxiv.org/abs/2108.13264) shows why aggregate
RL point estimates with few independent training runs can be misleading, and
advocates interval estimates and performance profiles. For this lab, retain
each training seed and per-family/band outcomes. Separate variability across
trained policies from sampling of missions, and keep mirror pairs/repeats grouped
when estimating uncertainty. Thousands of missions do not create thousands of
independent training runs. The exact estimator belongs to the later campaign.

[FastNoiseLite's official controls](https://github.com/Auburn/FastNoiseLite/wiki/Documentation)
separate noise type, frequency, octave gain/weighting and domain warp. Our
application is to expose composition and spatial scale independently and measure
their physical output. Noise naming alone does not set mission difficulty.
The current recipe implements its own layering over pinned Perlin primitives;
switching built-in fractal presets would be a separate generator change.

These are design applications of the cited sources. The numerical audit is
local evidence, and its thresholds are our diagnostic choices, not published
standards or measured universal difficulty boundaries.

## Validation and reproducibility

The audit and eleven geometry/reader tests pass without native flight execution.
Checks include analytical triangles and uphill profiles, separated obstacles,
plateaus, exact interval clipping, weighted slopes, collinear subdivision,
translation/reflection, tied slopes, invalid terrain, and changed/escaping input
files. The ordinary test invocation needs no retained output collections.

Two final full audits match exactly, reauthenticating all 4,048 inputs. Separately,
254 evenly sampled saved worlds pass 508 reflected/translated measurement checks
within 1e-8 relative and 1e-7 absolute tolerance; edge flags agree exactly. This
validates these geometry calculations, not native replay or a morphology classifier.

The final create-only [geometry audit](../outputs/mission-coverage-review/review-20261011T071327Z/geometry-audit.json)
and [validation receipt](../outputs/mission-coverage-review/review-20261011T071327Z/validation.json)
record source/manifest/input bindings. They are local generated research data;
the tracked summary and script keep the review useful in a fresh checkout.

```sh
rtk proxy python3 -B -m unittest discover -s studies/mission_coverage -p 'test_*.py' -v
rtk proxy python3 -B studies/mission_coverage/audit.py
rtk proxy node scripts/check-docs.mjs
rtk proxy node --test scripts/test-check-docs.mjs
rtk proxy git diff --check
```

Documentation validation covers 207 Markdown files and 1,383 local links with
zero issues; all eight documentation-checker Node tests and `git diff --check`
pass. Existing captures, source mission inputs,
controllers/planners and report editions are preserved. No flight campaign,
training, report publication, server lifecycle change, commit or push belongs
to this planning pass.

## Remaining calibration outputs

Before freezing a release, validate significant-crest prominence/width/spacing,
taxonomy-independent challenge criteria, overlapping display labels/bands and
joint coverage audits. Review pad transitions and bounded input previews with
consistent physical scales. Document unrecognized challenge mechanisms, observed
holes and correlations; retain qualifying unclassified geometry. Choose backend
proposal weights from observed coverage/yield before generating new cohorts.

Choose broad, stress and shift sample counts from yield and a separately declared
evaluation budget. Do not reinstate equal-cell quotas as completeness evidence.
These are input-pilot deliverables, not reasons to resume controller tuning or
launch another uniform PPO run before correcting the benchmark.

