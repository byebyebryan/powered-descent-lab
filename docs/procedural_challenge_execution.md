# Procedural challenge implementation loop

[Roadmap](procedural_challenge_plan.md) · [Planning review](procedural_challenge_review.md) ·
[Mission contracts](procedural_missions.md) · [Development](development.md)

Status: October 11, 2026 implemented and validated. The owner authorized a goal loop and incremental
commits after reviewing the limits of mission categorization. Work is in the
current checkout. No other agent is delegated.

## Authorized result

Implement roadmap slices 1–4: native neutral geometry; explicit V2 recipes and
bounded pad transitions; broad challenging collections and complete selection
receipts; input-only calibration and maintained coverage/report projections.
Prepare a concrete budgeted freeze/incumbent comparison plan for slice 5.
Controller tuning, training and new flight campaigns remain outside this loop.
Keep server lifecycle, accepted report editions, historical captures, frozen
inputs and their evidence identities unchanged. Commit reviewed milestones;
push/publication is not part of the request.

The qualification population must contain meaningful terrain challenge in every
admitted mission. Synthetic terminal/simple-transfer cases cover basic control;
gentle complete procedural missions can support separately declared curricula.
Generate broad continuous proposal distributions independently of classification.
Retain qualifying mixed/unclassified terrain; labels organize review rather than
define an exhaustive universe or admission quotas.

## Reviewable milestones

1. Revise coverage decisions and preserve the saved-input audit/research.
2. Add versioned, physically bound native descriptors with analytical geometry,
   symmetry, clipping, plateau, position/spacing and bounded-work checks.
3. Add configurable V2 terrain/mission contracts and bounded local pad edits;
   preserve V1 bytes/identities and both native/Python frontdoors.
4. Add challenge admission and immutable broad/stress/shift declarations with
   complete proposal accounting, no controller input and deterministic receipts.
5. Freeze a development pilot declaration, run bounded input-only generation,
   compare geometry/resolution, audit selected interactions and review bounded
   previews through maintained reporting components.
6. Reconcile runtime docs, record calibration limits and blind spots, run the
   maintained gate, and commit the readiness/comparison-budget record.

## Initial pilot declaration

Reserve a new create-only output directory. Use seed 20261011031 for layered
development proposals and 20261011032 for value-noise proposals, each with
10,000 scheduled indices and at most eight structural attempts per index.
Alternate physical direction by index; random parameter draws are index-local.
These seeds and all generated previews are exposed development inputs.
Do not generate held-out inputs until the recipe/admission/metric definitions
are frozen. The serialized recipe and pilot configuration must be committed
before generation; later adjustments have new identities and receipts.

The serialized [pilot declaration](../studies/mission_coverage/pilot_v2/README.md)
now binds both recipe files and the metric/admission/projection specifications.
It fixes diagnostic bins and five joint audits, caps 1 m reference QA at 564
inputs, and caps human preview review at 192 inputs. QA flags are diagnostic;
they do not promote a benchmark. Commit this declaration before the full run.

Predeclare diagnostic bins in physical units, with visible underflow/overflow:

| Axis | Diagnostic interior boundaries |
| --- | --- |
| Transfer distance, m | 600, 1,200, 1,800 |
| Signed pad rise, m | -400, -150, 0, 150, 400 |
| Target/source width, m | 32, 48, 64 |
| Chord barrier, m | 50, 100, 200, 400 |
| Sustained barrier width at 100 m, m | 40, 80, 200, 400 |
| Departure/approach barrier distance, m | 40, 100, 250, 500 |
| Significant crest count | 1, 2, 4 |

Audit distance x sustained width, rise x target width, departure proximity x
source width, approach proximity x target width, and crest spacing x barrier
height. An absent obstacle is an explicit missing value, not distance zero.
Bins are diagnostic choices, not completeness or universal difficulty criteria.
Crest-spacing interior boundaries are 40, 100, 250 and 500 m, with `null`
spacing when there are fewer than two significant crests. Present marginal ranges
and joint counts together; do not silently remove overflow or unclassified inputs.

## Preservation and acceptance

V1 schema validation, index/stratum scheduling, serialized bytes and physical
identities remain pinned. New physical inputs use explicit V2 contracts.
Prepared terrain equals raw terrain outside the declared pad-edit intervals;
finite local transitions cannot clear an inter-pad flight corridor.
Agents receive physical missions without seed/coverage membership. The native
plant, command ordering, gravity, resources, clocks, stop and replay semantics
remain authoritative and unchanged.

Each proposal has one terminal disposition; accepted inputs are unique by
physical identity and satisfy the declared challenge predicate after pad edits.
Stress/shift panels retain separate membership, provenance and denominators.
Taxonomy changes cannot change broad evaluation membership. Reversed generation,
symmetry and duplicate/exhaustion cases need meaningful tests.

