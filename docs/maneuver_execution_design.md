# Mission planner and maneuver controller design

[Documentation home](README.md) · [Research and validation review](planner_controller_design_review.md) · [Independent agents](independent_agents.md)

October 21, 2026. **Reviewed design proposal.** Use two flight-decision
interfaces: a mission planner requests maneuvers, and a maneuver controller
executes them. Request admission, execution lifecycle and backend state belong
inside the maneuver controller. The harness schedules them against the native
plant, and the evaluator assesses evidence independently. Start with explicit
activation of terminal landing on the mission's target. Characterize execution
and deterministic decisions before selecting a learning algorithm or budget.
The production interfaces, adapter and harness additions remain unimplemented
and unqualified; current direct-control experiment allocations remain closed.

The eventual objective is safe, efficient complete procedural missions. Landing
activation is the first bounded capability and decision test. Departure,
airborne correction and intermediate arrival goals require supported controllers
before a composed approach can claim complete mission capability.

## Planner controller and supporting components

| Component | Owns | Exchanges |
| --- | --- | --- |
| Mission planner | Intent, activation timing and, for future capabilities, goal selection, retention and revision | Physical observations and declared controller feedback in; keep/request decisions out |
| Maneuver controller | Request validation, command authority, commitment, lifecycle, backend state and command generation | Requests and actual observations in; native commands, acknowledgements and progress out |
| Harness | Scheduling, the one live episode, input roles, component identity, accounting and trace collection | Drives the planner and controller against the authoritative native plant |
| Evaluator | Physical and mission assessment, integrity, command replay and comparison accounting | Consumes saved execution and declared evidence; never supplies flight decisions |

Manual input, deterministic rules and learned policies provide decisions through
the same planner-facing request semantics. Operator describes the decision
source; executor describes the controller's lifecycle machinery. Neither needs
a separate flight-decision interface. Both planner and controller can react to
actual state; planner does not require a complete static flight plan.

The maneuver controller can contain a lifecycle facade and a replaceable guidance
backend. The current terminal controller can serve as that backend through an
adapter, retaining its candidate selection and plan state. A controller change
needs its own measured envelope; interface compatibility alone does not establish
equivalent capability. The existing `pd_control::Controller` and `pd_run::Agent`
traits remain compatible. The native attitude servo stays in plant actuation.

## Existing controller and ZEM ZEV guidance

The [state-target helper](../pd-control/src/guidance.rs) computes zero-effort
position miss and velocity error under constant gravity, then requests:

```text
requested_acceleration = 6 * ZEM / T^2 - 2 * ZEV / T
```

The [terminal controller](../pd-control/src/terminal/mod.rs) chooses the horizon
and desired terminal velocity, evaluates finite candidates, applies thrust/tilt
and predicted terrain constraints, and handles braking, settling and touchdown.
The helper is mathematical support for that implementation, not an arbitrary
state-arrival controller. The [allocator](../pd-control/src/guidance.rs) maps
acceleration to native engine-setting and attitude commands.

The planner-facing contract must not require a ZEM/ZEV horizon, error term or
gain. Those remain backend details. Another feedback law or an optimization-based
controller could later implement the same landing capability. Optional backend
diagnostics must be named and declared separately from common physical inputs.

Use the complete [resolved terminal configuration](../pd-control/src/terminal/config.rs),
including coupled opt-in flags, when identifying the backend. External clamps,
command transformations, cadence and coast behavior are also part of the
effective execution configuration.

## Proposed interface contract

The names below describe semantics; Rust traits and persisted schema names are
to be settled during implementation.

| Operation | Contract |
| --- | --- |
| Describe | Return interface version, implementation/configuration identity, supported goal kinds, operating assumptions and declared diagnostics |
| Submit request | Validate the request at a named native command tick; return accepted, already active, busy, unsupported or invalid, with a reason |
| Advance controller | From the actual observation and accepted request, produce one native command and lifecycle feedback |
| Observe episode end | Close execution from the native terminal/stop result, preserving physical, mission and execution dispositions separately |
| Assess optionally | Return tick-bound advisory estimates without advancing active-controller state or taking command authority |

