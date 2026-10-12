# Shared preparation for controller and RL development

[Documentation home](README.md) · [Architecture](architecture.md) ·
[Current development baseline](procedural_challenge_baseline.md) ·
[Plan review](controller_rl_preparation_review.md)

October 12, 2026. Status: reviewed proposal; execution has not started.
The owner requested a concrete plan and review before starting shared preparation
and then splitting current-controller and independent RL development.

## Intended outcome

Produce one committed, validated platform checkpoint from which two worktrees
can develop independently against the same declared physical tasks, input
cohorts, native success contract and rich reports. The split does not depend on
an improved incumbent, an RL landing or complete terrain coverage.

The shared plant, open-agent interface, versioned mission generator, native
evaluation/replay and report templates already exist. The 3,311 exposed V2
development missions have a verified incumbent baseline: 1,266/1,645 layered
and 1,290/1,666 value landings under the reflected cap-24 reference. Preserve
those captures and the [unified review](report_unified_procedural_results.md).

This plan proposes a bounded **preparation** allocation. It does not launch the
older [9,248-attempt comparison proposal](procedural_challenge_comparison_plan.md),
controller tuning, optimizer training or final-test evaluation. The next launched
scope must adopt an execution declaration before creating inputs or attempts.
The proposed defaults are at most 6,016 new proposal slots, 48 integration attempt
slots, zero training transitions and zero final-test flights. Skip the optional
fresh validation reference unless the launch declaration explicitly includes it.

## Decisions for the common contract

- Keep the existing nominal vehicle, Earth gravity, 6,300 kg initial fuel,
  source-pad-rest start, two-pad landing task and 90 s original horizon. Preserve
  120 Hz physics and 60 Hz native command boundaries. Agent decision cadence,
  action mapping and internal planning limits remain approach-owned and declared.
- The sealed **4 m piecewise-linear physical terrain** is authoritative. Finer
  noise samples are input diagnostics, never a replacement collision surface or
  a continuous-terrain guarantee. A finer physical world needs a separate recipe
  revision and comparison; it is outside this preparation.
- Keep the current broad V2 recipes, geometry/admission/morphology versions and
  bounded pad edits. Every admitted broad mission qualifies independently of
  controller outcomes, including mixed and unclassified morphology. Synthetic
  terminal/simple-transfer cases remain diagnostics; authored waypoint phases
  do not become a common mission contract.
- Layered Perlin is the primary broad population; value noise is a separately
  scored backend contrast. Both are already exposed to the designer. Neither
  their combined navigation nor fresh seeds establish an unseen-backend claim.
- Success requires the native physical/mission/execution/integrity/complete-
  replay tuple. Airborne stops, crashes, unsupported inputs, input errors and
  unfinished scheduled attempts retain their dispositions and denominators.
- Preserve the reflected acquisition-plus-terminal-safety cap-24 reference as
  the primary incumbent. Keep raw leftward preflight rejection as compatibility
  evidence. Do not change its settings or implementation during preparation.

## Work order and deliverables

| Stage | Shared work | Exit evidence |
| --- | --- | --- |
| P0 | Reconcile scope, resolution diagnostics and reference identity | Committed execution declaration, physical contract and retained-reference receipt |
| P1 | Reconcile exposures and declare recipes, roles, seeds and schedules | Exposure inventory and complete serialized collection requests committed before generation |
| P2 | Generate and verify the initial cohorts and stress inputs | Sealed manifests, full native verification, cross-cohort identity audit and stress shortfall record |
| P3 | Complete strategy-independent V2 metadata, evidence and report interoperability | V1/V2 fixture tests, excluded-input drift checks, truthful failure reports and Python/native parity |
| P4 | Exercise both composition paths on exposed inputs; optionally measure fresh validation reference | Bounded capture inventories, fresh independent verification and rich browser/report acceptance |
| P5 | Record the common checkpoint and two branch handoffs | Passing gates, artifact/source index, ownership rules, isolated build/output setup and readiness verdict |

P3 implementation can proceed alongside input-only P1/P2 work. P4 follows both.
All decisions affecting input membership precede their corresponding generation
and every controller outcome. Commit reviewable slices; retain failed artifacts.

### P0: establish the fixed physical task

Read the seven retained finer-reference warnings and classify their mechanisms.
Use at most 128 additional declared development proposals/reference checks to
test independent representatives if needed. Preserve the physical grid and
existing pilot. Diagnose a generic reader/metric defect before freezing if one
is found; do not repair the seven named worlds or change broad recipes based on
controller scores. Record unresolved fine-noise differences as limits of the
declared 4 m world.

Bind the actual incumbent implementation/configuration, source files and built
binary; identify the original reference captures and integrity receipts. A
later presentation commit does not relabel the original flight checkpoint.

### P1/P2: separate cohorts and exposure

Use the existing 3,311 evaluated inputs as the fixed development regression set.
Other previously generated, inspected or consumed inputs retain their original
roles and exposure histories. The larger pilot can supply additional development
inputs, but they need an explicit schedule and have no inherited flight result.

