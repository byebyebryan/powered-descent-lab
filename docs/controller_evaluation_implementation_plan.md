# Controller characterization implementation plan

[Evaluation design](controller_evaluation_design.md) · [Implemented controller](maneuver_controller_implementation_results.md) · [Development workflow](development.md)

October 21, 2026. **The unattended loop is complete.** The
[results](controller_evaluation_results.md) record software acceptance, exact
retained parity, corrected local characterization and the failed 99% confirmation
criterion. The [execution record](controller_evaluation_execution.md) retains
limits, prospective releases, technical invalidation and stage status.
This is the completed **standalone terminal PDG** pass. Its launch/commit
authority and allocations below are historical, not permission for another
campaign. The accepted finite-burn adapter and revised flat task have their own
[Poly Landing plan](poly_landing_controller_plan.md).

Start from the reviewed maneuver software `83303da` and evaluation design
`8adf2db`. Build the tools to measure the unchanged landing controller before
choosing another controller implementation or an RL task. One launch should carry
the work through implementation, internal review/repair, input audits, a cost pilot,
bounded characterization, activation comparisons and eligible fresh confirmation.
The checkpoints below are internal acceptance gates, not requests for another
user turn. The owner has authorized execution and reviewed checkpoint commits.

## Outcome and scope

The pass should make it possible to answer: given a declared airborne landing
population, where does immediate activation work, how does it fail, and what does
each fully verified attempt cost? Subsequent activation comparisons ask whether
a planner has a useful decision to make with that same controller.

Keep the public planner/controller split and the complete ordinary terminal
backend. No new executor/operator abstraction, guidance-law registry or package
is needed. Preserve native command semantics, configuration, cadence, original
fuel/deadline, frozen fixtures, proof schemas and accepted planner behavior.
Do not add a success-prediction admission gate or tune during characterization.

The software deliverables are a portable native-input set, versioned evaluation
recipes and prepared schedules, an input-only audit, and bounded batch execution
using the existing single-attempt evidence path. Include a frozen fresh-input
sampler and stage-level recovery so the loop can produce evidence as well as tools.
Real launch/transfer handoffs remain a separate integration pass.

## Source findings that determine the implementation

| Current boundary | Consequence for this pass |
| --- | --- |
| `resolve_terminal_matrix_runs` in [resolution.rs](../pd-eval-cli/src/resolution.rs) loads lane `ControllerSpec`s before expanding scenarios | Separate physical expansion from controller-descriptor assembly; input export must not load a controller config or instantiate a controller |
| The same file already has `resolve_terminal_matrix_scenario` | Reuse this physical construction, including adjustments, selectors, labels and terrain conditions; do not copy a second generator |
| The four retained clean/projected-error packs use a 90 s evaluation override on a 60 s base reachability horizon | Preserve both provenance values and the resolved 90 s native deadline; removing the override changes the task |
| [Physical identity](../pd-missions/src/lib.rs) already binds simulation, world, vehicle, initial state and mission | Reuse it; keep full scenario bytes and source occurrences alongside it because display metadata and seeds are excluded |
| Procedural collection V1/V2 require their own procedural recipes/resolved-mission records | Add a small native-scenario input format; do not invent procedural membership for retained terminal cases or alter existing collection schemas |
| [The composition](../examples/maneuver-policy/src/lib.rs) already executes one original Episode and reconstructs planner/controller behavior | Add scheduling around it; keep native execution and proof in that path |
| The current facade accepts airborne route-free scenarios at tick zero | Generate elapsed time through actual delayed activation; a reset handoff snapshot cannot establish original-mission integration |

The clean full pack declares `3 × 7 × 3 × 12 = 756` source slots; the
projected-error full pack declares `12 × 7 × 3 × 12 = 3,024`. Their combined
3,780 slots are a structural count, not a distinct-input count or an independent
sample size. The input audit must measure duplicates, including smoke/full
overlap and projected-error perturbations that resolve to the same physical case.
Preserve empty/reference, half/core and full/stress as original pack labels.
They do not automatically define this task's prospective operational core.

## Components and ownership

Names below describe proposed additions, not implemented entrypoints.