A capability declaration describes supported semantics and environment. It is
separate from empirical coverage and from a prediction of success for the
current state. An accepted request means the controller will attempt the supported
maneuver. It does not promise a landing.

Requests need an identity, goal kind, goal parameters and submission tick.
Feedback needs the request identity, acknowledgement/disposition, active state,
reason and relevant native tick. Repeated submission of the active landing
request is idempotent: it must not reset controller state or replenish time.
Malformed API calls and unsupported goals remain distinguishable from an
ordinary finite request that produces a poor flight.

Future arrival goals may specify position regions, velocity ranges, an absolute
arrival window, pose/rate requirements and completion semantics. Pass-through,
capture/hold and native touchdown are different goals. Only goal kinds actually
implemented and characterized belong in the supported capability list. Future
coordinates must identify the reference point and frame: vehicle centre versus
feet, world versus relative coordinates, and when a relative goal is resolved.
Refreshing guidance must not move a fixed accepted goal.

### Initial request and feedback vocabulary

The first interface has these conceptual values:

```text
PlannerDecision = Keep | Request { request_id, goal }
ManeuverGoal = LandOnMissionTarget { target_pad_id }
Admission = Accepted | AlreadyActive | Busy | Unsupported | InvalidRequest
ControllerState = Coasting | LandingActive | Closed
```

Request IDs are scoped to one episode. The accepted ID/payload pair is immutable;
rejected submissions remain in the trace without creating a backend instance.
The harness stamps each decision with its actual native tick. `Keep` preserves
the current behavior: upright coast before activation and continued landing
afterward. It never cancels or resets a maneuver. A successful admission begins
backend updates at the same command boundary. Accepted goals are immutable.

| State and event | Result |
| --- | --- |
| Coasting and supported landing request | Accept, initialize backend state once, enter LandingActive |
| LandingActive and identical active ID/payload | AlreadyActive; preserve all backend state |
| LandingActive and a new request ID | Busy; continue the existing landing |
| Active ID reused with changed payload | InvalidRequest; preserve the accepted goal |
| Well-formed unsupported goal or cancellation | Unsupported; preserve the current command owner |
| Stale request tick | InvalidRequest; do not apply it to a later state |
| Actual native end or bounded stop | Close once with the native physical, mission and execution result |
| Closed | No further planner or command callbacks |

The initial scope is an airborne, route-free `LandingOnPad` mission in the
declared environment, using its existing target. Ground launch and authored
waypoint/checkpoint goals are outside that capability. These are semantic/input
restrictions, not an outcome-derived filter for starts on which the backend wins.
Finite low-fuel, late and difficult entries remain explicit characterization
cases, with the original physical rules and deadline.

Feedback separates admission, active request/state and the native result. It
includes `decision_available` so a harness can distinguish a planner choice
from compulsory continuation. The initial planner receives physical observations,
mission geometry and common lifecycle feedback. Backend-specific advice stays
out of this initial policy input; optional advice is a named, costed variant.

Malformed decision data and invalid numerical native commands remain execution
errors. Ordinary Busy/Unsupported/admission rejection is a recorded decision
disposition and must not masquerade as a runtime failure. A terminal or failed
request does not erase the episode's actual outcome.

## First landing controller

The initial planner choices are continue the declared coast behavior or
activate landing on the mission's fixed target. The native
[run context and observations](../pd-core/src/model.rs) already bind target
geometry to that mission; arbitrary pad retargeting is a future capability.

