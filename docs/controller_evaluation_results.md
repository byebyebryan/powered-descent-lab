# Controller characterization results

[Design](controller_evaluation_design.md) · [Implementation plan](controller_evaluation_implementation_plan.md) · [Execution and repair ledger](controller_evaluation_execution.md) · [Evaluation workflow](evaluation.md)

October 21, 2026 local time; October 22 UTC. **The unattended pass is complete.**
The unchanged controller lands **598/600 fresh local-reference cases (99.67%)**,
but its one-sided 95% exact-binomial lower bound is **98.954%**, below the
predeclared 99% requirement. This is a completed negative qualification result.
Do not extend that sample, weaken the criterion or call the controller qualified
for general local control or actual mission handoffs.

The software and evidence path pass. All **945 retained direct/facade comparisons
are exact**, including their failures. Activation timing adds only one possible
landing beyond immediate activation on the 64-original grid. No RL training,
backend tuning, default promotion, push or report publication occurred.

## Verdicts and populations

| Gate | Result | Interpretation |
| --- | --- | --- |
| Software | All 19 maintained developer checks pass | Exact input export, immutable recipes/audits, bounded execution, recovery and evidence checks are implemented |
| Retained adapter parity | 945/945 exact comparisons and complete proof tuples | The facade preserves the ordinary backend on this selected panel |
| Structured recovery | 244/256 primary occurrences land; three failed parents | Designed coverage and regression evidence; transformed copies are not independent trials |
| Fresh reliability | 598/600; lower bound 0.989544510615803; criterion fails | Only the declared synthetic mixture was tested; no per-stratum 99% claim |
| Activation value | Immediate 61/64; hindsight union 62/64 | Limited landing headroom for timing alone; no learned policy was tested |
| Whole-mission handoff | Unmeasured | Original launch/transfer integration, elapsed clocks and mission-generated entry distributions remain a separate gate |

All landing claims above require the native target/mission outcome, neutral
integrity and complete replay, controller reconstruction, planner reconstruction
and outer-plan integrity. A verified crash remains a completed failed landing.
Unsupported diagnostics have no fabricated flight. Model-integrity diagnostics
with deliberately inconsistent geometry do not establish a physical capability.

## Frozen implementation and task

Retained parity uses source `0378812`, release executable SHA256
`996fb0da0fb9eca5c7d73d8a09744332cf14e59da5f293f066eb23e0af17c5be`.
Corrected local stages use source `e2d2482`, executable SHA256
`8e1499abfac8e1424ff34dcf7daf15565e92ad6743eb2c3032abeb24777f4a6c`.
The source trees and binaries remain at their exact bound epoch paths. Their
controller descriptors are equal; plant, backend, tuning and native command
semantics were not changed. The ordinary terminal controller retains terrain
clearance and disables all experimental flags. Physics/controller/planner
cadence is 120/60/10 Hz.

The neutral exporter resolves the retained inputs without loading a controller
configuration or making predictions. Four pre-refactor scenario/descriptor
fingerprints remain exact over 4,725 smoke/full occurrences. The two full packs
contain 3,780 occurrences and 3,672 distinct physical states. Smoke/full overlap
and source aliases remain explicit. The physical identity excludes display
labels and seed, while exact scenario bytes and lineage remain bound separately.

Pilot resources alone selected the 945-case retained panel, with seeds 0/1/6,
all 15 source groups, seven arcs and three velocity bands. Its 945 physical states
are distinct. The other 2,835 source slots were unallocated under the prospective
storage share; no full-pack landing rate is claimed. The original 60-second
reachability and resolved 90-second evaluation horizons are both retained.

The corrected synthetic task is `local-reference-v2`, using
`local-joint-sampler-v2`. Its primary bounds are:

| Factor | Declared bounds / construction |
| --- | --- |
| Clearance | 5–800 m, attitude-aware lowest-foot placement |
| Lateral entry | Signed offset up to 0.75 times clearance; targetward velocity over `sqrt(2h/g)+2` plus -6 to +6 m/s residual |
| Descent | 0.1–0.85 times `sqrt(2 * initial net upright acceleration * clearance)` |
| Attitude / rate | -0.25 to +0.25 rad; -0.15 to +0.15 rad/s |
| Mass / fuel | Dry mass 7,200 / 9,450 / 11,700 kg; positive fuel 1,000–6,300 kg |
| Pad / original deadline | Width 24–60 m; surface 0–20 m; deadline 60–90 s |
| Terrain | Flat; 12 m approach ridge; 18 m backstop; terrain baseline equals pad surface |

Native validity and facade support determine dispositions; controller forecasts
never filter the population. Pad/terrain agreement is checked over the complete
pad interval, including every interior heightfield vertex. All primary inputs
have zero surface error. The coupled construction is not a proof that every
state is recoverable by some controller.

Development uses cohort `development-v2`, seed 2026102203. Its 64 parent states
cover the 4³ clearance/descent/fuel grid and selected attitude interactions, with
identity, reflection, translation and label-only transforms. The 256 primary
occurrences represent 192 distinct physical inputs. There are separately 64
diagnostic occurrences. Counter-derived streams supply other factors; the panel
is not a complete Cartesian test of all interactions.

