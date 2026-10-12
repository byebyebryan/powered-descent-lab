# Overnight RL experiment plan

[Branch handoff](controller_rl_development_handoff.md) ·
[Earlier RL results](procedural_learning_results.md) ·
[Planning review](rl_overnight_experiment_review.md)

October 12, 2026. Proposed eight-hour experiment, **not launched**. A later launch
uses this contract, records its absolute deadline and exact requests, and commits
validated implementation milestones. Planning itself allocates no training,
missions, flights, report editions or server operations.

## Question and expected outcome

Can an independent policy discover native target landings through a curriculum
of complete procedural missions, then transfer that competence to challenging V2
terrain? Compare curriculum and uniform training while holding PPO, observations,
actions and the corrected v3 objective constant. If discovery still stalls, allow
one predeclared investigation of the control representation.

Here uniform means challenging H-only sampling. The comparison measures the
adaptive easier-to-harder sampler as a whole, including easier exposure. It does
not isolate stage ordering from that exposure; a matched static-mixture control
would be a later experiment.

The earlier nine allocations consumed 72,032,256 transitions; each selected seed
landed 0/2,048 gentle V1 held-out missions. Reward revisions changed fast crashes
into slow horizon stops, often 56–64 m above touchdown. Those observations justify
investigating discovery/control. They do not identify a single proven cause.
The unchanged incumbent's current challenging-development rates are 77.0%
layered and 77.4% value; that exposed population is distinct from new validation.

An overnight success is a completed, audited experiment with a clear positive or
negative result. Establishing repeatable challenging-terrain landings would be
valuable progress; matching the incumbent or producing a general controller is
not an assumed outcome. Do not spend the remaining night on undeclared variants.

## Ownership and fixed task

Use `../powered-descent-lab-rl-v2`, branch `dev/rl-v2-20261012`, starting at common
checkpoint `51c143c`. The controller worktree and incumbent implementations stay
unchanged. The main checkout owns this plan/review; record their launch revision
in the RL worktree without merging unrelated controller work.

RL implementation belongs to `experiments/rl` and a new owned experiment study
directory. Use additive, explicitly named V2 training/evaluation entry points;
retain historical v1/v2/v3 defaults and frozen artifacts. Evaluation must accept
the new design/configuration and bind every imported agent/sampler/adapter source
and any normalization state. An installed native/source/ABI receipt must match
the actual worktree environment. No executable incumbent dependency or fallback
belongs in the RL policy, reward or sampler.

Keep complete source-pad-rest to target-pad missions, nominal Earth plant,
6,300 kg initial fuel, original 90 s horizon, authoritative sealed 4 m terrain,
120 Hz physics and 60 Hz commands. Preserve native contact, rotation, throttle,
fuel, clock, capture and replay semantics. No airborne curriculum resets,
fabricated terminations, authored waypoint acceptance or outcome-filtered inputs.
Both physical travel directions remain in every stage; mirroring is an explicit
approach transformation, with direction a reporting facet.

Neutral platform changes are limited to a concrete orchestration/provenance defect
needed to execute this contract. Review and validate any such fix as a separate
shared commit before consuming it; do not tune plant, admission or benchmark
rules to accommodate the learner. If a broader platform change is required,
preserve the blocker and finish the bounded diagnosis.

## Wall, compute and evidence ceilings

Start the eight-hour clock when the experiment is launched, including preparation.
Declare the UTC deadline and stop starting training at least 90 minutes before
it. Run optimizer allocations serially with a fixed worker count; paired arms
use the same hardware and optimizer settings. No cloud allocation or dependency
upgrade is proposed. Existing pinned CPU dependencies are adequate for the pilot.

| Phase | Maximum planning allowance | Required exit |
| --- | --- | --- |
| Implementation, input sealing and contract checks | 90 min | Runnable V2 experiment, verified banks/schedules, passing relevant checks |
| Timing/exploration pilots | 30 min | Measured throughput, telemetry, declared feasible main allocation |
| Uniform/curriculum comparison | 180 min | Paired three-seed results at the same transition count |
| Conditional control-interface comparison | 90 min | One additional representation, paired schedules and honest censoring |
| Freeze, native verification, reports and final review | 90 min | Bound checkpoints/evidence, complete accounting and readable result |