| Aspect | Initial proposal |
| --- | --- |
| Environment | Tested vehicle and Earth gravity, 120 Hz physics and 60 Hz native command/control updates |
| Entry | Actual running airborne episode and the mission's landing target; the measured entry envelope remains to be established |
| Before activation | Engine off and upright attitude target, explicitly recorded as deterministic assistance |
| Activation | Accept at the current command boundary and begin backend updates from that observation; do not wait for an advisory entry gate |
| Active behavior | Backend owns throttle, attitude, candidate/horizon selection, braking, lateral cleanup and touchdown handling, including engine-off commands |
| Commitment | One landing attempt through episode termination; cancellation, retargeting and preemption remain unsupported initially |
| End | Native outcome or bounded stop, followed by independent evidence assessment |

The [plant](../pd-core/src/sim.rs) applies attitude commands even with the engine
off. Upright coast therefore changes pose through the existing native servo;
it is not uncontrolled rotation. All planners in a comparison use the same
coast behavior, and any later retrograde or planner-controlled coast is a
separately identified variant.

Create/reset backend instance state once for activation, preserving its exact
configuration. Read-only advice before activation must not warm its hysteresis
counters or guidance plan. Keep original plant state, fuel, mass and mission
clock; activation never creates a fresh scenario or deadline.

The ordinary terminal configuration is the initial adapter parity reference.
Keep configured predicted candidate clearance enabled. Disabling it changes
behavior and needs separate qualification. Route-level terrain navigation stays
above this capability, with real collision and contact rules authoritative.

The [entry assessment](../pd-control/src/terminal/state.rs) is advisory. Its
`LatestSafe` branch can select an urgent best-effort candidate, and its readiness
does not establish complete fuel, remaining-deadline or native landing
feasibility. The backend can generate commands while its mode is nominal
pending. A missing ready candidate must not become an undocumented automatic
takeover, an implementation error, or a claim of physical impossibility.

## Controller alternatives and comparison identity

The E12 persistence comparison provides an
existing candidate: optional arrival-time retention preserved all 171 benchmark
landings out of 189 inputs, and ordinary/retained modes each landed all 312
development inputs. Aggregate time/fuel distributions favored retention, with
the original paired exceptions retained in that record.

That comparison used an external 0.8-radian attitude-command clamp at both tested
controller cadences, 60 Hz and 10 Hz. The later E28 reference used its reviewed 10 Hz
wrapper with native 60 Hz command submission. Neither result transfers silently
to a different cadence, clamp, entry population or configuration. The original
receipts own the historical results; this design does not recertify them.

These are local workspace records, not artifacts bundled with this design.
In the sibling `powered-descent-lab-rl-v2` worktree, E12 is recorded in
`docs/rl_control_coherence_results.md` and
`outputs/rl-control-coherence/run-20261014T191314Z/completion-receipt.json`;
the E28 reference is implemented in
`studies/rl_airborne_audition/reference.rs`. Neither record qualifies the new
request interface or its proposed entry envelope.

Establish parity with the ordinary backend first. Then consider retained arrival
as one named candidate under a common characterization schedule. Freeze the
chosen controller/backend configuration before comparing planners.

Keep two comparison questions separate:

- A **planner comparison** fixes the controller/backend, coast policy, physical
  inputs, resources and observation/advice contract. It measures decision logic.
- A **controller comparison** fixes goal semantics and the planner algorithm or
  declared request program. Closed-loop planners receive each flight's actual
  feedback; a fixed recorded request schedule is a separate diagnostic. It
  measures execution behavior and changes in the operating envelope.

Later crossed planner/controller comparisons can measure dependence on a
particular backend. A policy trained with backend-specific advice has that
declared dependence. Exact command parity is a requirement for an unchanged
backend adapter, not a requirement for different control algorithms.

## Controller integration findings

The current source supports a small adapter, with the following boundaries.

