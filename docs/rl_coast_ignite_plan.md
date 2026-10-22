# RL coast and landing ignition plan

[Documentation home](README.md) · [Poly Landing evidence](poly_landing_execution.md) · [Historical RL source](../studies/rl_procedural_ppo/README.md) · [Planner and controller boundary](maneuver_execution_design.md)

October 22, 2026. **Research and implementation proposal.** The next experiment
learns when to end a supplied ballistic coast and start the existing Poly
Landing burn. General arrival control, departure and terrain avoidance are not
prerequisites. This proposal defines the task and its staged acceptance; it does
not launch implementation, training, native campaigns or report operations.

The owner subsequently launched the [execution loop](rl_coast_ignite_execution.md).
That record owns actual epochs, budgets and results; this document retains the
prospective design and selection rules.

The subsequent [stopping policy research and follow-up plan](rl_coast_ignite_stopping_plan.md)
examines the eleven retained misses and proposes a comparison using frozen
weights before allocating further learning.

The hypothesis is that an outcome-trained policy can choose useful ignition
times across different energies and starting phases while a fixed controller
performs every physical command. A complete episode includes coast, admission,
the actual burn and native contact or bounded failure. It begins airborne and
does not claim source-pad departure or general mission-planning capability.

## Research findings and design choices

The task is a finite-horizon **optimal-stopping problem**: at each opportunity,
continue coasting or commit to a landing attempt. The stopping payoff is the
actual result of executing the fixed controller. This is our application of
the formulation in [Becker, Cheridito and Jentzen, 2019](https://jmlr.org/papers/v20/18-232.html),
which studies learned stopping rules rather than rocket control. Their backward
training method and financial benchmarks are not a recipe replicated here.

The burn is a temporally extended action. The
[options framework of Sutton, Precup and Singh, 1999](https://people.cs.umass.edu/~barto/courses/cs687/Sutton-Precup-Singh-AIJ99.pdf)
supports a fixed execution policy with initiation and termination semantics and
variable duration. We need one such committed operation, not option discovery
or a general hierarchy. Count its actual native duration and work even when a
single learning-environment call executes the whole suffix.

[Pardo et al., 2018](https://proceedings.mlr.press/v80/pardo18a.html) distinguish
task deadlines from artificial rollout cutoffs. The observation therefore
includes remaining timing constraints. Native contact and the declared finite
task endpoint terminate learning without bootstrap; a learner buffer boundary
does not terminate or reset the physical episode. This also follows
[Gymnasium's termination guidance](https://gymnasium.farama.org/tutorials/gymnasium_basics/handling_time_limits/).

Use one discrete PPO implementation first. The
[PPO paper](https://arxiv.org/abs/1707.06347) supplies the clipped policy-update
method; [Stable Baselines3 2.9.0](https://stable-baselines3.readthedocs.io/en/v2.9.0/modules/ppo.html)
supports categorical actions. Neither establishes performance on this task.
An algorithm sweep, DQN comparison, imitation arm and learned duration selector
can wait for an actual failure mechanism or a separate research question.

[Agarwal et al., 2021](https://proceedings.neurips.cc/paper/2021/hash/f514cec81cb148559cf475e7426eed5e-Abstract.html)
motivates reporting uncertainty and variation across training runs. Use three
independent training seeds, report each validation trajectory, and freeze
selection before a shared fresh evaluation. Hundreds of evaluated missions do
not turn three training seeds into hundreds of independent learning runs.

Retained local review notes in `rl_lessons.md` also matter: L04 records
discount-related timing distortions; L05 requires declared assistance; L10
requires an engine-off control for supplied arcs. These motivate the explicit
objective, starting-state construction and baselines below. That reconciliation
is separately owned pending work; this plan does not require its files to build
or run the new approach.

## Mission population

Keep the accepted vehicle, Earth gravity, 120 Hz physics, 60 Hz commands, flat
ground, a 36 m pad and a 40 s original deadline. Use checked backend
`polynomial_landing_v2` without changing its reference or throttle arithmetic.
Each mission supplies a fixed absolute reference-end tick. RL chooses ignition
time only; no duration search runs inside the controller or policy adapter.

Here, a **perfect arc** means the engine-off centre-of-mass trajectory reaches
pad centre at the upright touchdown-centre height, 5 m above the plane, under
the native discrete gravity equations. It does not mean a passive safe landing:
incoming velocity and actual body contact still matter. Run the never-ignite
baseline through native contact rather than assuming its outcome.

Construct the arc around a nominal landing-entry witness using input-only math.
Let `dt = 1/120`, positive gravity magnitude `g = 9.81`, even burn length `N`,
nominal coast length `C`, and `B` ticks from the witness to the engine-off
5 m plane crossing. With target x zero, forward speed `vx` and downward witness
speed `s`, use:

```text
witness velocity = (vx, -s)
witness x        = -vx * B * dt
witness y        = 5 + s * B * dt + g * dt^2 * B * (B + 1) / 2

initial vy       = -s + g * C * dt
initial x        = -vx * (C + B) * dt
initial y        = 5 - initial_vy * (C + B) * dt
                     + g * dt^2 * (C + B) * (C + B + 1) / 2
reference end    = C + N
command coverage = C + N + 4
hard deadline    = 4800
```

These expressions preserve the native semi-implicit position update. At tick C
the coast reaches the nominal entry; at C+B its continued engine-off arc reaches
the declared geometric crossing. Native execution must independently establish
whether the proposed witness actually lands.

Proposed raw axes are independent hash streams, then coupled by these equations:

| Axis | Bounds |
| --- | --- |
| Nominal burn duration N/120 | 8–24 s, rounded to complete two-tick pairs |
| Witness forward speed divided by burn duration | 0.5–3 m/s² |
| Witness downward speed s | 8–55 m/s |
| Ballistic crossing fraction B/N | 0.28–0.42, B quantized to a physics tick |
| Fuel | 1,800–6,300 kg |
| Nominal coast C/120 | 0.5–6 s, quantized to the 10 Hz policy grid |
| Direction | Equal-probability physical reflection |

Retain every proposal and input-only exclusion. Enforce the crossing fraction
after rounding. Derive the previous vertical mixture
`q = (((witness_y - 4.995) / (N/120)) - 1.5) / (s - 1.5)` and require 0.4–0.6.
Then apply the existing pure reference descent, authority, fuel and servo
predicate, plus positive body clearance along the coast. This keeps nominal
witness parameters within the earlier landing recipe's bounds, while defining
a distinct correlated population that needs its own native validation.

Set initial angular rate to zero and initial attitude to the nominal witness's
first paired burn direction. The coast controller holds that supplied attitude.
This is explicit physical preparation supplied by the mission constructor,
identical for every compared policy. It does not establish learned arc creation
or unassisted attitude acquisition. Rotation during a mistimed ignition remains
part of the actual controller outcome.

Permit at most 16 raw proposals per requested admitted arc. Underfill is a
retained preparation failure. Do not filter fresh arcs by a native timing sweep,
native witness success or a trained policy's outcome. Keep witness C/N/B and
input-audit data in a separate provenance record; they are absent from policy
observations, rewards, action masks, learner weight initialization and training
labels. The witness-dependent physical initial pose remains declared starting-
state assistance, available equally to every compared policy.

## Planner and controller contract

The policy acts at ticks 0, 12, ..., 960: 81 opportunities from 0 through 8 s.
The last `WAIT` commits to never ignite. Earlier native contact closes the
episode before another decision. This declared ignition window covers every
nominal C and bounds the first task without adding an automatic takeover.

| Decision | Native execution |
| --- | --- |
| WAIT | The coast controller emits engine-off commands holding the supplied initial attitude; advance up to 12 physics ticks, stopping at the actual native endpoint |
| IGNITE | At the same command tick, submit one request using actual state, target and fixed reference-end tick; execute Poly Landing without further policy calls |
| NoPlan | Record one rejected attempt, retain engine-off controller ownership and run the same episode to contact or declared coverage; no retry or duration substitution |
| End of ignition window without a request | Coast to actual contact or declared coverage; no forced burn |

The controller owns all throttle and attitude commands, including coast. The
harness owns scheduling and the single live Episode. Initialization of a burn
backend at the ignition tick never resets plant state, fuel or clocks. Preserve
the original four-tick residual allowance and hard deadline separately.
At coverage exhaustion, a still-flying native state remains Flying/InProgress;
the learning task ends with reward zero and `terminated=True`. Do not relabel
the native result as a crash or bootstrap beyond that declared task endpoint.
The 10 Hz policy grid is local to this experiment; it does not round existing
60 Hz controller handoffs or change historical preservation semantics.

Use ten physical/context features with fixed scales: target-relative x/1500 m,
height above surface/1000 m, vx/100 m/s, vy/100 m/s, sin/cos attitude, angular
rate divided by pi/2, fuel/6300 kg, time to reference end/40 s, and remaining
ignition slots/81. The integer slot count comes from the native clock. Physical
state and the public timing constraint are available to every baseline too.
No scenario ID, generator seed, nominal witness time, feasibility verdict,
future rollout or timing-map entry becomes a feature. Use no running observation
or reward normalization in the first version.

## Reward, exploration and learner

Use terminal reward 1 for native target landing plus mission success, otherwise
0. Intermediate reward is zero. Training Summary mode records provisional
native task outcomes; replay-certified landing claims belong to captured
evaluation. Software faults stop the run and remain faults, rather than being
converted to ordinary zero-reward examples. Fuel, timing and contact margins
are reported separately, without shaping or efficiency reward in this pass.

Use `gamma=1`, `gae_lambda=1` and zero entropy coefficient initially. For the
finite task this removes discount preference between different successful
ignition times. At a true task endpoint the value bootstrap is zero; a PPO
buffer boundary retains its live episode and bootstraps normally. The pinned
[SB3 return implementation](https://raw.githubusercontent.com/DLR-RM/stable-baselines3/v2.9.0/stable_baselines3/common/buffers.py)
supports this lambda-return configuration. Variable physical burn duration is
still recorded; a future discounted or time-reward task needs explicit duration
semantics rather than treating a whole burn as one equal-duration physics step.

A naive 50/50 categorical initialization waits four seconds at 10 Hz with
probability only `2^-40`, approximately `9.1e-13`. Initialize exploration over
stopping times instead. At slot k, from 0 through 80, use:

```text
prior ignite probability = 1 / (82 - k)
ignite logit             = logit(prior probability) + learned residual(state)
wait logit               = 0
```

Initialize the residual actor head to zero. Before early-contact censoring,
each of 81 ignition slots and the never-ignite outcome has probability 1/82.
This prior uses the declared clock only. It supplies neither a successful time
nor an expert action. The same frozen prior is a development baseline. There
is no forced final ignition. Verify actual sampled stop times and native reward
access; the algebra alone is not a discovery-readiness result.

Proposed first recipe: SB3 PPO with separate 64-by-64 tanh actor/value networks,
CPU execution, four sequentially stepped vector environments, 512 decisions per
environment per rollout, batch size 128, four optimizer epochs, learning rate
3e-4, clip range 0.2, value coefficient 0.5, max gradient norm 0.5 and target KL
0.02. These are engineering starting values, not a literature-backed optimum.
Record actual optimizer steps and early KL stops. Inspect initialization and
the first update before releasing the remaining budget; numerical finiteness
alone does not establish useful learning or retention.

Train by categorical sampling. Primary evaluation uses the frozen deterministic
decision: ignite iff its probability is strictly greater than 0.5; ties wait.
The untrained deterministic prior therefore waits throughout. Do not mistake
that result for its sampled exploration coverage, or tune the evaluation
threshold on held-out outcomes.

## Baselines, selection and fresh evaluation

Use disjoint physical arc streams for development, training, validation and
confirmation. Training seeds have independent weights, sampler, action and
minibatch streams. Keep a shared 64-arc validation set fixed across checkpoints;
repeated evaluations of it remain exposed validation, not independent trials.
Check physical identity overlap as well as complete task identity, which also
binds reference-end time, coast policy and decision window.

Development compares immediate ignition, never ignite, the uniform stopping
prior, fixed delays `0, 0.5, ..., 8` s, and a state-dependent time-to-ground
threshold with thresholds `0.25, 0.5, 1, 1.5, 2, 3, 4, 6` s. The latter uses
current lowest body clearance h and vertical velocity vy with the simple
estimate `(vy + sqrt(vy^2 + 2*g*h)) / g`, never a controller forecast. This is a
causal heuristic, not an exact native contact or landing predictor. Baseline
selection uses validation success counts only;
ties prefer immediate, then the earlier fixed delay, then the smaller threshold.

For twelve development arcs, evaluate every 10 Hz ignition opportunity and
never ignite: at most 984 matched variants. Native pre-ignition contact may
censor a requested time. Report successful-window widths, nominal witness
results, early/late failures, NoPlan and passive outcomes. The per-arc hindsight
union is a diagnostic upper bound, not an executable baseline or training label.

Use three independent PPO seeds with a proposed ceiling of 131,072 policy
decisions each. Evaluate checkpoints at 0, 32,768, 65,536, 98,304 and 131,072.
For each seed select the highest validation native-success count; ties prefer
the earlier checkpoint. Select one primary package across seeds by the same
count, then earlier checkpoint and declared seed order. Report every seed and
learning curve. This is a selected-package experiment, not qualification of all
seeds or a comparison of algorithm families.

Validation sweeps may use bounded Summary mode to control storage; their scores
are explicitly unverified selection indicators. Before fresh release, capture
and verify each seed's selected checkpoint and the selected heuristic on all
64 validation arcs. Counts must agree with selection and all proofs must pass.
Do not select a different package because it happens to survive a verification
defect; fix the software boundary and declare the affected epoch honestly.

After source/package/baseline freeze, generate exactly 600 fresh arcs once.
Evaluate four frozen packages on each: selected RL, selected non-oracle
heuristic, immediate ignition and never ignite. Each package executes its own
complete native episode from the same original input. Unstarted/faulted slots
remain in the 2,400-slot schedule. Fresh witness timings are not flown as an
additional oracle panel, and no policy or baseline is reselected afterward.
Every pair in the primary comparison needs complete verified outcomes. A fault
or incomplete proof blocks that comparison; retain completion accounting and
do not silently shrink its denominator or convert a software fault to a miss.

Report replay-certified success rates and exact binomial intervals. For the
single primary comparison with the validation-selected heuristic, report paired
RL-only wins w, heuristic-only wins l, ties and absolute lift `(w-l)/600`.
A prospectively declared one-sided exact test uses
`P[Binomial(w+l, 0.5) >= w]`, with p=1 if there are no discordant pairs.
Positive evidence of decision advantage requires positive lift and p<=0.05.
The immediate/never comparisons are supporting diagnostics. Do not carry the
previous controller-only 99% qualification criterion into this different task.
A working hybrid that ties a strong heuristic is useful integration evidence,
with no demonstrated RL advantage. There is no automatic extension after a
negative result and no fresh-data controller tuning.

## Source review and implementation sequence

Current baseline is main checkpoint `b0bfb65`; the measured burn implementation
is `7b8f277` and its frozen source epoch. Relevant source boundaries are:

| Source | Reuse or required addition |
| --- | --- |
| [Shared reference](../pd-control/src/poly_landing/reference.rs) and [paired inverse](../pd-control/src/poly_landing/actuation.rs) | Reuse unchanged |
| [Checked landing controller](../pd-control/src/maneuver/poly_landing.rs) | Reuse actual-observation admission, one immutable reference and native-end handling |
| [Timed composition](../examples/maneuver-policy/src/poly.rs) | Existing fixed activation and replayed prefix prove regression behavior; they do not execute a learned planner |
| [Timed preparation/evaluation](../examples/maneuver-policy/src/evaluation/timed.rs) | Reuse bounded input identities, result accounting and rich report projection where compatible |
| [Neutral Python binding](../pd-python/src/lib.rs) | Exposes physical commands today; preserve its independence rather than importing Poly Landing into it |
| [Retired experiment](../experiments/rl/README.md) | Historical reference only; its namespace and archived sources remain retired |

Implement a small coast/landing mode controller in `pd-control` and a new
composition/binding under proposed `examples/coast-ignite-policy`. Put the new
Python approach under proposed `experiments/coast-ignite`, with its own manifest,
lock and environment. Do not add a general maneuver-goal system or modify the
accepted Policy 3 planner for this experiment.

The retained environment's metadata currently lists SB3 2.9.0, Torch 2.14.1+cpu,
Gymnasium 1.4.0, NumPy 2.5.3 and maturin 1.15.0. These are candidate compatibility
references, not an installed-runtime acceptance result. A new environment needs
its own exact lock, imported-version/native-artifact checks and finite-update
smoke; the retained environment remains untouched.

Use one native stepper with Summary and Captured modes, and prove they emit the
same decisions, commands and native endpoints. Training executes real controller
suffixes; it never looks up precomputed stopping payoffs. Captured evaluation
needs three distinct checks: neutral physical replay; controller reconstruction
from recorded wait/ignite requests; and planner reconstruction by rerunning the
frozen model, feature code and deterministic decision rule on reconstructed live
observations. Reconstructing a saved ignition tick with the old timed-program
verifier alone is not learned-policy verification. Bind weights, runtime/source
identity, feature scales, hazard prior and controller configuration.

| Stage | Deliverable and condition to advance |
| --- | --- |
| A Software contract | Pure generator/hazard/return tests; native summary/capture parity; off-boundary contact; same-tick ignition; NoPlan/never/expiry closure; model save/load and both reconstruction paths |
| B Cost and timing preflight | A create-only 64-episode cost pilot (eight development arcs under eight declared schedules), followed by the twelve-arc timing map. Verify nominal witnesses and measure useful timing headroom and decision-grid resolution before training |
| C Learning readiness | 256 sampled untrained episodes; require positive outcomes on at least two distinct arcs, then inspect one complete PPO update. Zero reward access, policy collapse or software drift blocks the remaining allocation |
| D Bounded learning | Three fixed seed budgets and fixed checkpoint schedule; no outcome-driven algorithm or reward sweep |
| E Frozen evaluation and report | Verify selected validation packages, release one 600-arc confirmation, then the established mission tree and complete rich details |

Cost-pilot aliases and timing-map variants retain their shared arc identity;
64 episodes are not 64 independent missions. If fixed timing already captures
the available success windows, record the limited decision headroom rather than
claiming an RL advantage or discarding easy inputs. If the 10 Hz grid misses a
native opportunity, diagnose decision resolution before changing the task.

All counts above are proposed ceilings. A future execution declaration must
freeze streams and the cost pilot's wall/byte/step limits before that pilot.
Use its measurements to freeze each later phase's limits before starting it.
Proposed outer limits are one native worker, four hours total,
30 minutes and 50 million native steps per training seed, 32 GiB new artifacts
and 50 GiB free space. Four vector environments execute sequentially in that
worker. Count policy decisions, original episodes, all native burn/coast steps,
pure admission work, optimizer steps and proof passes separately. A long IGNITE
call must be interruptible at native boundaries, not evade worker/time caps.
Finish in-flight accounting without converting resource exhaustion into a task
failure or silently allocating another episode. Release final capture/report
budgets before opening fresh inputs; do not discover storage cost at delivery.

## Review and validation of this proposal

The coast and hazard equations were checked with scalar arithmetic only:
1,000 reverse/forward gravity round trips had maximum error `9.10e-13`; all 82
uncensored stopping outcomes had probability 1/82. An exploratory 10,000-draw
structural calculation retained 3,708 proposals before actuation checks, spanning
1,088 ascending, 307 near-apex and 2,313 descending starts. Its maximum geometric
crossing error was `1.14e-13` m. These calculations used no native transition,
learner, retained mission output or frozen fresh stream, and establish neither
native feasibility nor learnability.

Review must specifically catch a hidden witness feature, a forced successful
takeover, changed burn arithmetic, default 50/50 early ignition, a discounted
terminal objective, task deadlines mislabelled as truncation, post-commit policy
samples, Summary outcomes presented as verified landings, pooled training-seed
claims and a saved ignition schedule presented as model reconstruction.

The initial scope stays supplied target-directed arcs, coast timing and one
fixed landing executor. Generalized control, departure, terrain shaping,
mid-burn recovery, learned touchdown duration and fuel optimization remain
separate extensions. Findings from this experiment should decide which of
those is worth pursuing next.