Use analytical geometry and output-independent tests, then the maintained
development gate for native changes. The old saved-input audit remains local
geometry evidence, not flight replay. Claims after the pilot describe the
declared sampling distribution and measured interactions, not complete coverage
or guaranteed solvability. Document unresolved gaps before input promotion.

## Progress

- Roadmap revision: broad evaluation, stress and shift separated; overlapping
  labels and unclassified retention replace the twelve-cell qualification quota.
- Saved-input analysis: preserved 4,048-input evidence and its eleven pure tests.
- Planning checkpoint checks: eleven audit tests, thirteen documentation/gate
  helper tests, 208 Markdown files / 1,393 local links / zero issues, and clean
  whitespace validation. No runtime change or new flight evidence in this commit.
- Neutral native geometry: implemented under the [versioned contract](procedural_geometry.md).
  Fifteen focused analytical/contract tests plus all ten preexisting mission
  tests pass. Positions, spacing, endpoint surroundings and pad-edit exclusions
  are independent of labels and strategies. Review caught and fixed a
  tolerance-adjacent peak-height defect before accepting the metric version.
- Final geometry code passes all eighteen maintained developer checks, including
  workspace/CLI tests, formatting, strict Clippy, Node tests and dependency
  boundaries. The [gate log](../outputs/procedural-challenge/run-20261011T074706Z-geometry/gate.log)
  and [result](../outputs/procedural-challenge/run-20261011T074706Z-geometry/gate-result.json)
  bind unchanged source files during the run. This is automated preservation
  evidence, not a new terrain flight campaign or qualified benchmark.
- V2 generation/pads: implemented under the [runtime contract](procedural_generation_v2.md).
  Continuous mission/composition draws, explicit backend provenance, balanced
  index-local direction, retained raw terrain, finite local transitions and
  raw/prepared geometry receipts are independent of labels and approaches.
  Ten new generator tests cover reverse/parallel order, exact outside-edit
  geometry, grid-adjacent boundaries, structural exhaustion, raw/receipt drift,
  finer reference samples and pinned default identities for both backends.
- Proposal frontdoors: explicit V1/V2 recipe/resolved dispatch, CLI `recipe` /
  `propose`, and Python recipe/alias calls. Three version tests preserve V1
  serialized generation bytes. Two actual Python-call tests cover generation
  and complete native/Python command-script record equality. The existing RL
  environment installation and all control/learning code remain unchanged.
- Generation checkpoint: all eighteen maintained developer checks pass with
  source unchanged during the [gate](../outputs/procedural-challenge/run-20261011T081226Z-generation/gate.log).
  The [release frontdoor receipt](../outputs/procedural-challenge/run-20261011T081226Z-generation/frontdoor-validation.json)
  verifies CLI/Python agreement for both V1/V2 backends, custom serialized
  recipe/metric calls, unchanged physical identity under metric changes, V1
  metric-option rejection and bounded CLI configuration reads. No collection
  pilot, training, performance campaign or benchmark qualification occurred.
- Challenge/collection implementation: the [V2 contract](procedural_collections_v2.md)
  now retains every structurally resolved proposal and accounts for each scheduled
  index exactly once. Broad admission uses raw-and-prepared sustained obstruction,
  substantial pad rise or repeated significant crests. Multi-label taxonomy is
  independent; qualifying unclassified inputs remain admitted. Separate stress
  panels record quotas/overlap, and shift declarations require changed support
  or backend rather than fresh seeds alone. Default thresholds remain development
  candidates pending the predeclared input-only pilot.
- Collection integration: strict V1/V2 manifest/load/verify dispatch, create-only
  usage receipts, CLI `collect` and Python admitted-input reads are implemented.
  The common evaluator consumes only physical scenarios and binds all V2
  proposal files, including exclusions. Its outcome/proof/replay contract and
  every strategy remain unchanged. The incumbent report composition still needs
  a truthful V2 projection before a future comparison.
- Seventeen new analytical/accounting/interface checks pass, including exact
  threshold behavior, raw-only/prepared-only disagreement, mild exclusions,
  retained unclassified terrain, canonical/reversed schedules, taxonomy-independent
  membership, physical duplicates, structural exhaustion, stress overlap,
  projection/file/symlink drift, Python per-version byte shapes and common
  evaluator detection of excluded-input mutation. All 62 focused mission,
  Python and evaluator checks pass. All eighteen maintained developer checks
  pass with source unchanged during the [gate](../outputs/procedural-challenge/run-20261011T083727Z-collections/gate.log)
  ([result](../outputs/procedural-challenge/run-20261011T083727Z-collections/gate-result.json)).
  The [release frontdoor receipt](../outputs/procedural-challenge/run-20261011T083727Z-collections/frontdoor-validation.json)
  verifies CLI-created collections and actual Python reads across both V1 and
  both V2 aliases, explicit repeated attempts, exposure receipts, create-only
  refusal, excluded-input refusal and same-support shift rejection. No campaign,
  benchmark promotion or training allocation occurred.