Confirmation uses cohort `confirmation-v2`, seed 2026102204, continuous scalar
draws, the declared discrete mass/terrain mixture, identity only and no
diagnostics. All 600 physical identities and parents are unique and disjoint
from the declared retained/development/invalidated inventories. Older global
exposure is not comprehensively known. These are synthetic airborne entries,
not sampled launch/transfer handoffs. The audit retains joint-cell gaps; neither
passing preflight nor an aggregate rate certifies every cell or boundary.

## Measured outcomes

| Panel | Scheduled | Complete native results | Target landings / native outcomes |
| --- | --- | --- | --- |
| D retained parity | 945 | 945, plus 945 direct references | 865 land; 80 crash; every pair exact |
| E2 primary | 256 | 256 | 244 land; 12 crash, across three failed parents |
| E2 diagnostics | 64 | 56; eight unsupported gravity rejections | 28 native target flags, 12 crashes, 16 timeouts; eight target flags are the deliberately inconsistent model probe |
| F2 activation | 64 originals × 7 | 448 | Paired program results below; these are not 448 independent missions |
| G2 fresh confirmation | 600 | 600 | 598 land; two crash; all proofs/inventories pass |

D's empty/reference and half/core groups each land 315/315. Full/stress lands
235/315, and all 80 retained crashes are in its high-speed band. These historical
pack labels do not define a newly qualified operational core.

E2's three failed identity originals are indices 32, 72 and 220: a long lateral
approach ending with exhausted fuel; a fast, low-fuel entry that crashes with
fuel remaining; and a 5 m-clearance lateral entry. Their transformed outcomes
agree. All 78 executed parent label controls have exactly identical native
artifacts, full controller frames except compute timing, and planner traces.
Reflection has zero fuel difference; translation's maximum absolute difference
is 3.64e-12 kg. Neither transform changes a landing verdict. The two unsupported
parents remain rejected rather than treated as failed native flights.

Diagnostics each retain eight occurrences: near-empty fuel crashes eight;
three-tick deadlines time out eight; narrow pads time out eight; ascending
entries land four and crash four; attitude and near-contact cases each land
eight; unsupported gravity rejects eight. The below-target probe reports eight
native target flags at the terrain instead of the declared pad elevation. It
demonstrates the model/input boundary and stays outside capability scoring.

The independent binomial check uses a separate three-term survival sum and
bisection. It reproduces the Rust lower bound within 1e-12. The confidence
criterion remains failed; no extra samples or revised bounds were released.

## Activation comparisons

Every program starts the same original mission, coasts through actual native
time, fuel and attitude evolution, and then activates the same controller.
There is no state, fuel or deadline reset. Native termination before activation
remains an observed result.

| Program | Landings / 64 | Gained / lost versus immediate | Never reached activation |
| --- | --- | --- | --- |
| Immediate | 61 | — | 0 |
| Never | 0 | 0 / 61 | 64, by definition |
| Tick 12, 0.1 s | 61 | 0 / 0 | 0 |
| Tick 60, 0.5 s | 47 | 0 / 14 | 1 |
| Tick 120, 1 s | 36 | 0 / 25 | 15 |
| Tick 240, 2 s | 30 | 1 / 32 | 16 |
| Tick 480, 4 s | 17 | 1 / 45 | 21 |

The 2 s and 4 s gains are the same original: index 32. Immediate activation
exhausts its 1,000 kg fuel; a 2 s delay lands with about 519 kg remaining.
The hindsight union is 62/64, versus the best fixed programs' 61/64. This is an
in-sample bound over the tested grid, not a deployable oracle, a global optimum
or evidence that an RL policy would realize the gain. Across the same 61
successful originals, 0.1 s delay changes fuel by median -2.34 kg, with both
savings and increases. That is descriptive development evidence, not a promoted
fuel-saving policy. Immediate activation remains the frozen confirmation policy.

## The two fresh failures

Both are 7,200 kg dry-mass approach-ridge cases. They reach ground beyond the
target with excessive lateral velocity and 0.42 rad attitude. Vertical speed
has largely been arrested, fuel remains and the original deadline has not run
out. These observations locate a lateral/contact failure pattern; they do not
prove that an alternative controller can recover either input.

| Fresh index | Initial clearance / fuel | Initial vx / vy | Contact vx / vy | Fuel left / elapsed |
| --- | --- | --- | --- | --- |
| 324 | 529.53 m / 1,020.94 kg | -31.77 / -94.09 m/s | -14.22 / -0.18 m/s | 641.79 kg / 11.39 s |
| 494 | 690.25 m / 1,283.14 kg | -36.79 / -132.01 m/s | -23.41 / -0.61 m/s | 856.02 kg / 10.28 s |

These exposed cases can become development regressions in a later pass. They
cannot be reused as fresh confirmation for a revised controller or envelope.

## Technical invalidation and evidence retention

