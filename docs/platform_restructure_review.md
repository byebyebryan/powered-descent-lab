# Platform restructure review and validation

[Restructure plan](platform_restructure_plan.md) · [Documentation home](README.md)

October 10, 2026. Outcome: the revised design is ready for a bounded first
implementation slice. This is a source and design review; runtime extraction,
generator performance and RL training remain implementation acceptance work.
Controller tuning is parked and no new flight result is claimed.

## Review scope

The goal loop captured the proposed design, checked current ownership and
dependencies, challenged migration and mission-management assumptions, revised
the plan, and applied documentation validation. The source checkpoint is
`a449975e3f842094455c228b452e6b9b3e7801f4`, with existing report/navigation/server
changes in the checkout. Those unrelated files are outside this pass.

The review uses current source and the documented
[combined-safety results](ballistic_combined_validation_results.md). It preserves
the accepted Policy 3 boundary, opt-in development candidates, and every frozen
historical input and outcome. It does not repeat the numerical flight audits.

## Ownership findings

| Finding in current source | Consequence for the design |
| --- | --- |
| [Cargo evaluation dependencies](../pd-eval/Cargo.toml) include controller, planner and report implementations | Generic evaluation and built-in application registration must be separate packages |
| [Report dependencies](../pd-report/Cargo.toml) and [renderer inputs](../pd-report/src/lib.rs) include executable controller types | Extract neutral trace contracts and saved descriptor decoding before claiming independent reporting |
| [Controller execution](../pd-control/src/lib.rs) already accepts `dyn Controller`, but [resolved run descriptors](../pd-eval-cli/src/model.rs) require `ControllerSpec` | The algorithm interface is partly open; the registry and persisted execution descriptor still constrain the evaluator |
| [V2 sessions](../pd-eval-cli/src/waypoint_v2.rs) own live state and execute complete pieces synchronously | Support session/program execution alongside online policies; a per-tick trait alone is insufficient |
| [Ballistic execution](../pd-eval-cli/src/ballistic_feedback.rs) mixes acquisition, recovery, terminal composition, disk capture, repeat verification and rendering | Extract solver execution from orchestration of evidence; training must not call the experiment frontdoor |
| [Core stepping](../pd-core/src/sim.rs) distinguishes ordinary stepping from physics-only contact classification | Actual training and evaluation episodes need ordinary mission/contact/timeout semantics |
| [Ordinary clearing execution](../pd-eval-cli/src/local_clearing.rs) uses global two-tick command boundaries | Preserve global command order and existing cadence during host extraction |
| [Terminal builders](../pd-control/src/terminal/mod.rs) change coupled private switches outside the [serialized configuration](../pd-control/src/terminal/config.rs) | Complete new approach configuration must record resolved effects while historical descriptors retain their original identity |
| [Terrain source loading](../studies/terrain_profiles/study.py) extracts hash-bound classes from an external checkout; [refinement](../studies/terrain_profiles/refinement.py) fixes pads and templates | Add a self-contained versioned generator; do not rewrite historical study recipes |
| [Challenge generation](../studies/terrain_profiles/challenge.py) already avoids solver feedback and corridor clearing | Preserve this property while widening complete mission dimensions |

The graph is currently acyclic. These findings concern dependency direction and
mixed responsibility, not a failure of the physical model or proof system.
Existing authored waypoint contracts are legitimate mission rules. A planner's
internally selected route is a different concern and does not become a mandatory
waypoint contract for every independent agent.

## Design gaps resolved

1. **Planner ownership:** retain pure math in `pd-plan`; place maintained solver
   orchestration and its terminal composition in proposed `pd-planner`. Its
   dependency on current controllers does not enter the neutral platform.
2. **Application packaging:** make `pd-eval` a neutral library and move the
   `pd-eval` binary to proposed `pd-eval-cli`. Keep installed commands, explicitly
   migrate Cargo package callers, and preserve the default-off CLI boundary.
3. **Runtime modes:** define policy, session and actual host responsibilities.
   Keep copied prediction separate from actual stepping; retain existing time
   zero, contact and command-chunk behavior.
4. **Stops and restart:** distinguish physical, mission, algorithm and resource
   dispositions. Preserve zero-command bounded proof and retained-prefix replay.
   Fresh airborne initialization does not authorize arbitrary historical restart.
5. **Planning reserve:** the 90-second terrain mission's 80-second planning bound
   and short-query allowance remain algorithm policy, not a shared environment
   deadline. The source defaults are in
   [planner input](../pd-eval-cli/src/planner_flight/input.rs), with
   [deadline calculation](../pd-plan/src/waypoint_v2.rs).
6. **Generation rejection:** use declared structural checks, finite deterministic
   rejection and typed generation failures. Never substitute another input
   because a controller stops. Track generation counts separately from runs.
7. **Input identity:** retain complete bytes plus physical mission identity,
   version numerical generation and PRNG substreams, and detect duplicate content
   across names, seeds and collections.
8. **Collection roles:** immutable ordered inputs have a separate usage record.
   The observed fresh 1k is regression evidence. More seeds and wider dimensions
   are different coverage claims; sample size and uncertainty need declared rules.
9. **Reports and diagnostics:** preserve rich views while registering PDG
   diagnostics separately. The existing parabolic shape reference is a
   [documented diagnostic](transfer_suite.md), not a scoring rule to silently
   replace with a controller-specific trajectory.