- Serialized pilot declaration: native recipe JSON and metric/challenge/morphology
  specs are retained with file hashes in the [configuration](../studies/mission_coverage/pilot_v2/pilot.json).
  The [declaration review](../outputs/procedural-challenge/run-20261011T083727Z-collections/declaration-review-validation.json)
  verifies its bound files, native request validation and balanced regular QA
  schedule: 125 rightward and 125 leftward indices per backend. Review corrected
  a stride/parity blind spot before committing or running the full pilot.
  Declaration commit `9fc5335` precedes input generation.
  These are exposed development seeds, with zero flight/training allocation and
  no held-out generation or benchmark freeze.
- Native bounded input audit: implemented under its [contract](procedural_input_audit.md).
  Separate proposed/admitted marginals, joints, observed-pair correlations,
  conditional directions, overlapping labels/paths, endpoint/pad effects and
  coarse shape concentration remain independent of admission. Bounded reference
  and preview selection follows the declared schedule. Review prevents occupied
  joint cells from starving rare morphology groups within the preview budget.
- Native audit checks: ten new analytical, empty/constant/missing-statistic,
  order/coordinate, sampling-budget and saved-input checks pass, including
  repeated audit equality, excluded-input corruption and unchanged physical
  bytes under finer reference QA. Six runner checks join the eleven historical
  audit checks; create-only dangling paths fail before build/generation.
  All eighteen maintained checks pass with source unchanged during the
  [gate](../outputs/procedural-challenge/run-20261011T092340Z-input-audit/gate.log)
  ([result](../outputs/procedural-challenge/run-20261011T092340Z-input-audit/gate-result.json)).
  [Release validation](../outputs/procedural-challenge/run-20261011T092340Z-input-audit/frontdoor-validation.json)
  preflights both complete 10,000-index requests without generation, then verifies
  small seed-981 audits for both backends, population reconciliation and refusal
  to reuse an artifact before recording further exposure. Reserved pilot seeds
  had not yet been generated at that checkpoint.
- Declared pilot completed: all 20,000 resolved proposals verified, with 8,168
  layered and 8,080 value challenges admitted. There are no structural failures
  or exact duplicates within/across streams. Mixed/unclassified qualifying inputs
  remain in the denominator. The [calibration record](procedural_challenge_calibration.md)
  owns continuous ranges, interactions, pad effects, sparse cells, resolution
  warnings and the backend decision; no complete-coverage claim is made.
- Maintained input views: shared full batch/status templates now project these
  saved audits without generation or flights. Full counts, marginal plots,
  conditional directions and joint matrices precede bounded collection/morphology/
  barrier-band browsing. Details retain raw/prepared geometry and original JSON;
  one preview band and twelve lazy images per page preserve the earlier UX.
  Native and Node projection/pagination checks pass. Final-source desktop/mobile
  [browser acceptance](../outputs/procedural-challenge/browser-20261011T095305Z/browser-result.json)
  checks image absence, assets, paging, reload, direction, detail return and
  overflow. The [frontdoor receipt](../outputs/procedural-challenge/validation-20261011T095600Z-reports/frontdoor-validation.json)
  verifies all 578 generated pages/snapshots, source/executable binding,
  create-only/dangling refusal, excluded-file drift and rebound wrong statistics.
- The [next comparison proposal](procedural_challenge_comparison_plan.md) declares
  freeze prerequisites, composition/report gaps and a maximum 9,248-attempt/
  replay budget, with broad/stress/backend populations kept separate. It is
  separately scoped work; the current loop has zero flights/training, no held-out
  generation, no accepted publication and no server lifecycle changes.
- Final validation: all eighteen maintained developer checks pass, with the
  complete tracked/untracked source set unchanged during the
  [gate](../outputs/procedural-challenge/gate-20261011T100100Z-final/gate.log)
  ([result](../outputs/procedural-challenge/gate-20261011T100100Z-final/gate-result.json)).
  Native preservation, source/frontend verification, local browser acceptance
  and calibration interpretation are retained as separate evidence. No paths
  under the plant/runtime, controllers/planner or RL experiment changed in this
  loop; V1 fixture inputs and prior captures remain intact.

## Completion boundary

Authorized slices 1–4 and the separate comparison-budget proposal are delivered.
Native mission generation/admission/accounting, common evaluation, independent
Python consumption and neutral presentation keep their distinct owners. The
revised approach is broad conditional sampling with measured blind spots;
taxonomy organizes review and cannot certify complete coverage.

The next work begins with the [comparison prerequisites](procedural_challenge_comparison_plan.md),
including resolution-policy decisions, separate stress declarations and truthful
V2 incumbent/report composition. It is not an automatic continuation into tuning,
training, final held-out generation or a controller campaign. The current pilot
remains exposed development evidence with all original inputs/receipts preserved.