Proposed new input ceilings:

| Collection or diagnostic | Proposal slots | Role and use |
| --- | ---: | --- |
| Additional resolution QA | At most 128 total | Development; input-only |
| Stress recipe support pilot | At most 256 total: 128 per candidate | Development; input-only recipe comparison |
| Layered broad validation | 512 | Validation; reusable candidate evaluation |
| Value broad validation | 512 | Validation; separate backend rate |
| Initial layered stress collection | 512 | Development stress; separate diagnostic denominator |
| Layered broad final test | 2,048 | Held-out; no policy execution or individual inspection during preparation |
| Value broad final test | 2,048 | Held-out; separate backend rate and the same access restriction |
| **Total maximum newly proposed inputs** | **6,016** | Proposal ceilings, not promised admitted counts |

Keep the eight-attempt structural limit. Admission/exhaustion, exact duplicates
and stress exclusions consume their original slots; never refill to achieve a
desired accepted count. Preserve every resolved proposal, including exclusions.
Training transitions are zero. Future training/curriculum streams need their own
declared seed/index namespaces and must exclude these reserved evaluation inputs.

Reconcile the V2 pilot, V1 cohorts/characterization, prior training worker logs,
evaluation captures, usage receipts and other retained procedural exposure.
Record hashes where retained and seed/recipe/index namespaces where that is the
available evidence. State inventory gaps; an absent record is not evidence of
non-exposure. Conservatively reserve known-exposed and unresolved namespaces
against new held-out use. The exposure claim is limited to the declared audited
history, not an assertion about every unrecorded past input. Select unused seed
namespaces only after this inventory. Check
new admitted physical identities against known prior exposures and across every
new role. A collision fails the cohort freeze: retain the candidate and receipt;
do not edit its manifest, silently drop the collision or draw a replacement.

Commit full requests before generation, including role, recipe, backend, metric,
admission, morphology, population, seed and ascending index schedule. Full
native verification covers admitted and excluded inputs. Final-test preparation
permits structural/hash verification only: no policy runs, individual previews,
counterfactual inspections, public input report or training/checkpoint-selection
access. Record any accidental exposure and retire that final-test claim.

### Initial stress declaration

Use four geometry-only panels with inclusive constraints and quota 64 each:

| Panel | Prepared geometry constraints |
| --- | --- |
| Wide source, close departure obstruction | Source width 64–80 m; departure barrier distance 0–40 m |
| Narrow target, close approach obstruction | Target width 24–32 m; approach barrier distance 0–40 m |
| Tight significant crests | At least two measured crests; minimum crest spacing at most 40 m |
| Long sustained obstruction | Longest interval at least 800 m wide above the 100 m chord barrier |

Missing measurements never match numeric constraints. All selected stress inputs
still pass the unchanged raw/prepared challenge rule. Panels can overlap; the
unique union is at most 256 inputs and is the collection denominator.

Before the support pilot, serialize two candidate layered recipes: A is the
current broad recipe; B keeps its physical envelope and other controls but uses
two octaves and structure wavelength 100–400 m. Validate both declarations
against the native sampling/recipe guards before generating any proposal.
Compare at most 128 proposals per candidate, with the same panel definitions.
Select by largest minimum panel matching count, then total matching memberships,
then A on a tie. Count unique qualifying proposals before quota capacity, excluding
duplicate receipts; panel memberships are not additive mission counts. This is
input-only support selection, not a solvability test.
Freeze the selected recipe before the final 512 stress proposal slots.

Record every unfilled quota and structural explanation. Empty/sparse panels
remain explicit coverage gaps; they do not authorize more proposals, threshold
relaxation or an assertion of stress completeness. The initial split can proceed
with those documented gaps for broad-development work. Broader stress or shift
claims require a later versioned extension.

### P3: complete the V2 composition path

The native bridge already loads and executes V2. The remaining gap is primarily
in the RL application's collection/report composition:

- Replace inferred V1 direction/stratum fields with validated native metadata.
  V2 uses backend-specific family identity, physical direction, measured geometry
  and receipt-derived morphology, with no fabricated V1 stratum.
- Reuse the native versioned collection's immutable file inventory, including
  **excluded proposal files**, for once-per-campaign evidence. Bind selected
  inputs per episode. Validate source bytes against the sealed collection and
  bind every new helper in the declared source scope.
- Make independent verification check projection metadata against original
  versioned inputs, as well as checking physical identity and native outcomes.
- Populate previews/endpoint context from saved native execution and preserve
  rich detail payloads. Input/unstarted/unsupported states cannot invent a flight.
- Handle unavailable metadata truthfully for input failures. Distinguish it from
  verified unclassified morphology and valid missing obstacle/spacing values.
  If the current strict report DTO cannot represent it, add the smallest explicit
  status representation with V1 compatibility tests; retain the complete failed
  schedule and common presentation rather than supplying fake measurements.