These are ceilings, not required waiting times. Finish early when the declared
questions are answered or an early-stop rule fires. Failed stages and incomplete
allocations retain their receipts; unused time does not authorize extra designs.

| Allocation | Maximum optimizer transitions |
| --- | ---: |
| Two fresh timing pilots, 262,144 each | 524,288 |
| Direct-action uniform/curriculum, three seeds each, 8,388,608 each | 50,331,648 |
| Conditional velocity-action uniform/curriculum, three seeds each, 4,194,304 each | 25,165,824 |
| **Absolute total** | **76,021,760** |

Use rollout-aligned counts: with 16 environments and 512 steps, a rollout is
8,192 transitions. Count actual optimizer transitions, including pilots and any
resumed work; never silently add PPO rounding beyond the ceiling. The pilot may
omit the extension/optional phase or decline main training if the initial matched
pair cannot fit. A smaller diagnostic allocation is labeled timing-censored and
cannot trigger validation. The pilot cannot increase these maxima.
Save optimizer, sampler and RNG state needed
for genuine continuation; a new optimizer is a new allocation.

Additional ceilings are 12,416 new proposal indices, 32,768 fixed-panel summary
attempts, 4,096 measured native capture attempts and one fresh saved-command replay
per executed capture. Training trajectories are separately counted in worker
ledgers. Source-fixture software checks remain separate and cannot be reported as
measured performance. Retain native structural attempts and all exclusions.

Limit new output growth to 80 GiB and leave at least 40 GiB free. Measure proposal
and capture sizes in the pilot before starting a full campaign. Preserve existing
evidence; reaching a resource ceiling ends allocation rather than triggering
cleanup. Partial campaigns remain visibly incomplete with their full predeclared
schedule. Do not shrink their denominator after seeing results.

## Inputs and curriculum

Seal inputs before optimization. Use the existing native V2 generator and readers;
the experiment owns scheduling and training-bank orchestration, not another noise
implementation or challenge-admission rule. Proposed namespaces below are reserved
only after the launch audit confirms that they are unused.

| Bank | Layered / value seeds | Proposals per backend | Recipe difference from V2 defaults |
| --- | --- | ---: | --- |
| C0: landing discovery | 20261012061 / 20261012062 | 1,024 | Travel 150–450 m; both pads 40–80 m; macro amplitude 10–40 m; structure amplitude 20–80 m; warp amplitude 0–20 m; feature amplitude 5–20 m |
| C1: transfer bridge | 20261012063 / 20261012064 | 1,024 | Travel 300–1,200 m; both pads 32–80 m; macro amplitude 80–320 m; structure amplitude 120–600 m; warp amplitude 0–60 m; feature amplitude 20–80 m |
| H: challenging training | 20261012065 / 20261012066 | 4,096 | Exact frozen broad V2 recipe/admission for each backend |
| C0 diagnostic | 20261012067 / 20261012068 | 32 | C0 recipe, development-only, disjoint from training |
| C1 diagnostic | 20261012069 / 20261012070 | 32 | C1 recipe, development-only, disjoint from training |

Keep all other recipe fields, including sampling resolution and octave controls,
unchanged. Validate complete serialized recipes natively before generation. These
control ranges predict easier sampling; realized geometry must confirm the stage
description. They do not certify solvability or ordered difficulty for every input.

C0/C1 banks retain all structurally resolved proposals, including gentle missions;
they are explicitly training/diagnostic populations without challenging benchmark
status. Retain complete native proposal results, recipes, scenario/physical hashes
and rejection receipts in create-only files. Validate saved proposal bytes against
one native regeneration before use, then bind the immutable bank inventory. That
is at most 24,832 generation requests including the check, with native bounded
structural attempts; repeated episodes load the sealed bank instead of regenerating
it. Do not duplicate native geometry/admission calculations in Python.