| Component | Proposed files | Owns |
| --- | --- | --- |
| Native input interchange | `pd-missions/src/scenario_inputs.rs`, export in `lib.rs`, focused tests | Versioned scenario-file references, physical and byte identities, source occurrences/alias groups, bounded loading and create-only storage; no controller or outcome types |
| Retained terminal adapter | `pd-eval-cli/src/terminal_inputs.rs`, scoped changes to `resolution.rs`, `lib.rs`, `main.rs`, `Cargo.toml` | Existing pack expansion into native inputs, resolver provenance and explicit export; add only a dependency on neutral `pd-missions` |
| Evaluation definition and audit | `examples/maneuver-policy/src/evaluation/{mod.rs,recipe.rs,prepare.rs,audit.rs,sampler.rs}` | Task version, population/roles, physical variations, frozen joint sampler, program schedule, budgets, lineage/exposure and input-only coverage |
| Bounded batch application | `examples/maneuver-policy/src/evaluation/{run.rs,results.rs,recovery.rs}`, additive CLI wiring/tests | Deterministic scheduled attempts, existing per-attempt bundles, complete result inventory, resource accounting and stage-level continuation |
| Integration | Existing development gate/tests and workflow docs | Dependency/source-binding checks and documented side effects after commands exist |

Keep `pd-missions`, `pd-run`, `pd-eval` and `pd-report` free of strategy imports.
The maneuver example must still have no dependency on `pd-eval-cli`, `pd-plan`,
`pd-planner`, legacy composition or Python. Controller configuration and
acceptance policy belong to the approach's recipe, not the neutral input set.
Do not generalize these modules into a new experiment framework during this pass.

## Checkpoint one Exact native inputs

First record the current resolver's ordered scenario/provenance fingerprints
for all four clean/projected-error smoke/full packs before changing expansion.
Use input resolution only. Retain an independently pinned aggregate digest and
readable landmark assertions for mass, velocities, selector order, conditions
and both horizons. Merely comparing two callers of the refactored function is
insufficient preservation evidence.

Extract controller-free expansion inside the evaluator. Its in-memory result
contains each complete original `ScenarioSpec`, source occurrence, selector
labels and resolved numeric parameters. It performs no output writes, controller
updates, Episode steps, forecasts, cache access or reporting. The legacy run
resolver continues to attach its controller descriptors with unchanged ordering,
validation and serialized results. Preserve behavior for reactive-terrain
conditions sharing the helper as well as the four export target packs.

Expose an explicit proposed `pd-eval export-terminal-inputs PACK --output NEW_SET`.
It accepts terminal-matrix packs only and rejects mixed/unsupported pack entries
explicitly. It has no default planner pack. Syntactic lane validation may remain;
controller names/configuration are provenance and are not resolved or opened.
A test with an unavailable controller config proves the exporter is independent
while ordinary legacy execution retains its existing config error.

The neutral input set stores full native scenarios per source occurrence and a
manifest binding their exact bytes and physical hashes. Preserve source pack/base
input hashes, resolver version/source identity, original run/lane/entry IDs,
seeds, selectors, parameters and expectation labels. Keep aliases visible rather
than changing scenario labels or silently deduplicating files. Source provenance
is data, not a controller selection instruction.

Reserve a new root; write its request/source declaration first, scenario files
next and completion manifest/receipt last. An incomplete export stays incomplete.
Bound loader file sizes, slot counts and paths; require regular files without
symlink traversal and prevent input/output overlap. Unknown schemas, altered
bytes, duplicate occurrence IDs and incorrect physical/alias identities fail.
Keep the small common DTOs independent of evaluator-specific Rust model types.
Record the producer executable hash and versioned resolver contract separately
from any runtime source snapshot. A compiled-source provenance claim additionally
requires a build-time ledger; the input set's byte/physical checks do not imply it.

**Exit:** pre-refactor fingerprints remain exact; export round-trips every
occurrence, physical input and provenance field; controller config files are
unneeded; create-only and tamper tests pass. No flight is needed for this checkpoint.

## Checkpoint two Prepared recipe and input-only audit

