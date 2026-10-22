# Maneuver controller first implementation

[Planner and controller design](maneuver_execution_design.md) · [Research and validation review](planner_controller_design_review.md) · [Current guidance](guidance.md)

October 21, 2026. Source-backed follow-up to design commit `8fbb4eb`.
**Design draft; no production interface or new flight evidence.** The terminal
backend is the better-established starting point. Its maneuver lifecycle and
operating envelope still need validation before a planner can rely on them.

## What already exists

| Part | Current implementation | Consequence |
| --- | --- | --- |
| Feedback commands | [Terminal update](../pd-control/src/terminal/mod.rs) combines candidate selection, hysteresis, ZEM/ZEV acceleration, tilt/thrust allocation, settling and touchdown rescue | Adapt this whole implementation first |
| Command interface | [Controller](../pd-control/src/lib.rs) supplies ID, reset and command-frame update | Preserve that existing trait and its callers |
| Complete construction | [Resolved configuration](../pd-control/src/terminal/config.rs) includes tuning and all experimental flags | Use it for identity and fresh construction; it excludes live backend state |
| Physical execution | [Episode](../pd-run/src/episode.rs) advances the actual plant, retains clocks/fuel and exposes native stops | Keep the controller outside plant ownership |
| Request lifecycle | No goal identity, admission, commitment or final-result delivery in the existing Controller trait | Add a facade around the backend |
| General arrival control | The shared acceleration helper is private; terminal targets, candidates and touchdown handling are landing-specific | Arbitrary position/velocity arrival remains a later capability |

Existing tests cover terminal command generation, several difficult landing
regressions, reset, hysteresis, retained plans and configuration reconstruction.
They do not exercise the proposed maneuver facade. This pass reviewed source and
tests; it did not rerun a controller campaign or establish a new landing rate.

## The replacement boundary

Keep one public **maneuver controller** boundary, with a lifecycle facade and a
replaceable **landing backend** inside it. The backend replacement unit includes
candidate/horizon selection, its prediction model, state retention, command
allocation and touchdown behavior. Changing only the acceleration formula leaves
the old law's candidate and terrain checks in place, which is not a validated
way to substitute a new controller.

The initial backend delegates to `TerminalPdgController`. A later backend can
use another feedback or optimization method while serving the same supported
landing request. Give each effective implementation a complete descriptor and
characterize it separately. Keep the mathematical helper private for now;
extracting a public guidance-law hierarchy is not needed to wrap this backend.

The existing transfer controller also chooses boost/coast/terminal phases and
owns its entry policy. Wrapping the full transfer implementation would include
those decisions in the controller comparison. It would not isolate the proposed
planner's landing-activation decision. Start with terminal landing alone.

## Recommended first contract