H uses native V2 broad collections: train only on admitted entries and retain
insufficient-challenge, duplicate and exhausted proposals in the complete ledger.
No accepted-count refill. Audit **all resolved proposals**, including exclusions,
against the shared exposure inventory and every reserved cohort before training.
An exact cross-role collision stops sealing; it is not silently removed or
replaced. Hash-check all banks before/after allocations and record every reuse.

Both arms use the same H bank, fixed 80% layered / 20% value episode-reset weights,
and balanced left/right sampling within each backend/stage. Reuse is allowed and
recorded; report both episode and transition exposure shares. Finite-bank training
is not an unseen-stream claim. Features receive
physical observations/terrain only, never seed, backend, labels or admission.

The uniform arm samples H throughout. The curriculum arm begins with the stage
mixtures below. Mixing is an episode-reset decision; ongoing missions are never
restarted when the mixture advances. Keep a challenging component from the start.

| Curriculum stage | C0 / C1 / H mixture | Advance after two consecutive diagnostic checks |
| --- | --- | --- |
| Discovery | 70% / 20% / 10% | C0 native summary success ≥50% and C1 ≥15%, separately on each backend |
| Transfer | 20% / 40% / 40% | C1 success ≥40% and H ≥5%, separately on each backend |
| Challenge | 10% / 20% / 70% | Final mixture; no further advancement |

These are declared experiment thresholds, not qualification criteria. A seed
that does not pass stays in its current stage. The uniform arm evaluates the same
diagnostics at the same counts, but they do not change its sampler. No reward or
horizontal-distance threshold advances the curriculum.

The recurring 256-input development schedule contains all 64 C0 and 64 C1
diagnostics plus 64 layered and 64 value inputs from the already exposed V2
development population. Select the H panel before flights by canonical index,
32 inputs per physical direction/backend, without reading controller outcomes.
Persist exact physical hashes and append usage receipts. Retain mixed/unclassified
geometry. Geometry labels organize later review, not panel admission or sampling.
Any structurally exhausted diagnostic proposal keeps its scheduled failure slot;
do not refill or reduce the panel denominator.

## Learning designs and decision rules

The initial pair preserves the existing 64 physical features, normalized direct
throttle/attitude actions, 10 Hz policy cadence, 0.75 s upright launch primitive
and corrected v3 discount-aligned terminal potential. The launch primitive consumes
the original time/fuel; it is an approach choice. Preserve PPO's existing settings:
512 steps, batch 512, five epochs, learning rate 3e-4, gamma 0.999, GAE 0.95,
clip 0.2, entropy 0.003, two 128-unit layers and initial log standard deviation -0.7.
Do not add observation/reward normalization or change rewards in this comparison.

Use fresh optimizer seeds 1101, 1102 and 1103 for every arm. Pilots use 1099,
start from scratch and are excluded from design selection. At each 1,048,576
transitions, save a create-only checkpoint and evaluate the same development
schedule deterministically. Paired seed order is declared before launch; alternate
which arm runs first. A continuation must preserve the original allocation state.

Each direct-action arm first receives 4,194,304 transitions per seed. Extend
**both arms/all seeds** to 8,388,608 only if either arm has a native summary H
success, or a curriculum seed has at least two C0 diagnostic successes. If neither
occurs, stop that pair at the common halfway point and retain the negative result.
If wall timing cannot fit the extension, keep the common completed point rather
than comparing a full-budget arm against an unfinished one.

Pilot diagnostics log reward components and discounted returns, commanded and
realized throttle/attitude, clipping/saturation, speed, target separation in both
axes, footprint/attitude contact limits, fuel and native stop distributions. Check
that touchdown rewards exceed crash/hover alternatives under the declared
objective, including terminal discount and potential cancellation. Objective bugs
must be repaired/versioned before the main allocation; changing the intended
objective would be a separate experiment and is outside this night's revisions.
Audit the existing 24-point local and route terrain observations against the
sealed geometry: sparse queries may miss narrow hazards. Record that limitation
without changing the feature encoder during the sampler comparison.