| Existing behavior | Consequence for the first controller facade |
| --- | --- |
| `Controller::reset` resets controller state; `update` returns an ordinary command frame | Keep the trait compatible and add request/lifecycle semantics around it; activation must not reset the plant |
| Terminal update computes commands directly, including in nominal-pending mode | Accepting activation grants command authority immediately; advisory readiness does not defer it |
| Candidate selection reads current mass and motion but does not certify total fuel use or completion within remaining mission time | Preserve the original deadline and native fuel accounting; characterize low-fuel/short-time entries explicitly |
| Descent, flare and touchdown are telemetry phases | Determine completion from the native episode result, never from a phase label or small geometric error |
| Backend state includes readiness hysteresis and optional retained arrival time | Repeated requests and advice must not reset or advance that state; exact continuation needs more than the resolved constructor configuration |
| The generic Agent loop exits when its native episode stops and has no final-result callback | The approach harness must finalize the maneuver from the returned execution or direct Episode step result |

Backend outputs use the existing native `Command` contract. For available fuel,
engine setting `u = 0` is off; positive setting applies thrust fraction
`min_throttle + u * (1 - min_throttle)`. For example, `u = 0.5` with minimum
throttle 0.25 applies 0.625 of maximum thrust. Attitude is an absolute target
executed by the rate-limited native servo. A replacement controller needs these
vehicle/actuation semantics, and the controller facade must not silently reinterpret
legacy output as a physical thrust fraction. Requested acceleration, emitted
engine setting and realized motion should remain separately inspectable.

Physical state alone is not a general Markov-state guarantee for a planner above
a stateful controller. Retained horizons and hysteresis can affect future
commands. The initial irreversible landing task has no further planner choices
after activation. Before enabling goal revision or composition, decide which
controller state summaries, history or recurrent policy state make those choices
well specified; generic lifecycle feedback alone is not proven sufficient.

Remaining implementation decisions are the exact descriptor/trace encoding and
durable restart method. The default ordinary terminal configuration supplies the
first parity reference. The retained-arrival candidate must bind its complete
configuration and external clamp. Neither a feasibility filter nor controller
tuning is added to make an activation comparison succeed.

## Harness scheduling and state continuity

Use one ordinary [native episode](../pd-run/src/episode.rs). The harness owns
its lifetime and advances it only through admitted native commands. Planner and
controller code may not replace state, reset fuel or restart clocks.
Counterfactual activation trials reproduce the original mission prefix before
switching authority; they do not teleport into a freshly budgeted entry state.

For each live native command boundary:

1. Read the actual observation and previous controller feedback. An already-ended
   episode produces no further planner request or command.
2. If a planner decision is due and a choice is available, provide that observation
   and the declared feedback/advice, then record and acknowledge the decision.
3. Let the active controller/backend produce a command from the same observation,
   or produce the declared coast command before activation.
4. Record the source of command authority and advance the native episode to the
   next command boundary, stopping earlier on native contact or a bounded stop.
5. Feed the actual resulting outcome into the lifecycle and resource ledgers.
   On end, deliver exactly one final native result to the controller facade.

Start with 60 Hz controller/backend updates and a 10 Hz planner decision grid.
Before activation, a planner decision is due every six native command intervals.
After activation, the controller continues at 60 Hz and no further planner choice
is available in the initial committed-landing task. Holding the planner's request
must not hold a stale terminal command for those six intervals. Cadences, phase
alignment, first-decision timing and rounding belong in the run declaration.

Ordinary rejected requests still consume physical time: an existing active
backend continues updating, or the declared coast behavior runs before activation.
Repeated rejection cannot create zero-time retries.
Request rejection alone must not be sent through `Episode::decision_failed`,
which represents an execution-invalid native stop. Invalid numerical commands
and genuine implementation failures retain that native error path.

The approach harness must implement final-result delivery explicitly: the
current [Agent runner](../pd-run/src/agent.rs) stops calling `update` once the
episode ends. Either finalize the facade from its returned execution, or own the
incremental episode loop and use the actual step result. Ordinary evaluation
already permits composition through `assess_execution`; no new controller
dependency or strategy callback is required in neutral evaluation.

### Decision transitions and elapsed time