Add proposed `prepare-evaluation --recipe FILE --inputs SET --output NEW_PLAN`
and read-only `check-evaluation PLAN` to the maneuver example. Preparation
resolves every proposed case and program into an immutable schedule. It never
invokes a control update, steps an Episode, predicts a trajectory or reads flight
outcomes. Native scenario validation, initial geometry/observation queries and
the actual facade's semantic support check are allowed; keep those responsibilities
outside the neutral interchange layer.

Use two approach-owned versioned records:

- A recipe declares task/capability/backend identity, population and role rules,
  explicit levels/joint constraints/transforms, scheduled indices, exposure
  policy, programs and resource/acceptance policy. No implicit Cartesian sweep.
- A prepared plan binds the recipe and input-set bytes, every proposed/resolved
  case and disposition, scenario/program hashes, physical and parent identities,
  original native budgets, deterministic attempt IDs and duplicate/alias groups.

Start with a retained-regression recipe whose bounds come from the unchanged
packs. For structured development, support explicit named physical fields and
selected joint cases derived from native scenarios: clearance/descent/fuel,
offset/lateral speed/pad margin, attitude/rate/clearance, mass/fuel/deadline and
approach terrain/backstops. Use typed variations with units, not an arbitrary
JSON-path mutation language. Add correctly transformed translations/reflections
and label-only controls with explicit lineage. Keep fixed vehicle-family
parameters fixed. Initial zero fuel is an invalid native input; use positive
near-empty fuel to test exhaustion during execution.

Numeric ranges, levels, joint rules and counts are mandatory recipe data.
Derive a development proposal from the input audit and intended handoff
requirements before flight outcomes. If operational requirements are not yet
defined, mark the population as development characterization: do not manufacture
a qualified core from a box around the eleven software cases. The loop can define
a prospective synthetic local reference population and estimate performance for
that population, explicitly leaving intended mission-handoff adequacy unresolved.
Implement its joint sampler now. Freeze distribution, correlations, transforms,
validation/rejection rules and independent cohort streams before measured work;
release confirmation inputs only after the controller and policy are fixed.

The audit reports these quantities without a landing verdict:

| Audit | Required result |
| --- | --- |
| Inventory | All proposed slots, valid/invalid/unsupported dispositions and reasons; no disappearing slots |
| Identity/exposure | Distinct physical cases, source aliases, parent/transform groups and development/fresh overlap against supplied exposure inventories; unknown history stays unknown |
| Physical coverage | Marginal ranges, declared bins and selected joint-cell counts for the design's coupled factors; missing combinations remain visible |
| Native timing | Original horizon/command limits, 120/60/10 Hz cadence and absolute activation ticks; no reset elapsed clock or silent tick rounding |
| Schedule and cost bound | Per-role case/attempt counts, programs, declared duplicate execution rule, native-step bound and proof-pass estimate |

Semantic support does not imply recoverability. Valid challenging entries stay
scheduled. Invalid/unsupported diagnostic inputs keep their disposition without
fabricated flight evidence; an unsupported promised core case prevents a ready
core schedule. Any deduplication rule is prospective and preserves all aliases
and original denominators. Repeated programs or transformed copies do not
increase the independent confirmation count.

**Exit:** a small synthetic recipe and retained input set prepare/check
deterministically, all proposed slots are accounted for, audit and schedule
tampering fail, and a fresh checkout needs no ignored captures. Review checkpoints
one and two together internally, fix issues and continue into batch execution
without waiting for user review when these checks pass.

## Checkpoint three Bounded execution and honest results

Add proposed `run-evaluation --plan PLAN --output NEW_RUN`. Start with one worker
and deterministic ordering. Revalidate the full plan, input bytes, supported
capability, compiled source/executable and declared resource limits before the
first attempt. Use the existing captured run/save/verify path per scheduled
case/program. Extend the shared file-binding helper additively so the batch path
also binds the prepared plan, recipe and input-set manifest before decode and
after execution. Keep the existing two-file API compatible. The batch result
receipt ties each declared attempt to its actual bundle, physical/program identity
and native budget; scenario/program binding alone does not establish execution
of the declared schedule. Neutral evaluation/seal schemas stay unchanged.

Each program starts a fresh native Episode from the same original scenario.
Immediate, never and fixed ticks retain current semantics. Delayed trials
actually coast under the upright idle owner to the absolute activation tick;
fuel, attitude motion and original deadline are native state. Termination before
activation is an outcome, not an omitted trial. A diagnostic snapshot reset is
not an original launch/transfer handoff.