The first local sampler raised pad surfaces while keeping terrain at zero.
The native collision model uses terrain and classifies target membership by
horizontal footprint; native replay can therefore reproduce an apparent target
landing in an inconsistent scene. Inspection of a diagnostic exposed the fault.
All v1 local capability results were quarantined, rather than pooled or replaced
in place. Its E/F schedules completed; G retained 226 completed results, one
interrupted worker and 373 unstarted slots, without a completion seal or
confidence verdict. That worker's missing counts remain unavailable.

One permitted measurement repair and the sole confirmation technical restart
produced the corrected geometry, a preflight guard and new task/cohort/seed
identities. Sample sizes, numeric bounds, programs and confidence criterion
stayed fixed. All original sources, declarations, outcomes and analyses remain
at their original paths. D remains valid and separate. The execution ledger
records the fault, repair, accounting correction and prospective releases.

## Cost, reports and validation

| Valid stage | Elapsed, including evidence | Output bytes | Median / p95 supervised attempt |
| --- | --- | --- | --- |
| D | 286.87 s | 5,137,844,183 | 0.301 / 0.442 s |
| E2 | 60.88 s | 1,087,344,842 | 0.181 / 0.342 s |
| F2 | 70.55 s | 1,119,352,940 | 0.121 / 0.281 s |
| G2 | 124.83 s | 2,296,762,418 | 0.201 / 0.301 s |

Each ordinary saved attempt includes one original and seven separately counted
proof/reconstruction passes. Direct parity adds one reference original. Across
valid, invalidated and pilot work there are 4,248 completed original executions
including references, plus at most one unknown interrupted original. Known
proof work includes 23,136 passes, of which 36 come from twelve saved-report
reviews. The interrupted attempt may have up to seven additional proof passes;
they are unavailable, not zero. Native software-test work is outside those
campaign totals but inside the loop's global wall/storage accounting.

Wall/byte measurements include supervision, serialization and proof work on one
host under ambient load. They are not hard real-time guarantees. Thread CPU
time and internal forecast work are unavailable in the batch records. The
existing rich template's zero CPU card is not a CPU measurement, and successful
post-contact velocity cards can show the plant's settled zero state. Contact
analysis above uses saved incoming contact states instead.

The artifact root is
`outputs/controller-characterization/loop-20261022T043655Z/`. Important receipts
are `declarations/allocation-declaration-v1.json` and `-v2.json`,
`technical-invalidation-v1.json` and its accounting correction,
`analysis-v1/D-retained-analysis.json`, `analysis-v2/`,
`confirmation-independent-check-v1.json` and `closeout-accounting-v1.json`.
Plans, source epochs, executable hashes and row/result/capture seals are retained
there. Ordinary inventory checks add no replay. Keep the exact epoch paths for
composite reconstruction; neutral physical proof is a separate boundary.

Twelve new reports use the maintained rich template, with fresh three-part proof
checks each and no original flights. Headless Chrome screenshots verify a fresh
failure and the delayed-activation comparison. Review examples:

- [Fresh failure 324](../outputs/controller-characterization/loop-20261022T043655Z/reports/review-v1/07-confirmation-failure-324.html) and [failure 494](../outputs/controller-characterization/loop-20261022T043655Z/reports/review-v1/08-confirmation-failure-494.html).
- [Immediate fuel exhaustion](../outputs/controller-characterization/loop-20261022T043655Z/reports/review-v1/05-activation-immediate-fuel-exhaustion.html) and [the same original with a 2 s delay](../outputs/controller-characterization/loop-20261022T043655Z/reports/review-v1/06-activation-two-second-delay-landing.html).
- [Deliberately inconsistent model diagnostic](../outputs/controller-characterization/loop-20261022T043655Z/reports/review-v1/11-inconsistent-pad-model-diagnostic.html), excluded from capability claims.

The full report selection/verification receipts retain every report and proof
counter. No accepted report selector, navigation index, server or publication
was changed. The maintained gate passes all 19 checks, including 18 maneuver
library tests, two ordinary CLI tests, four batch CLI tests, workspace tests,
strict Clippy, formatting, docs/Node checks and dependency boundaries. Frozen
plant/controller/planning/fixture bytes and unrelated work remain preserved.

## Next decision

The harness is ready for a bounded controller comparison; the reliability and
whole-mission gates remain open. Define the actual mission-generated handoff
requirements before declaring an operational envelope. Use the preserved fresh
lateral failures, three structured failed parents and retained heavy/high-speed
family to separate physical/budget infeasibility from control limitations.
Compare any explicit opt-in controller change against the unchanged baseline
on the same inputs, with paired preservation and complete proof tuples, then
freeze a new controller/task before another independently allocated confirmation.
Do not retrospectively trim this population or reuse its failures as final tests.

The present timing-only panel offers one hindsight landing beyond immediate
activation and no better fixed landing policy. Establish a mission-relevant,
state-dependent decision advantage over simple deterministic baselines before
launching RL. General maneuvers, arbitrary arrivals and whole-mission handoffs
remain separate work above the existing planner/controller split.
