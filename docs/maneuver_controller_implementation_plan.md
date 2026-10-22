# Maneuver controller implementation plan

[Controller design](maneuver_controller_design.md) · [Planner and controller design](maneuver_execution_design.md) · [Design review](planner_controller_design_review.md)

October 21, 2026. **Implemented experimental software pass.** The
[implementation record](maneuver_controller_implementation_results.md) owns final
checks and scope clarifications. The retained plan below defines the terminal
landing controller interface, one native episode and independent physical/decision
reconstruction. Completion means a runnable, tested deterministic composition
ready for bounded characterization. The
operating envelope remains a subsequent measurement task.

## Post cleanup starting point

The [RL code retirement](rl_code_retirement_results.md) is complete, with all
18 maintained developer checks passing. That establishes the platform baseline;
at that checkpoint the maneuver facade and `pd-maneuver-example` were unimplemented.
The four slices are now implemented. Cleanup preserved the controller/backend
behavior and supported first goal; the following slices define the accepted
software scope.

Start from the checkpoint containing the reviewed retirement and follow-up
design, rather than the earlier design-only commit `8fbb4eb`. Record the effective
source/configuration hashes before slice one. Preserve unrelated work and the
historical RL worktree when preparing an isolated checkout.

The new controller and example are Rust code with no dependency on the retired
PPO package, its `.venv`, old run directories or retained native SDKs. Historical
studies supply context and recorded comparisons. Reusable code is adopted only
through an explicit, separately tested change. The workspace's neutral Python
bridge retains its ordinary build prerequisites independently of this approach.

## Scope and completion boundary

The pass includes an additive maneuver-controller API in `pd-control`, the
unchanged ordinary terminal backend, and a small approach application with
immediate, never and fixed-tick activation programs. It includes software tests,
fresh-checkout parity cases, create-only evidence and a verification command.
Use 120 Hz physics, 60 Hz controller updates and a 10 Hz planner decision grid.

Keep terminal tuning, entry-gate policy and command transformations unchanged.
The first capability is `LandOnMissionTarget` on airborne, route-free landing
missions. Retained-plan experiments, arbitrary arrival goals, launch/transfer
control, retargeting, preemption, learning/rewards, Python control bindings and
durable mid-maneuver resume are later slices. Full terminal-pack parity and
activation/envelope campaigns need their own declared schedules; the software
pass does not establish those coverage claims.

## Code ownership

Paths below are proposed additions unless linked to existing source.

| Owner | Files | Responsibility |
| --- | --- | --- |
| Reusable controller | `pd-control/src/maneuver/{mod.rs,contract.rs,landing.rs,tests.rs}`; additive export in [lib.rs](../pd-control/src/lib.rs) | Request/lifecycle types, controller interface and ordinary terminal implementation |
| Native controller regression | `pd-control/tests/maneuver_parity.rs` | Direct-backend versus facade command and native execution equality |
| Approach composition | `examples/maneuver-policy/{Cargo.toml,README.md,src/lib.rs,src/main.rs}` | Deterministic planner programs, Episode scheduling, evaluation and explicit CLI |
| Approach evidence | `examples/maneuver-policy/src/{trace.rs,verify.rs}` and integration tests | Structured trace, byte binding and separate planner/controller reconstruction |
| Workspace integration | [Cargo.toml](../Cargo.toml), `Cargo.lock`, [development gate](../scripts/check-planner-development.mjs) and its tests when needed | Add package `pd-maneuver-example`; check its intended dependency boundary |
| Workflow documentation | [Development](development.md), [evaluation](evaluation.md), [reports](reports.md) and the design pages | Document implemented entrypoints and separate software acceptance from flight qualification |

The new example is a composition package and may depend on `pd-control`,
`pd-core`, `pd-run`, `pd-eval`, `pd-missions` evidence utilities and `pd-report`.
It must not import `pd-planner`, `pd-plan`, `pd-benchmark` or `pd-eval-cli`.
Keep the existing independent checkpoint example independent. Neutral platform
crates retain their current dependency restrictions. No new registry entry,
default selection or maintained planner change is required.