For retained direct/facade comparisons, use the same original native inputs,
Episode scheduler and ordinary default terminal configuration on both sides.
Compare complete commands, observations, clocks, fuel, contact and outcomes,
including failures, with the existing comparator's narrow identity/timing
exclusions. This is a separately scheduled parity panel, not an automatic full
pack campaign in ordinary tests and not a run of a different legacy controller
configuration merely because the pack names a lane.

Reserve one new result root and write the frozen schedule first. Each attempt
gets a result record; executed attempts get their own create-only bundle,
including failed landings. Unsupported diagnostics retain their input disposition
without a fabricated bundle; proof faults and incomplete I/O keep their evidence.
Do not overwrite, fill an old partial bundle or automatically retry a landing to
replace its result. Keep landing outcomes separate from proof status; a correctly
verified crash is a valid failed flight.
Scored success requires the complete native landing/mission/integrity/replay
tuple plus companion binding and both reconstructions.

Continue after an ordinary physical miss. Quarantine the affected stage on
source/input drift, unexpected harness errors, direct/facade mismatch or
proof/integrity/reconstruction failure; retain the failing slot and mark remaining
slots not started. The recovery rules below distinguish a repairable tooling
fault from a controller limitation or an exhausted loop budget. Keep scientific
acceptance separate from execution validity: a completely verified all-crash
schedule is an executed evaluation with failed landing performance.

Use finite attempt/native-step bounds and declared wall-time/storage caps. A
single supervised attempt subprocess lets the parent enforce a deadline across
execution and proof. Reserve each attempt's declared maximum bytes before launch
and enforce that bound on serialized artifacts before writing, including bundle
and result files; a post-write size check cannot enforce a storage ceiling.
Check aggregate remaining caps before starting each attempt. A hard interruption
retains incomplete evidence. Record completed, interrupted and not-started slots
explicitly, and prevent a partial campaign from claiming complete coverage.
Support stage-level continuation under the exact-identity rules below; durable
mid-flight resume and automatic promotion remain outside scope.

Count original execution and all assessment/save/post-save proof work. The
current CLI path is one original episode plus seven proof/reconstruction passes;
later verify/report commands add three passes each. Avoid an extra per-attempt
CLI verify on top of that path. Record actual steps, backend updates, planner
calls, wall time and retained bytes. Internal forecast work and unavailable CPU
timing stay unavailable. Source additions under existing participating `src/`
trees must remain covered by the compiled-source ledger and runtime scope.

Produce machine-readable per-attempt rows and summaries by population/stratum,
without pooling regression, diagnostics and confirmation into one rate. Include
fuel/time/contact metrics where available and original native reasons alongside
any derived failure grouping. Preserve existing rich detail rendering for
explicit inspection; report generation/publication is not a batch side effect.

**Exit:** tiny temporary test schedules exercise successful execution, valid
failed landings, unsupported inputs, proof faults, changed inputs, existing
destinations, interrupted writes and cap exhaustion. Every declared slot has a
status, existing one-case behavior remains compatible and no maintained report,
selector or server is changed.

## Review and validation

Keep numerical flights confined to the existing software panel and tiny
integration tests until a measured allocation is declared. The full retained
input expansion is input-only test data; full direct/facade execution remains
an explicit campaign. No ordinary check depends on retained `outputs/` evidence.

At each checkpoint run the affected Rust tests and dependency checks. Finish
the software pass with `rtk proxy node scripts/check-planner-development.mjs`:
workspace/CLI boundaries, formatting, strict Clippy, documentation and Node
checks must pass. Preserve the retirement checker and optional status of the
retained 44-case planner parity. Update development/evaluation workflows only
when the proposed commands exist. Review source hashes, fixtures and unrelated
worktree bytes against the starting inventory before claiming completion.

## Unattended scope and proposed limits

The loop owns implementation, ordinary tests, internal source review, input-only
population design, prospective phase declarations, measured runs within these
limits, failure analysis and a final reviewable record. It may fix tooling and
repeat necessary checks on its own. It does not need to stop after building the
exporter or ask permission at every phase boundary.

