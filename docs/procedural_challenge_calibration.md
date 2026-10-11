# Challenging procedural input calibration

[Roadmap](procedural_challenge_plan.md) · [Audit/report contract](procedural_input_audit.md) ·
[Execution record](procedural_challenge_execution.md) ·
[Next comparison scope](procedural_challenge_comparison_plan.md)

October 11, 2026: the declared V2 input-only development pilot completed.
It establishes measured challenge membership and input consistency. It does not
freeze a benchmark, prove solvability or provide a new controller result.

The [declaration](../studies/mission_coverage/pilot_v2/pilot.json) was committed
as `9fc5335` before generation. Native audit/runner source was committed as
`638f487`. The [completed run receipt](../outputs/procedural-challenge/run-20261011T093222Z-pilot/run-result.json)
binds unchanged source/executable and both collection/audit identities. All
20,000 resolved inputs, including exclusions, passed complete native verification.
No held-out inputs, flights or training were allocated.

## Population and challenge

| Measured population | Layered Perlin | Value noise |
| --- | ---: | ---: |
| Scheduled / resolved proposals | 10,000 | 10,000 |
| Admitted unique challenges | 8,168 | 8,080 |
| Insufficient challenge | 1,832 | 1,920 |
| Structural exhaustion / exact duplicate exclusions | 0 / 0 | 0 / 0 |
| Admitted right / left | 4,078 / 4,090 | 4,000 / 4,080 |
| Mixed morphology | 5,142 | 4,765 |
| Qualifying unclassified morphology | 922 | 897 |
| Mountain crossing only | 1,190 | 674 |
| Broad massif only | 83 | 111 |
| Successive ridges only | 98 | 29 |
| Elevation transition only | 733 | 1,604 |

These display groups partition admission; individual labels overlap. Both
streams retain every unique qualifying input independently of its labels.
No admitted physical identity is shared across the streams. Each admitted
mission passes at least one **same** challenge path on raw and prepared terrain.
Sustained-obstruction, pad-rise and repeated-crest path counts overlap:
layered 6,942 / 4,014 / 4,314; value 6,148 / 5,316 / 2,869.

| Admitted descriptor | Layered p10 / median / p90 | Value p10 / median / p90 |
| --- | ---: | ---: |
| Transfer distance, m | 589.6 / 1,448.9 / 2,199.7 | 605.5 / 1,465.2 / 2,214.7 |
| Signed target-pad rise, m | -538.5 / -6.1 / 528.9 | -751.6 / -12.4 / 721.9 |
| Chord barrier, m | 112.0 / 359.9 / 784.4 | 35.0 / 310.5 / 820.8 |
| Sustained width above 100 m, m | 22.2 / 277.1 / 879.4 | 0 / 295.3 / 1,017.3 |
| Significant crest count | 1 / 2 / 5 | 0 / 2 / 4 |
| Target width, m | 29.4 / 51.2 / 74.1 | 29.5 / 51.8 / 74.4 |
| Departure barrier distance when present, m | 44.8 / 186.3 / 795.2 | 47.1 / 196.4 / 927.0 |
| Approach barrier distance when present, m | 44.5 / 176.1 / 772.6 | 48.8 / 213.2 / 893.7 |

Median prepared relief is 722.4 m layered and 794.1 m value. These are
substantially larger than the observed V1 learning collections' roughly 80 m
median relief, under the [saved-input audit](procedural_challenge_review.md).
This is a physical-distribution difference, not a controller improvement.
Low/zero chord barriers remain challenging when another declared path passes;
large pad elevation changes are intentionally included.

## Interactions, concentration and holes

| Admitted observed-pair correlation | Layered r | Value r |
| --- | ---: | ---: |
| Distance × sustained width | 0.386 | 0.399 |
| Signed rise × target width | 0.000 | 0.008 |
| Departure distance × source width | -0.001 | -0.025 |
| Approach distance × target width | 0.015 | 0.008 |
| Minimum crest spacing × chord barrier | -0.185 | -0.120 |

Ten marginals and five full joint matrices reconcile to each population's count,
including explicit missing cells. Missing departure/approach barriers account
for 739 layered and 1,492 value inputs; fewer than two significant crests make
spacing missing on 2,608 / 3,833. Missing does not mean zero distance.

The diagnostic matrices expose support and selection limits. Source/target width
is bounded to 24–80 m; distance to 300–2,400 m. Missing-width/distance rows are
empty by construction. Very close (`<40 m`) barriers adjacent to widths `≥64 m`
are sparse: layered departure/approach 10/8; value 8/14. Those demands interact
with finite pad shelves/transitions. Very tight (`<40 m`) crest spacing contains
only 10 layered and 5 value admitted inputs, concentrated in high-barrier terrain.
These are candidate **separate stress** targets; do not refill or relabel the broad
stream after inspecting them. Empty/sparse cells require structural interpretation
before anyone treats them as missing feasible missions.