One conditional additional representation is allowed if neither direct-action arm
has any H success in its completed development checks, and at least two thirds
of C0 attempts fail across the three curriculum seeds' latest common-endpoint
checks. Confirm representative failures from recorded commands/physical state
before activating it; zero success alone is not proof
that the control interface is at fault. Declare the decision and measurements.

The alternate policy outputs normalized desired horizontal/vertical velocities,
mapped to ±60 / ±40 m/s. Its independent adapter uses current measured velocity,
fixed 2 s horizontal and 3 s vertical tracking time constants, gravity compensation,
current mass and thrust limits to allocate throttle/attitude. Clamp the requested
upward thrust acceleration to nonnegative, attitude to ±0.8 rad and throttle to
[0,1]; native actuation and minimum throttle remain authoritative. Recompute at
each native 60 Hz boundary while holding the policy setpoint for six boundaries.
Keep the same launch primitive, features, reward, PPO and input banks. The adapter
uses no terrain avoidance, target-arrival gates or incumbent code; declare the
analytic mapping and saturation behavior before its pilot/checks.

Run both uniform and curriculum arms with this representation, three fresh seeds,
at most 4,194,304 transitions each. Its initial 262,144-transition signal checks
are part of those allocations, not extra pilots outside the transition budget.
It is one additional independent approach, including its deterministic adapter,
rather than evidence of purely learned
low-level thrust control. Compare representations only at their common 4,194,304
point. Curriculum effects compare matching representations at matching counts.
A longer direct-action allocation cannot be attributed solely to representation.
No second adapter, algorithm switch, demonstration policy or reward redesign is
available inside this contract.

## Selection, validation and final-test access

Checkpoint ordering within a seed uses layered H native summary landing rate,
then the value-backend rate as a guard, then earlier checkpoint. Layered is the
primary rate; no pooled 50:50 qualification score. Distance, hover proximity and reward
curves diagnose failures but never choose a purported landing policy.

If the optional representation ran, the selection horizon is the common
4,194,304 point. Otherwise use the primary pair's largest common completed point,
4,194,304 or 8,388,608. Select checkpoints only at or before that horizon.

At that point, rank designs by median three-seed layered H rate,
then worst-seed layered rate, then median value rate, then fixed design order
direct-uniform, direct-curriculum, velocity-uniform, velocity-curriculum. Keep all
seed results and full schedules. Compare the equal-budget point for experiment
conclusions; any later direct-action checkpoint is development-only extended
evidence when the optional representation ran. Choose the design before validation,
then freeze each of its three development-selected checkpoints and all required
source/native/SDK/state bytes.

Full validation is eligible only if at least two seeds of that frozen design have
an H development success and at least one H landing per such seed has complete
native capture/integrity/replay proof. These selected proof examples establish
individual landings, not a panel rate. Zero-progress experiments finish with
development diagnosis; they do not spend thousands of validation flights merely
to repeat the earlier negative confirmation.

If eligible and the complete campaign fits the reserved wall/disk allowance:

- Evaluate **all three seeds** on all 424 layered and 420 value validation inputs:
  2,532 scheduled RL attempts. No best-seed-only score or validation re-ranking.
- Run the unchanged reflected cap-24 incumbent once on the same 844 inputs as
  a contemporaneous reference. Its earlier 3,311-input rate stays separately labeled.
- Evaluate the frozen RL seeds on the separate 50-input layered stress union:
  150 attempts, sparse panel memberships and separate denominator.
- Reserve at most 128 other measured captures for integration/diagnostic/progress
  checks. Thus the declared complete path uses at most 3,654 capture attempts,
  within the 4,096 ceiling. No unbudgeted retries or repeated replay campaigns.