Proposed defaults for one launch are twelve hours of total wall time, with the
last hour reserved for validation and closeout; one native worker; at most
40 GiB of new input/capture/report artifacts plus 20 GiB of new build/source
material; and a 50 GiB free-space floor. Recheck available space at launch and
lower limits if needed, without deleting existing evidence. These are ceilings,
not an instruction to fill time or disk. All retries, diagnostic reproductions,
replays, reports and repair epochs count against the same limits.

Keep the ordinary backend and native plant frozen throughout. Alternate backend
work, tuning, new success filters, RL training and real mission-prefix integration
are not automatic fallbacks when this controller fails. Preserve unrelated work,
accepted defaults, report selectors and server state. Local report artifacts use
new destinations and maintained rich templates; publication is not part of the loop.

## End-to-end stage schedule

| Stage | Work and ceiling | Automatic transition |
| --- | --- | --- |
| A: Implement and review | Complete the three software checkpoints, frozen joint sampler and recovery tests; run the maintained gate | Repair owned tooling until checks pass or the global cutoff requires closeout |
| B: Audit and freeze | Export all retained inputs; audit aliases, semantics and coupled coverage; freeze physical bounds, roles, full candidate lists and cohort rules before flights | Write immutable task/recipe declarations and continue |
| C: Cost pilot | At most twelve original native executions, including direct-reference probes, light/heavy, difficult and delayed/never cases; count every additional proof pass | Use cost and bytes only to allocate later counts; keep pilot outcomes as exposed development data |
| D: Retained parity | Target all 3,780 full-pack direct/facade pairs; cost-limited alternatives are the fixed 945-slot or 315-slot panels described below | Retain misses, require exact adapter parity and report allocated versus full source coverage |
| E: Structured recovery | Up to 256 development cases with immediate activation, plus 64 separately scored frontier/unsupported diagnostics | Produce coupled-factor outcomes and failure analysis without changing the population |
| F: Activation | Up to 64 original development cases, each with the same seven programs | Compare paired outcomes, actual activation ticks, fuel/time and hindsight best-of-grid; do not count programs as independent missions |
| G: Fresh confirmation | A fixed 300 or 600 independent missions from one frozen primary local population, using immediate activation | Execute once, then compute exact confidence limits and the declared verdict; failure does not trigger more sampling |
| H: Closeout | Validate inventories, review code, summarize coverage/cost/failure mechanisms and render bounded local rich details | Finish with evidence and a concrete next recommendation, including negative or incomplete results |

Before the pilot, fix task ranges and role rules, a balanced ordering of all
candidate cases, and the seven activation programs: immediate, never, and fixed
absolute ticks `12, 60, 120, 240, 480` (0.1, 0.5, 1, 2 and 4 s at 120 Hz).
These are a finite development timing probe, not a claim to cover every useful
activation time. All are on the existing 10 Hz decision grid. Retain programs
that terminate before activation and do not move their ticks after seeing outcomes.

Use up to thirty minutes for the cost pilot. After it, freeze all later phase
counts and limits before starting stage D. Divide remaining measurement time
before the closeout reserve in fixed proportions: 30% retained parity, 20%
structured recovery, 20% activation and 30% confirmation. Use twice the largest
measured relevant pilot cost/retained size as a planning estimate, with separately
measured reference-execution cost for parity. These estimates do not replace
hard subprocess and storage limits. Unused time may flow forward without
increasing a frozen phase's sample count.

Choose the largest affordable panel from these predeclared choices:

- Retained parity: 3,780, 945 or 315 source slots. Each smaller panel covers all
  fifteen condition/payload entries, seven arcs and three velocity bands, using
  respectively seeds `0,1,6` or seed `0` from the full definitions. Compare the
  corresponding smoke aliases during input audit; do not relabel them as fresh.
- Structured recovery: 320, 160 or 80 total cases, with four development cases
  per diagnostic case. Preserve the declared balanced order and report which
  interaction cells the allocated prefix actually covers.
- Activation: 64, 32 or 16 original cases, each with all seven programs. Choose
  the balanced case panel before its flight outcomes; do not select only observed
  wins or failures from characterization.