Use the vocabulary and admission rules in the [committed design](maneuver_execution_design.md#initial-request-and-feedback-vocabulary).
The first facade accepts `LandOnMissionTarget`, begins in upright coast and
commits to one landing attempt. The planner can choose when to activate it.
Retargeting, cancellation, preemption and non-landing goals remain unsupported.

| Operation | First implementation responsibility |
| --- | --- |
| Construct/describe | Validate a frozen configuration and declare supported mission/environment semantics before execution |
| Admit request | Check goal, mission target, request identity and actual submission tick; update only lifecycle state |
| Produce frame | Emit upright coast, or update the active landing backend exactly once at this native command boundary |
| Observe native end | Close once from actual physical/mission outcome, bounded stop/failure and final tick; produce no command |

Admission and command production are separate operations within the same native
boundary. Acceptance initializes backend state once, then command production uses
that boundary's actual observation. It cannot insert a coast interval or wait for
readiness. Keep, duplicates and rejections never reconstruct or reset an active
backend. The harness advances one ordinary interval after a rejected decision;
submission itself is not a zero-time retry loop.

The facade owns the coast/landing command choice. The harness owns when to ask
for it and when to step the episode. Keep exactly one command owner at every
live boundary. Native contact/deadline can end between boundaries; close from
that actual result before another request or backend update.

The trusted legacy adapter can use the original validated `RunContext` needed
by the existing Controller trait. Activation does not construct a new scenario
or episode. The planner receives only the physical `MissionView`, observation
and declared lifecycle feedback. Scenario labels and collection roles stay out
of its input. Completion passed into the facade is a native disposition;
integrity, replay and qualification remain the evaluator's later responsibility.

## Configuration and commands

Begin with the ordinary default terminal configuration, predicted terrain
clearance enabled and all experimental flags disabled. Freeze the complete
resolved configuration, source/build identity, 120/60 Hz execution, upright
coast and command transformations. Preserve raw backend commands; add no
external angle clamp. The historical retained-arrival candidate and its clamp
remain a separate comparison after ordinary adapter parity.

The resolved configuration validator checks version, coupled flags and finite
tuning. It does not establish valid physical ranges for every arbitrary tuning
value. The first slice should accept the frozen default configuration rather
than expose a general tuning surface backed only by this validator.

One actuation detail needs explicit accounting. The nominal
[acceleration allocator](../pd-control/src/guidance.rs) emits requested thrust
norm divided by maximum acceleration as the engine setting. The
[native plant](../pd-core/src/sim.rs) maps positive engine setting `u` to applied
fraction `m + u * (1 - m)`, where `m` is minimum throttle. Other terminal branches
use [an inverse conversion](../pd-control/src/terminal/planning.rs) when targeting
an applied thrust fraction. These are existing, distinct paths. For example,
`u = 0.5` with `m = 0.25` produces applied fraction `0.625`.

Preserve both paths in the parity adapter. Do not add a global inverse mapping
at the facade, which would change some commands and convert others twice.
An eventual control-allocation correction needs its own behavior review and
comparison. When diagnosing an alternate backend, distinguish requested
acceleration, emitted setting, applied thrust and realized acceleration.

Retained-plan mode also changes more than the arrival clock: the terminal source
uses a touchdown-centre vertical target while the retained guidance plan is
active. Preserve that backend's complete behavior and configuration rather than
implementing a nominally equivalent countdown in the facade.

## Admission and measured capability

Separate three questions:

1. Does this implementation support the mission/environment and goal semantics?
2. Will it attempt this valid request from the actual current state?
3. On which declared input populations does it achieve verified native landing?

The initial semantic scope remains airborne, route-free `LandingOnPad` missions
on the fixed mission target, with the declared vehicle/environment/cadence.
Predicted success is not an admission prerequisite. Low fuel, late activation
and difficult finite states retain honest outcomes. Ascending and below-target
entries need explicit characterization strata; they have no inherited terminal
coverage claim. Do not turn those limits into a controller-success filter.

`ballistic_entry_ready` is a caller-specific adapter: it requires descent,
uses a caller-supplied projected miss and evaluates with terrain ignored.
The transfer controller adds its own projection and ascending-deferral rules.
Neither is the generic request-admission rule. The first planner gets no
backend-specific advice; a deterministic advice-based comparator must declare
the exact assessment and its cadence/history separately.

Candidate terrain clearance is sampled prediction under the guidance model,
with target attitude and a relief threshold. It is not executed route clearance,
full actuator/fuel/deadline feasibility or native contact proof. Terminal phases
and a retained plan's internal completion flag likewise do not close a maneuver.

## State and restart

The facade retains accepted request/payload/tick, coast/active/closed state and
the final native disposition. The backend retains its own readiness counters,
guidance mode, settling state and optional plan. The harness retains planner
cadence and decision history. Keep these owners distinct in trace reconstruction.

Use fresh whole-episode runs and original-prefix reconstruction for the first
characterization. Do not offer durable mid-maneuver resume in this slice.
`resolved_configuration()` constructs a fresh backend; it is not a checkpoint.
Original-prefix reconstruction must rerun requests and backend updates as well
as physical stepping. Command replay alone reconstructs the plant, not the
controller's hidden state. Native exactness and reconstruction cost remain
acceptance checks; public state serialization can follow an actual resume need.

## First implementation and acceptance

Add deterministic lifecycle/types and the terminal adapter inside `pd-control`.
Keep existing Controller/Agent contracts and IDs. Approach scheduling/planners
belong in the composition package; runtime/evaluation/Python plant bindings stay
independent of concrete strategies.

The first acceptance sequence is:

1. Exercise the real facade with deterministic test backends: same-boundary
   activation, one command owner, duplicate/rejection preservation, reset,
   configuration identity, genuine errors and native-end closure.
2. Compare immediate activation with direct terminal execution under identical
   configuration. Require exact commands, states, clocks, fuel and outcomes;
   preserve failures and assess added metadata separately.
3. Verify real Episode scheduling and request-to-command reconstruction,
   including partial terminal intervals and unchanged native budgets.
4. Measure a small cost pilot, then declare an activation/envelope comparison
   with frozen physical inputs, native prefixes, deterministic request programs
   and separate regression/frontier/fresh populations.

The existing abstract checker remains a specification aid. The new facade needs
native tests, and the operating envelope needs measured flights under a separate
bounded declaration. Controller tuning and RL training are later decisions.

The [implementation plan](maneuver_controller_implementation_plan.md) turns this
boundary into four reviewable software slices, with file ownership, a minimal
native runner, trace reconstruction and explicit acceptance checks.