Every admitted input has a distinct coarse shape bucket in this pilot.
The bucket combines thirteen quantized chord samples with distance, rise, widths
and crest count. High-dimensional sparse signatures can separate similar demands;
this result does not prove an absence of near neighbors. A meaningful metric on
local obstacle placement/width or a multiscale profile remains future work.
There is no complete-coverage conclusion from labels, filled cells or sample size.

## Raw terrain, pad preparation and resolution

Outside declared pad-edit intervals, terrain remains unchanged by construction
and native preservation checks. Observed raw/prepared outside-edit p95-slope
differences are numerical projection residuals: maxima about `5.4e-13` layered and
`5.1e-14` value. Median edited fractions between pad centers are 6.9% / 6.9%;
maxima are 39.4% / 38.5%, concentrated on short transfers.

Pad transitions still dominate the maximum prepared slope on 2,794/8,168 layered
and 3,554/8,080 value inputs. Finite transitions remove the old undeclared edge
step convention, but do not imply gentle edges. These counts use the new
declared edit intervals and must not be equated with the older pad-adjacent
segment flag. Outside-edit p95 slope medians are 3.677 / 2.869, so steep terrain
also exists independently of pad edits.

| 1 m counterfactual QA selection | Inputs | Raw error >25 m | Barrier / width flags | Admission changes | Max raw error |
| --- | ---: | ---: | ---: | ---: | ---: |
| Layered regular paired indices | 250 | 1 | 0 / 0 | 0 | 25.51 m |
| Layered targeted boundaries | 32 | 3 | 0 / 0 | 0 | 49.05 m |
| Value regular paired indices | 250 | 2 | 0 / 0 | 0 | 29.83 m |
| Value targeted boundaries | 32 | 1 | 0 / 0 | 0 | 25.78 m |

Layered flagged indices: regular 3040; targeted 1311/2883/4949.
Value flagged indices: regular 2081/3521; targeted 3732.
The 25 m raw-error, 25 m barrier-change and 40 m sustained-width-change flags are
diagnostic declarations, not benchmark acceptance thresholds. Zero sampled
admission changes does not establish resolution independence or a continuous
error bound. Targeted cases cannot be pooled into a population-rate estimate.
The sealed 4 m polyline remains authoritative; reference samples never replace it.

## Backend decision and readable review

Keep layered Perlin as the primary broad terrain foundation. Keep value noise
as a separately reported backend contrast: it produces wider signed-rise tails,
more elevation-only challenges, fewer retained crests and lower typical outside-
edit slopes under these matched controls. It is useful complementary evidence,
without a basis for an automatic 50:50 qualification average. No blend inside
each mission is needed. Neither backend spans all terrain geometry.

The [local rich input report](../outputs/procedural-challenge/report-20261011T095305Z-pilot/index.html)
shows complete aggregate counts and 96 preselected previews per backend.
Its [projection receipt](../outputs/procedural-challenge/report-20261011T095305Z-pilot/render.json)
binds saved inputs, source bytes, snapshots, pages and shared plot bounds.
Collection/morphology/barrier-band hierarchy, one open preview band and twelve
images per page preserve bounded browsing. Input details contain raw/prepared
plots and admission paths; no flight result is implied.

The [desktop/mobile browser check](../outputs/procedural-challenge/browser-20261011T095305Z/browser-result.json)
checks initial image absence, paging, reload, direction filtering, detail return,
assets, overflow and errors. Numerical preservation, source authentication and
browser interaction are separate evidence from human interpretation or flights.

## Readiness and remaining limits

Slices 1–4 now have implementation and input evidence. V1 identities and all
controllers/planners remain unchanged. The platform can supply an independent
agent through physical V2 scenarios without incumbent imports. There is still
no V2 controller performance estimate and no frozen qualification benchmark.

Before freeze, review the resolution warnings as mechanism evidence, decide the
intended physical resolution policy, and declare separate sparse-interaction
stress panels. A reference study must inspect more than one targeted seed if it
motivates a recipe change; do not repair the seven named worlds individually.
Changes need new recipe/spec identities and new declarations, preserving this pilot.

The envelope is one Earth plant, fixed resources, rest starts, two pads, a 90 s
horizon and a 1D sampled heightfield. It excludes airborne-start states,
resource/plant perturbations, overhangs and 3D traversal. Endpoint context is
finite; five interactions cannot cover higher-order dependencies. Physical
reachability remains unknown. These limits belong in the next comparison's
headline scope, rather than being hidden behind a mission taxonomy.