- Confirmation: 600 or 300 independent missions with immediate activation.
  Freeze N from cost alone. If neither fits, mark confirmation unallocated and
  preserve the reliability question as unresolved; do not weaken the population
  or confidence criterion to fit the time left.

If no minimum panel fits, mark that stage unallocated and continue eligible
independent work. A selected panel's interrupted tail is incomplete, not a
smaller completed panel. Keep every full candidate slot in the allocation ledger
as selected, intentionally unallocated or not started. Report full-pack parity
only when the full allocation completes; smaller panels retain their own names.

The initial confirmation policy stays immediate even if development suggests a
better fixed tick. This answers current local-control competence without selecting
and confirming an extra policy in the same allocation. Record alternatives as
follow-up candidates. Pilot and development failures do not cancel confirmation
based on performance; eligibility depends on a frozen valid population, sound
software and sufficient resources.

For one explicitly declared primary local population, the working confirmation
target remains a one-sided 95% lower confidence bound of at least 99%. At N=300,
zero failures give a lower bound of about 0.99006. This follows the
[exact-binomial method](https://itl.nist.gov/div898/software/dataplot/refman2/auxillar/exacbino.htm)
under the declared independence assumptions. Stratified rows remain visible;
this allocation does not promise the same bound for each stratum or simultaneous
claims. If mission requirements demand those claims, freeze the larger allocation
and multiplicity treatment before execution or report insufficient evidence.
Structured/transformed panels cannot supply the independent confirmation N.

## Recovery without changing the experiment

At each measurement epoch retain its exact executable and participating source
tree at their bound paths. The current reconstruction contract includes those
paths; an archived binary alone does not preserve it. Prepare isolated source
epochs rather than editing a measured tree or borrowing a foreign worktree.
Record source, binary, configuration and input identities in the run ledger.

Maintain immutable per-attempt completion records and a stage ledger. After a
supervisor interruption, create a new continuation root referencing the old run.
Reuse completed attempts only after checking exact task, recipe, scenario,
program, native budget, compiled source/executable and bundle identities, with
the required verification charged to the budget. Reuse the original outcome;
a failed landing is never retried to replace it. An incomplete attempt remains
incomplete; any diagnostic rerun gets a new identity and cannot replace the
scheduled result. Do not append missing files to an old bundle or restore an
Episode midway through flight.

For an unexpected tooling/proof fault, quarantine the affected stage, diagnose
the cause and allow at most two repair epochs within the global budget. A repair
requires a demonstrated cause, a relevant regression check, rebuild, focused
parity/integrity tests and the maintained gate before new measurements. Use a new
source epoch/output root. Preserve earlier evidence and state explicitly which
results remain comparable; never merge confirmation across code epochs. Permit
at most one confirmation restart, only when a demonstrated defect invalidates
the phase's input, scheduling, outcome or proof semantics. Record the affected
scope and technical cause before allocating its replacement. Use fresh inputs
and the same fixed N/criterion within the remaining global budget. A valid
scientific failure never permits that restart.

If the cause requires changing the backend/native semantics, remains unresolved
after the repair limit, or cannot be repaired within the closeout cutoff, retain
the evidence and finish with the fault and remaining work. Do not keep rerunning
the same failed command or silently skip a proof check to keep the loop moving.

## Completion and final decision

The final record must include reviewed software/check results; frozen recipes
and exposure inventories; proposed/allocated/executed/verified counts for every
stage; native outcomes and failure mechanisms by role and coupled state; actual
fuel/time/contact metrics; all proof dispositions and costs; and preserved paths
to source epochs, binaries, captures and local rich reports. Bound report detail
rendering to twelve selected attempts and account for its additional proof work.

Return separate verdicts for software readiness, observed local recovery,
fresh-population reliability, activation decision value and whole-mission
integration readiness. Negative performance is a valid measured result. Budget
exhaustion or unexecuted required work remains partial completion; it is not
evidence that the controller passed or that an unfinished implementation is done.

Recommend RL only if a measured reliable region and useful state-dependent
activation or declared fuel/time tradeoffs survive simple deterministic
comparisons. Otherwise return the concrete controller weaknesses or adequate
simple policy and a focused next pass. A revised core/controller needs a new
version retaining earlier failures. Whole-mission adequacy still requires the
later prefix/command-owner adapter with actual native handoff parity.