Keep the retired `experiments/rl` namespace empty of executable/project files.
Review the new application for calls into archived Python or old run paths as
well as checking Cargo dependencies. The retirement checker remains part of
acceptance; the isolated research checkout is optional preservation evidence.

## Slice one Controller API and ordinary backend

Add an experimental `ManeuverController` interface with descriptor, request
submission, command-frame update, feedback and native-end notification. Implement
it with `TerminalLandingController`, using a shared internal lifecycle helper.
Alternate controllers can implement the same supported goal contract. Keep the
existing `Controller` trait compatible and the terminal acceleration helper
private; the replacement boundary is the complete landing implementation.

Define versioned typed requests, admissions, feedback and final native
dispositions. Request identity is episode-local; the harness supplies the actual
physics tick. Keep planner `Keep` separate from new request submission. The
initial planner emits only Keep or a landing request; no generic JSON action
payload or general guidance-law hierarchy is needed.

Freeze serialization and admission precedence with table-driven examples before
wiring execution: malformed/unknown wire schemas are input errors; recognized
unsupported operations are semantic rejections. A recognized cancellation can
return Unsupported without becoming a supported planner action. A missing pad
is invalid; a request for another existing pad is unsupported retargeting.
For an active landing, an identical ID/payload is AlreadyActive, changed payload
under that ID is InvalidRequest, and a different otherwise-valid request is Busy.
Stale ticks are invalid before any state mutation. Semantic rejections preserve
the current command owner.

Freeze the ordinary resolved terminal configuration with terrain prediction
enabled and experimental flags disabled. Identify configuration independently
of runtime state. Reject unsupported tuning through this first constructor;
the existing finite-value validator is not a general physical-range validator.
Construct/reset backend state once at activation, then call its existing update
from that boundary's actual observation. Before activation emit `Command::idle()`.
Preserve existing native engine-setting semantics, attitude commands, telemetry
and markers; add no angle clamp or global throttle conversion.

The facade owns no plant and predicts no successful completion. Native end
notification carries the final tick, physical/mission outcomes, stop and failure
dispositions. It is delivered once even when landing was never requested.
Frame generation after closure, duplicate/out-of-order command ticks and context
replacement are execution errors. Queries and duplicate requests must not advance
backend state. Each episode uses a fresh facade; durable restore is not exposed.

Admission checks mission/environment semantics, not predicted success. Pin the
vehicle-family invariants and allowed physical variations in the capability
descriptor and tests. In particular, retain current mass/fuel and original
deadline; finite difficult/low-fuel starts cannot be rejected just because the
reference backend loses. Ascending and below-target diagnostics carry no
inherited performance claim.

Distinguish unsupported whole inputs from rejected requests during an otherwise
supported episode. Validate mission/environment support before execution; an
unsupported input gets that disposition rather than a fabricated flight result.
After execution starts, ordinary request rejection continues the current owner
and preserves the actual native prefix.

**Exit:** deterministic test implementations exercise the same public contract,
the terminal adapter has immediate-activation parity on the native regression
panel, and existing controller tests pass without numerical changes.

This is the first review checkpoint: the additive public contract, ordinary
terminal implementation, lifecycle tests and exact native parity panel form
one complete library change. Freeze the panel inputs and the comparator's narrow
identity/timing exclusions before comparing results. Review that checkpoint
before adding the new workspace package, CLI, trace storage or report command.
The later slices complete the runnable composition; passing slice one alone
does not establish harness or operating-envelope acceptance.

## Slice two One episode and deterministic programs

Implement the scheduler in `pd-maneuver-example`, directly owning a
[`pd_run::Episode`](../pd-run/src/episode.rs). Keep the original validated context
for the trusted backend; expose only `MissionView`, actual observation and common
lifecycle feedback to the planner. Use `scenario_horizon_limits` for ordinary
runs. Explicit shorter bounds belong in tests and must retain their bounded-stop
disposition.

At each live command boundary, obtain the actual observation, invoke the planner
only when a choice is available and its 10 Hz decision is due, acknowledge the
decision, produce one controller frame and step the episode. Activation starts
the backend on that same boundary. After commitment the backend continues at
60 Hz while planner invocations stop. A rejected request consumes the normal
interval through the current owner. Only genuine execution errors use
`Episode::decision_failed`.