Prefer a neutral metadata helper and thin adapters over copying incumbent
projection logic into RL. `pd-missions` owns input semantics; `pd-python` exposes
neutral consumption; callers supply the existing `pd-report` DTOs. No neutral
crate or RL module may import executable incumbent strategy packages. Preserve
V1 aliases, persisted inputs/captures, old reward/action designs and replay rules.

Each comparison is declared for one **role and population**. Its two backend
families may share a mission tree. Broad, stress, development, validation and
final-test results remain separate comparison views and denominators. Bind
recipe/admission/manifest identities to the comparison declaration and reject
incompatible composition; the current backend-only renderer is not permission
to pool broad and stress campaigns into one family rate.

The RL training sampler's curriculum, features, actions, reward and optimizer
redesign remain RL-branch work. This stage must support fixed V2 inputs without
changing the historical V1 training defaults or launching optimization.

### P4: integration and reference measurement

Select and commit 16 **already exposed** integration inputs: eight per backend,
covering both directions, mixed/unclassified inputs and the challenge paths.
Select using input metadata, never reference landing outcomes.

Run exactly one attempt per input through each of three named composition paths:
the unchanged reflected reference, raw compatibility reference, and Python/RL
evaluation with a pre-bound deterministic test policy. Its descriptor and source
must identify the actual untrained policy, with no invented PPO checkpoint
provenance. A launch can instead name an existing frozen checkpoint, whose
identity and compatibility must pass preflight before any smoke outcome.
No optimizer steps or checkpoint selection occur. Ceiling: **48 scheduled attempts and at
most 48 captured executions/independent replays**; raw leftward unsupported
dispositions remain scheduled. Source-fixture fault injection covers invalid
input, initialization/capture failure and excluded-file drift without altering
real sealed cohorts.

The smoke passes on complete accounting, identity, integrity, replay, metadata,
no policy metadata leakage, and rich navigation/preview acceptance. It has no
landing-rate target. Use an owned disposable QA server/browser if needed; leave
the user's existing server and accepted report selectors untouched.

A fresh validation reference is optional and skipped by default: one reflected
incumbent attempt per admitted validation input, with at most **1,024** scheduled
attempts/captures/replays across both backends. It uses the already frozen
reference and makes no selection based on outcomes. Existing development proof
is sufficient for the split if this optional stage is skipped. Declare the
option in the execution request before flight. Final-test flights remain zero.

Total preparation ceiling: **1,072 scheduled study attempts**, one fresh neutral
verification per executed capture, **zero training transitions**, and zero final-
test attempts. Ordinary temporary-fixture tests remain software checks, not new
measured campaign allocations. Set an eight-hour wall ceiling when launching;
retain partial outputs and an incomplete stage at that boundary.

### P5: checkpoint and branch handoff

Run the maintained development gate, relevant Python/native tests, source-fixture
V1/V2 compatibility/error checks and actual smoke report browser acceptance.
Retained 44-case numerical parity stays an explicit opt-in, not a dependency on
ignored local evidence for ordinary gates. Preserve original source/report hashes.

The final handoff binds the shared source commit, schemas, dependency lock,
actual native build/ABI receipts, reference descriptor, collection requests and
manifests, exposure/identity audit, preparation budgets, verification receipts,
report links and unresolved coverage limits.

| Owner after the split | Files and responsibility |
| --- | --- |
| Shared platform maintenance | `pd-core`, `pd-run`, `pd-missions`, `pd-eval`, `pd-python`, `pd-report`, shared workflow/docs and evaluation contract |
| Incumbent branch | `pd-control`, `pd-plan`, `pd-planner`, `pd-benchmark`, existing `pd-cli`/`pd-eval-cli` composition and approach-specific experiments |
| RL branch | `experiments/rl`, any independently owned agent package, sampling/curriculum, features/actions/rewards, optimizer and checkpoints |

Create both worktrees from the common checkpoint only after the readiness verdict.
Each has its own target/build directory, Python environment/native extension and
new output namespace. Copy and verify immutable cohort assets when paths change;
never silently rewrite path-bound native receipts. Give usage journals distinct
IDs and reconcile exposure before final evaluation. Keep final-test inputs out
of the everyday branch workflow. Shared changes land through an explicitly
reviewed platform change consumed by both branches.

Each branch gets a separately declared experiment hypothesis, wall/attempt or
transition budget, candidate-selection rule, failure reporting and final-test
release rule. Algorithm development and new training start under those later
contracts, not under this preparation allocation.

## Readiness verdict

The split is ready when the common physical/scoring contract is recorded,
development/validation/final-test cohorts are sealed and exposure-audited, initial
stress accounting is honest, V2 composition works through both paths, integrity/
replay/report checks pass, and the handoff has reproducible owners and artifacts.

It does not require filled stress quotas, proof that every mission is solvable,
controller improvement, an RL landing or a fresh final-test score. Unreconciled
evidence, incompatible metadata or inability to represent genuine metadata
unavailability is a gate failure. Expected failure fixtures pass only when their
dispositions are represented and verified truthfully; they do not weaken the
frozen input or landing contracts. A later launch stops at this split point.