10. **RL acceptance:** reward and feature encoding belong to the RL adapter.
    Shared physical semantics, explicit information modes, command replay,
    complete checkpoint/configuration identity and resource accounting are
    prerequisites for meaningful comparison.
11. **Migration size:** start with a wrapper over existing core execution and one
    ordinary controller. Low-capture execution, solver relocation and CLI
    packaging changes belong to later slices; enforce new neutral boundaries as
    each crate is introduced.
12. **RL timing:** a small independent adapter can follow the runtime and mission
    milestones. Generic evaluation and throughput checks precede substantial
    training; retiring every historical executable is not an RL prerequisite.

## Adversarial design checks

| Situation | Required behavior in the revised plan |
| --- | --- |
| An independent agent has no PDG phases or controller registry entry | Neutral factory/descriptor admission; optional diagnostics; no built-in dependencies |
| An agent finishes a command chunk at a terminal contact | Host stops at authoritative termination before accepting a further update |
| A planner rejects the mission before issuing a command | Typed algorithm stop and truthful bounded evidence; no fabricated crash or landing |
| A training episode reaches its mission clock while still airborne | Ordinary core timeout and explicit completion reason even with summary capture |
| A generated airborne mission is unsupported by the existing planner | Preserve it in the full collection denominator; expose any secondary supported slice |
| Two seeds or labels produce the same physical mission | Detect overlap through physical content identity rather than treating labels as independence |
| Generation order or worker count changes | Versioned substreams and per-proposal identity retain resolved content |
| Every proposal fails structural validation | Bounded generation failure with rejection reasons, distinct from mission failure |
| An observed evaluation collection is relabeled unseen | Usage history prevents restoration of unseen status |
| A terminal builder enables additional switches | Resolved approach configuration records the full effect; old serialized hashes are untouched |
| An experiment has historical reports but no maintained execution consumer | Preserve readers and evidence; retire its executable only after explicit caller/test mapping |
| GPU inference cannot rerun identical policy decisions | Report policy-repeat limits separately; captured native commands still require deterministic plant replay |

These checks establish design requirements. Their executable counterparts belong
to the migration stages; they are not claims that the proposed APIs already pass.

## Decisions deferred to implementation

Exact Rust signatures and the small first-slice file list are finalized before
editing runtime code. Generator ranges and structural rejection limits are
declared in the first distribution version, then characterized without a solver.
Throughput and memory measurements determine batching and training scale.
No RL framework or learning algorithm is selected in this review.

No unresolved ownership decision blocks the first slice. Later stages require
their own migration inventories and evidence. In particular, repository-level
independent evaluation and reporting are accepted only after transitive
controller/planner imports have actually been removed.

## Documentation validation

The focused documentation checks passed:

- `rtk proxy node scripts/check-docs.mjs`: 187 Markdown files, 1,232 local
  links, zero issues; 88 generated-evidence links intentionally skipped.
- `rtk proxy node --test scripts/test-check-docs.mjs scripts/test-check-planner-development.mjs`:
  13 passed, zero failures.
- `rtk proxy git diff --check`: passed.
- The dependency table was parsed and checked: eight proposed packages form an
  acyclic graph; all five neutral packages have no transitive strategy dependency.
  This validates the proposed graph, not unimplemented Cargo manifests.
- SHA-256 comparison preserved all ten pre-existing modified/untracked files.

The changes in this pass are the plan, this review record, documentation-home
links and active roadmap status. These checks do not execute flights, regenerate
reports or establish runtime numerical parity. The goal loop closes at the
reviewed plan; implementation begins as a separate bounded task.

## Unattended execution adjustment

The subsequent user request expands the intended implementation run to roughly
a day of autonomous progress. The [execution plan](platform_restructure_execution.md)
supersedes the initial small-first-slice stopping point while retaining the
reviewed architecture and evidence boundaries. Implementation has not started.

The adjustment changes four execution decisions:

1. The goal spans the full platform, with accepted slices as checkpoints. A
   controller wrapper alone does not satisfy completion.
2. Neutral runtime and missions lead to generic evaluation/reporting and an
   independent policy adapter before the deepest maintained-solver extraction.
   Transitional legacy execution stays application-owned, is inventoried, and
   must be extracted before the whole restructure is called complete.
3. Routine API, recipe, test and adapter choices are made within the launched
   goal without repeated approval. Existing dirty work and frozen evidence remain
   protected; publication, tuning and substantial RL training remain outside it.
4. Validation has explicit allowances, milestone records and an anti-stall rule.
   Input characterization is separate from flight evidence; inheritance of a
   failed baseline gate is explicit rather than silently treated as success.

Current source confirms why the packaging transition is needed: the existing
evaluation binary registers built-ins through its strategy-dependent library,
the feature-enabled CLI imports its session directly, and reports import
controller descriptors. The retained 44-case parity directory and accepted
selector/report paths exist at preparation time; their integrity is rechecked
at launch. No new numerical audit or flight was performed in this adjustment.

The execution-plan adjustment passes documentation validation: 188 Markdown
files and 1,245 local links, zero issues; all 13 relevant Node tests pass;
`git diff --check` and new-file whitespace checks pass. SHA-256 comparison keeps
all ten unrelated modified/untracked files unchanged. These are documentation
and preservation checks, not implementation or flight acceptance.