Immediate activation requests at tick zero. Never always chooses Keep. Fixed
activation requests at a declared absolute tick on the 10 Hz decision grid
(multiples of 12 physics ticks). Reject off-grid program configuration rather
than round silently. A target tick beyond native termination simply never fires.
Use one stable request ID and no randomness or backend-specific advice.

Close the facade from the actual Episode stop/summary, including partial command
intervals and errors, before another callback. Convert captured Episode output
to the existing `BoundedAgentRunArtifactsV1` and call
[`assess_execution`](../pd-eval/src/episode.rs); no strategy hook or completion
callback is added to neutral runtime/evaluation.

The runnable evidence path is captured execution. A summary-only library path
retains counters and final physical summaries without per-step trace vectors;
it cannot later manufacture a verified bundle. Test summary/captured physical
equality separately from captured trace and replay acceptance.

**Exit:** scheduling tests establish real 120/60/10 Hz behavior, no extra coast
interval at activation, no planner calls after commitment, unchanged native
budgets and correct completion/error accounting in captured and summary modes.

## Slice three Trace binding and reconstruction

The composition descriptor binds program, facade/backend configuration, cadences,
first-decision phase, coast behavior and command mapping. The explicit evidence
scope covers manifests/lockfile, all participating source packages, scenario and
program inputs, plus the executable identity supplied by `EvidenceScope`.
Exclude generated destinations from that before/after scope.

Bind the actual compiled source and executable, rather than a Git HEAD plus a
dirty flag or an old RL closure receipt. Historical archives and unrelated
documentation are outside the new execution scope. The five pre-existing
documentation hash mismatches recorded by retirement retain their old
provenance; implementing this approach does not rewrite those run bindings.

Record a versioned trace of actual planner opportunities/decisions, request
admissions, accepted goal/activation, command authority at each boundary and the
final native disposition. Native commands and existing backend telemetry remain
in the ordinary capture. Bound trace loading to regular files and the declared
native horizon/update counts; reject unknown schemas and inconsistent ticks.
Keep wall/CPU timings outside exact behavioral
reconstruction; count physical steps, backend updates and planner calls explicitly.

Use a new bundle directory with three siblings: `capture/`, `trace.json` and
`receipt.json`. Let neutral `save_evaluation` reserve `capture/` and write its
existing evaluation/seal unchanged. Write the companion receipt last, binding
the exact evaluation/seal and trace byte hashes, descriptor and input identity.
Never append to an existing capture. If a write fails, retain a visibly incomplete
new bundle; absence of a complete receipt must fail composite verification.
Do not fill in missing pieces of a previous attempt on retry.

Verification has three independently reported results:

1. **Physical replay:** load and validate the sealed ordinary evaluation using
   the neutral verifier, including physical/mission outcome and integrity.
2. **Controller reconstruction:** feed recorded requests at recorded ticks into
   a fresh frozen controller on observations regenerated through native command
   replay; compare admissions, every command, authority and final lifecycle.
3. **Planner reconstruction:** regenerate the deterministic program's decisions
   from the recorded opportunities and verified observations/feedback; compare
   requests and the absence of calls after commitment.

Bind trace fields to authoritative native observations/ticks rather than trusting
the trace to grade itself. Report a mismatch without rewriting physical outcomes.
Backend/source mismatch prevents a reconstruction claim; command-only physical
replay remains independently meaningful. Count all verification/reconstruction
passes in resource receipts, including replays invoked by existing save/load
validation APIs.

Add a small explicit CLI: run one supplied scenario/program into a new bundle;
verify an existing bundle read-only; render one explicit report destination
through the maintained rich template. No collection sweep, report publication,
selector update or server lifecycle action is part of these commands. Report
the trace/reconstruction dispositions in a separate receipt/CLI summary first;
new report UI is not required for this software pass.

**Exit:** valid bundles verify, all selected tampering cases fail the relevant
check, existing outputs are never overwritten, and a partial write cannot be
mistaken for completed composite evidence.

## Slice four Integration and review

Wire the new example into workspace tests and the maintained development gate.
Add a dependency check allowing `pd-control` for this composition package while
rejecting the planner/legacy composition dependencies above. Preserve all
existing neutral-package and default-off CLI checks.