Summaries have no saved-command proof. Count verified landings only with physical
target contact, mission success, valid execution, stable integrity, complete native
command replay and a mission-terminal stop. Preserve unsupported, invalid-input,
initialization, execution, capture and finite/horizon failures in their schedules.
Fresh checkpoint-free verification precedes accepted campaign projection.

The 1,669 layered and 1,668 value **final-test inputs remain unopened**. No training,
preview, individual geometry inspection, checkpoint selection or flight accesses
them. This night provides development/validation evidence, not final qualification,
unseen-backend generalization or complete-terrain coverage. Stress holes and the
existing physical-resolution warnings remain explicit limitations.

## Validation, commits and morning review

Before optimization, run relevant Python environment/objective/sampler checks,
native command parity and the maintained development gate. Add meaningful checks
for bank identity/tamper handling, curriculum advancement, sampler/RNG continuation,
complete failure accounting and imported-source/frozen-checkpoint binding. Inspect
actual random-action and fixed-command traces; passing an API check alone does
not demonstrate discovery or control. Preserve original installed environments.
Existing terminal/simple-transfer fixtures can diagnose physical control in
preflight checks; their outcomes stay separate from procedural performance.

Routine implementation decisions and reversible fixes within this declaration
proceed autonomously and are recorded. A substantive new hypothesis, additional
design, changed benchmark or final-test release ends this allocation's expansion;
preserve the finding for the next daytime decision rather than improvising it.

Commit validated slices: experiment contracts/banks tooling; V2 trainer/sampler;
pilot and fixed allocation; paired results; optional adapter and checks; selection
freeze/evaluation; final evidence/report documentation. Keep generated banks,
optimizer bytes and captures under fresh ignored `outputs/rl-overnight/<run-id>`
paths, with compact tracked indices and hashes. Never modify bound experiment
source/native bytes during an allocation. Do not push or merge unrelated branches.

Produce a concise experiment overview linked to maintained rich campaign/detail
reports. Put achieved landings, the three seed rates, native failure distributions
and the stage reached first. Show learning curves and endpoint height/speed/offset
plots together so a hovering near miss is visible. State training transitions,
unique/reused inputs, validation exposure and proof counts alongside outcomes.
Extend the common presentation; retain mission-first hierarchy, backend branches,
full/focused modes, one expanded band, twelve previews per page and rich plots.
Use terrain-framed previews with explicit off-frame flight disclosure.

Build new local report editions, preserving prior pages and accepted selectors.
Browser-check the bounded tree and a rich detail using disposable QA processes;
do not restart the user's report server. Supply verified local paths and a served
link only if the existing server actually exposes the new artifact. A skipped or
partial campaign needs an equally clear diagnosis/report, not invented rates.

The result record must answer: what learned to land, on which population; whether
curriculum/control representation helped at equal budgets; which failures remain;
what integrity checks passed; and the single best next experiment. Complete a
final diff/source/evidence review and commit the record before the deadline.

## Research basis and limits

Procedural diversity and disjoint evaluation matter for generalization; the
[Procgen paper](https://proceedings.mlr.press/v119/cobbe20a.html) supports that
principle, without proving this terrain distribution complete. The
[SB3 practical guidance](https://stable-baselines3.readthedocs.io/en/v2.9.0/guide/rl_tips.html)
supports multiple seeds, normalized continuous actions and simpler discovery
tasks; curriculum thresholds and bank sizes above are our experiment choices.
[Potential-based shaping](https://ai.stanford.edu/~ang/papers/shaping-icml99.pdf)
motivates keeping gamma/terminal accounting consistent. The previous native
tests/results establish our v3 implementation's cancellation identity; the theory
does not promise successful PPO exploration or make resource penalties equivalent
to an unpenalized task.

The old pilot and first allocation measured seconds/minutes rather than hours,
so a large transition ceiling may be feasible locally. Different V2 geometry and
episode lengths can change throughput substantially. Only this night's timing,
resource and signal pilot chooses what actually fits; prior speed is not a launch
promise. These sources support experimental discipline, not a predicted landing
rate or the particular velocity adapter.