The preferred future learning binding records actual decision transitions.
`Keep` advances native execution to the next available planner opportunity or
native end. Accepted landing activation executes all subsequent native control
steps through episode termination and returns that actual duration. This is a
temporally extended decision, not a state jump. The trace distinguishes native
updates, decision opportunities and policy invocations; compulsory continuation
must not create sampled planner actions, actor-loss rows or exploration draws.

For a base-step discount `gamma`, aggregation over `k` base steps preserves
`R = sum(gamma^i * r_i)` and the nonterminal continuation multiplier `gamma^k`.
The time unit, reward timestamps and actual partial terminal interval must be
declared. With a non-unit discount, treating every maneuver as one equal-duration
step changes the objective. A finite mission deadline is an environment end;
an unfinished training rollout is a different truncation with different bootstrap
semantics. These accounting identities do not select the reward preferences.

A fixed-interval learning adapter is also possible, but must explicitly identify
and handle forced continuation. It cannot silently substitute ordinary sampled
actions after commitment. Settle its equivalent accounting before fitting or
comparing it with the preferred decision-transition binding.

Exact continuation must retain or reconstruct planner state/RNG, controller
lifecycle, backend hysteresis/plan state, held commands and cadence phase. The
current terminal backend is clonable within a process but has no public durable
state encoding. Reconstruction from an original command/request prefix needs
its own exactness and cost checks before it is used for training checkpoints.

## Evaluation framework and evidence

Reuse the existing platform rather than create another plant or landing grader.