Document runnable commands only after the application exists. Update current
workflows surgically, preserving the already-dirty evaluation/report/navigation
work. Review each slice separately; commit/push and measured campaign execution
are distinct follow-up actions.

**Exit:** the focused tests and full maintained developer gate pass, an ordinary
fresh checkout needs no ignored capture, and final review confirms that existing
controllers, fixtures, historical receipts and report selectors are unchanged.

## Validation panel

| Check | Required evidence |
| --- | --- |
| Lifecycle and requests | Same-boundary acceptance; Keep/duplicate/rejection preserve state; conflicting ID, unsupported goal/mission, cancellation, stale tick and malformed input have distinct dispositions; no post-end command |
| Backend independence | A deterministic alternate test implementation satisfies the contract without terminal internals; it supplies no landing-capability claim |
| Native parity | Predeclared small panel covering nominal and loaded descent, lateral cleanup, nonzero attitude, terrain prediction, near-contact settling, low fuel/exhaustion, short deadline and difficult entries; compare direct default backend with immediate facade activation |
| Exact comparator | Underlying commands, states, actions/events/samples, fuel, clocks, contact and outcomes match, including failures; original backend telemetry matches; compare explicit identity/timing additions separately with a narrow field list |
| Tick handling | Native end at an odd physics tick; activation at zero and a later decision tick; deadline before activation; already-stopped bounded episode makes no decision or frame call |
| Capture modes | Summary and captured modes have identical physical summaries; only captured execution makes saved-command replay claims |
| Reconstruction | Changed request tick/payload, admission, owner, command, final result or program identity is detected; controller and planner checks fail independently |
| Bundle integrity | Changed trace/evaluation bytes, missing receipt, wrong hashes, existing destination and injected partial write are rejected without modifying prior evidence |
| Platform preservation | Existing workspace/controller tests, dependency and CLI boundaries, formatting, Clippy, Node tests and documentation checks pass |

Freeze the small panel's case IDs/physical inputs before evaluating the adapter;
derive synthetic variants without editing frozen fixture files. Use native-valid
timing tests: scenario `max_time_s` must remain positive; zero-step termination
uses an explicit bounded hard end at zero. Use a fractional positive horizon or
short explicit bound for odd-tick termination. The abstract model's zero-length
schedule is not itself a valid zero-time scenario.

Full clean/trajectory-error pack comparison remains a separate explicit parity
run. Current matrix expansion is internal to `pd-eval-cli`; do not copy its
generator into the new approach or import that application for convenience.
When scheduling that comparison, use an evaluator-owned adapter and preserve
the original resolved-input identities. Historical saved outcomes are not the
numerical oracle for changed source: compare contemporaneous direct and wrapped
execution with the same configuration, retaining historical provenance separately.

The retained research-worktree `terminal_input_export.rs` is a possible starting
point for that later evaluator-owned access, not an implementation dependency.
Its interface covers clean-family variations; adoption needs parity against
the current resolver and explicit trajectory-error coverage. Keep the first
software panel independent of that extraction and preserve its archived source.

## Validation commands and stop conditions

After implementation, run focused tests before the maintained gate:

```sh
rtk proxy node scripts/check-rl-retirement.mjs
rtk proxy cargo test -p pd-control
rtk proxy cargo test -p pd-maneuver-example
rtk proxy node scripts/check-planner-development.mjs
```

These commands are now runnable. The first library checkpoint passed the
retirement/controller checks and full maintained gate before adding the example.
The retained 44-case planner capture remains optional under the existing
`--parity-capture`
workflow and is not a substitute for terminal adapter parity.

Stop on the first unexplained command/state mismatch, lifecycle/clock violation,
evidence overwrite or dependency leak. Reduce it to a native regression and fix
the adapter/composition; do not tune the backend, weaken comparisons or remove
failing inputs to complete the pass. Keep changes additive so the new module and
example can be unselected while existing controller paths remain runnable.

The handoff contains reviewed source, passing software checks, the exact parity
panel and remaining coverage limits. A later characterization declaration then
fixes input populations, activation programs, full-pack/fresh/frontier roles,
cost pilot and hard limits. It can decide whether timing offers a useful planner
problem; no training allocation follows automatically from this software pass.