| Existing foundation | Responsibility retained |
| --- | --- |
| [Agent and physical mission view](../pd-run/src/agent.rs) | Open approach integration without generator labels or collection membership at decision time |
| [Episode runtime](../pd-run/src/episode.rs) | Actual native stepping, summary/captured modes, original bounds and terminal handling |
| [Episode evaluation](../pd-eval/src/episode.rs) | Physical/mission outcomes, source integrity and authoritative bounded command replay |
| [Collection evaluation](../pd-eval/src/collection.rs) | Fixed schedules, input exposure, every attempt disposition and campaign integrity |
| [Capture sealing](../pd-eval/src/capture.rs) | Exact evaluation bytes and create-only destinations |
| [Common reporting](independent_agents.md#rich-reporting) | Rich saved-flight detail and campaign presentation |

The existing `CollectionExecutorV1` is a campaign composition callback. It is
not the internal maneuver execution role defined here. The neutral runtime/evaluator
must stay independent of concrete controllers and learned planners.

The new harness needs a versioned composition declaration binding planner,
controller facade and backend identities/configurations; source and binary/checkpoint
hashes; native and planner cadences; coast/advice/action mappings; mission/input
identities; seeds; schedule; and resource limits. Exposure roles stay in
orchestration, outside decision-time observations. Freeze selection and final
evaluation release rules before examining those outcomes.

Retain three separate evidence checks:

1. **Physical replay:** saved native commands reproduce the authoritative states,
   contact, events and outcomes. The existing evaluator supplies this layer.
2. **Controller reconstruction:** recorded requests, activation ticks and frozen
   backend reproduce commands and lifecycle feedback exactly. This establishes
   that execution honored the planner/controller contract.
3. **Planner reconstruction:** frozen deterministic decisions, or preserved
   stochastic streams, reproduce requests from the recorded observations and
   declared feedback. This establishes attribution to the named planner.

Layer two or three failure does not rewrite physical outcomes. A verified native
landing still needs target touchdown, mission success, stable integrity and
complete command replay. A qualified planner result additionally needs its
request/execution attribution checks. Summary runs remain measurements without
saved-command proof.

Record planner decisions/requests, acknowledgements, command authority, backend identity,
advice and interventions against native ticks. Existing finite telemetry and
markers can carry review fields. A complete structured trace and a create-only
companion receipt must bind its hash to the exact evaluation/capture identity;
that binding is an addition to implement, not an existing guarantee. Do not
rewrite sealed captures or mutate a supposedly frozen before/after evidence
scope with newly generated trace files.

Keep unsupported input, request rejection, native crash/off-target/timeout,
controller execution failure, capture failure and proof failure distinguishable.
Every scheduled attempt retains its disposition; repeats do not create more
unique physical missions. Preserve core/frontier/diagnostic distinctions and
report landing rate, contact violations and failure severity separately.
Compare fuel/time on common successful inputs and retain the paired exceptions.
Choose outcome/risk preferences before turning them into a training reward.

Budget native physics work, planner decisions, controller updates, advisory
queries, counterfactual trials, captures, independent replays, reconstruction,
checkpoint storage and presentation work separately. Measure a small pilot
before freezing a larger schedule and hard limits. Report rendering/publication
remain explicit operations using maintained templates and preserved evidence.

For learning comparisons, the independently fitted seed is a replication unit;
hundreds of missions from one model do not supply hundreds of independent fits.
Retain paired mission outcomes, group mirror partners and repeated physical IDs,
and distinguish fixed-corpus regression counts from sampled-population estimates.
Report per-fit results and uncertainty appropriate to that sampling design.
Freeze checkpoint selection and prevent repeated final-test-driven tuning. The
[research review](planner_controller_design_review.md#research-basis) motivates
these requirements without prescribing a universal minimum seed count.

## Deterministic characterization before learning

First validate software semantics with controlled implementations: command
authority, cadence, idempotent requests, rejection, reset, completion and exact
continuation. An alternate simple backend can check interface independence
without claiming new landing capability.

Then establish immediate-activation parity against direct terminal execution on
the maintained clean and trajectory-error packs. Match full command/state/fuel/
clock/outcome records under the same configuration, including failures; matching
only landing totals is insufficient. Historical receipts keep their source
identity, and retired traces are not assumed available as numerical oracles.

Finally declare a bounded activation characterization using:

- Maintained/exposed terminal inputs as regression and mechanism diagnostics.
- Real mission-derived approaches with reproducible native prefixes, selected
  by declared physical/input rules rather than reference-controller success.
- Explicit variation in position/velocity, pose/rate, mass/fuel and remaining
  mission time. Keep arbitrary below-target/climbing and new environment
  extensions separate from the initial supported envelope.
- Paired ordinary/retained execution if that candidate is selected for review.
- Immediate, never, fixed-delay and a declared current-assessment-derived
  deterministic trigger, all sharing controller and coast behavior.

Measure activation tick, first powered command, success/failure, contact
violations, fuel, duration and the width of successful activation windows. A
best-time sweep is a hindsight diagnostic upper bound, not a deployable planner.
Test whether useful windows remain resolvable at the proposed planner cadence.

The outcome should identify regions where activation timing matters, where
several choices are equivalent, where the controller fails, and where the current
advice is misleading. A simple deterministic trigger may be sufficient. A
learning allocation requires a remaining decision problem, a frozen usable
controller and independent evaluation inputs; another reward revision is not an
automatic consequence of a weak timing result.

## Ownership and next implementation boundary

The [controller-first design pass](maneuver_controller_design.md) details the
backend replacement boundary, first facade, command mapping and native
acceptance work. It is a follow-up draft rather than an implemented interface.

Reusable deterministic execution belongs in `pd-control`, preserving existing
controller IDs, defaults and serialized contracts. Approach composition, planner
logic and any Python binding that depends on controllers belong in the approach
package. `pd-python` remains a neutral physical/evidence binding. Keep missions,
runtime, evaluation and reporting free of concrete strategy imports.

The next implementation slice is an additive maneuver-controller facade, existing
terminal backend adapter, deterministic planners and composition harness. The
[review record](planner_controller_design_review.md) separates completed abstract
checks from pending native integration and flight acceptance. Before a flight
allocation, settle serialized request/trace binding and durable reconstruction,
pass native adapter/parity checks and declare the characterization cost/schedule.
Controller tuning, broader maneuvers and learning remain separate slices.
