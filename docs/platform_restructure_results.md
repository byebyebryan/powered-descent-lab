# Platform restructure implementation record

[Plan](platform_restructure_plan.md) · [Execution contract](platform_restructure_execution.md) · [Review](platform_restructure_review.md)

Status: complete, October 10, 2026. All six completion conditions in the
execution contract are met. Accepted implementation, workflow closure and final
integration/preservation checks are recorded below.

## Baseline and ownership

Starting commit: `a449975e3f842094455c228b452e6b9b3e7801f4`.
The source inventory, starting binary patch and copies of initially dirty files
are retained in `outputs/restructure/run-20261010T170648Z/`. `baseline.json`
records 539 tracked/untracked source identities and ten protected dirty paths.
The complete maintained developer gate passed all 11 checks before source edits;
its log is `baseline-gate.log` in that run directory.

Existing report navigation/server work is unrelated and remains owned by its
original writer: root README, reports documentation, navigation fixture and
renderer, scripts README/server launcher, navigation discovery module, Python
server and server/library tests. These starting changes are excluded from this
goal's commits. The architecture plan, review, execution contract and their docs
home/roadmap links are owned by this goal.

The retained Policy 3 comparator
`outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native` was available
at launch. Frozen inputs, historical outcomes, selectors and retained reports
are preserved. New validation artifacts stay under the unique run directory.

## Accepted checkpoints

- A: starting ownership and source inventory captured; baseline gate passed.
  The owner explicitly authorized commits during implementation. No push or
  publication is included.
- B: `pd-run` owns neutral frames, telemetry, update records, performance and
  online/prepared execution. `pd-control` re-exports the unchanged serialized
  record shapes and adapts its existing reset/update interface. The open `Agent`
  interface receives physical inputs without scenario labels, seeds or tags.
  Three maintained controllers were compared with fresh instances using the
  direct core callback oracle: exact commands, frames, events, samples and full
  manifests matched. Four independent runtime tests cover complete replay,
  prepared binding/cadence, odd hard endpoints, zero commands, failed decisions
  and membership-independent actions. Reset failures retain a typed zero-step
  stop. Incremental reset/step and capture allocation checks remain in E.
  `runtime-tests.log` records 176 controller and four independent tests passing.
  The full maintained gate is recorded in `runtime-gate-v2.log`; the first
  attempt caught a constructor naming error before execution and is retained.
  The all-features normal `pd-run` tree contains only core and utility crates.
  All ten unrelated starting dirty files remain byte-for-byte preserved.

- C: `pd-missions` implements versioned recipes, index-local
  seeded value-noise terrain, local pad preparation, finite structural rejection,
  create-only collections, physical/byte identities and append-only usage receipts.
  Seven contract tests and strict crate Clippy passed. Characterization admitted
  10,000 unique missions with one rejection/retry and no exhausted requests:
  10,001 proposals, one more than the plan's nominal 10,000 proposal allowance.
  No solver results informed this distribution. All 10,000 requests were repeated
  in reverse order with exact result identity. The 64-mission integration
  collection was generated and verified; see [workflow/coverage](procedural_missions.md).
  Recipe and mission clocks/resources are explicit. Input readers do not regenerate
  terrain, and no Pylander checkout or retained capture is a dependency.
  The full maintained gate passed all 11 checks (`missions-gate.log`).
  The pre-composition retained comparator passed exact numerical parity on all
  44 cases (`pre-composition-parity.log`); timing exclusions remain the three
  original wall-time fields. Retained inputs and artifacts were read-only.

- D: the old application moved to `pd-eval-cli`, retaining the installed binary
  name `pd-eval`. Neutral `pd-eval` now owns open-agent evaluation, complete
  scheduled attempt accounting, create-only capture seals, explicit source/input
  integrity and native command replay. Physical prefix verification remains
  distinct from complete episode verification and the verified landing tuple.
  `pd-report` consumes neutral records and opaque configuration JSON; its common
  rich template is preserved. A generic report projection includes proof/stop
  disposition, capture navigation and display-only unsampled endpoints.
  Three independent evaluation tests cover capture/replay/rendering, source
  drift, failed decisions, capture tampering, factory failures and explicit repeats.
  Source fingerprint scopes and current package callers follow the moved code;
  historical binary names, receipts, inputs and outcome records are unchanged.
  The installed command's application registration and legacy solver modules are
  transitional exceptions mapped in the [application inventory](../pd-eval-cli/README.md).
  They still require milestone F; this relocation is not solver extraction.
  The integrated gate passed 16 checks, including four all-features normal-tree
  boundaries and exact retained 44-case parity (`composition-gate.log`). All 207
  terrain pure/synthetic Python tests passed (`composition-python-tests.log`).
  Current docs/source links and Cargo package examples were migrated together.
  Three initially dirty files needed narrow integration edits (root README,
  reports doc and server launcher); their starting navigation/
  server hunks are preserved and excluded from commits. Only this goal's changes
  relative to their clean HEAD versions were staged for those paths. The server
  launcher's new build selector is inside an original uncommitted hunk and
  remains with that hunk; it cannot be committed alone against clean HEAD.

- E: `pd-run::Episode` provides explicit mission reset and incremental ordinary
  steps. Online agents and prepared programs use its captured mode. Summary mode
  retains no trace vectors; rewards and training stay external. Eight existing
  core projection/validation helpers became public without changing their bodies.
  Direct core comparisons pass at every tested control boundary, including zero,
  odd endpoints, contact, checkpoint, horizon and failed decisions. Numeric or
  unserializable telemetry is rejected before a command is recorded. Caller-owned
  guarded execution retains the original bounded core backend.
  The independent `pd-policy-example` package has complete checkpoint, feature
  scaling, clipping and action mapping; its all-features normal tree contains no
  maintained controller/planner. Its demo weights remain fixed and untrained.
  The 64-mission development collection plus two declared repeats produced 66
  captures and 66 maintained rich detail reports, with 66/66 stable integrity and
  complete native replay. Both repeats matched exact execution after excluding
  measured update/performance timing. All demo episodes crashed on tick one;
  this validates integration and honest outcome accounting, not landing ability.
  Collection-wide evidence freeze is recorded as well as each episode's binding.
  `policy-integration/` retains all attempts; native replay was also exercised
  through the read-only example command. `policy-throughput.json` records 32
  summary and 32 captured early-stop runs plus eight full proof assessments;
  those timings mainly measure initialization and evidence cost.
  A separate predeclared synthetic held airborne trace measures sustained
  runtime: eight summary and eight captured runs each advanced 11,232 aggregate
  physics ticks (1,404 each) and 5,616 control updates. Every physical summary
  matched, and all eight captures replayed exactly. Aggregate execution was
  3.56 ms summary / 4.41 ms captured; replay 4.08 ms, serialization 1.64 ms for
  2.05 MB, process peak RSS 4,232 KiB. This simple local trace has no forecasting
  or trained inference and ended at actual contact before its 30-second cap.
  See `runtime-held-trace.json` for complete inputs/counts. The gate passed 17
  checks including five independent dependency trees and exact retained 44-case
  parity (`policy-gate.log`). Added instrumentation passed strict Clippy separately.

## Scope after this run

No required migration item remains in the accepted execution contract. Reward,
training backend, learned weights and flight-performance improvement belong to
the next explicitly scoped experiment. Legacy authored pack adapters and
compatibility exports are intentional application services, rather than partial
migrations. Existing report navigation/server changes remain the other writer's
uncommitted work. No flight-performance improvement or trained policy is claimed.

## F1: Policy 3 solver ownership

Policy 3 session lifecycle, nominal physical realization, local clearing,
acquisition, fixed-program terrain audit, early-exit validation and source proof
now belong to `pd-planner`. The crate composes core/runtime, pure `pd-plan` math
and maintained feedback implementations. It has no dependency on evaluation,
report rendering, mission collections or the evaluation application. The
maintained gate includes a transitive solver dependency check.

Existing application module names are compatibility exports. The opt-in
`pd-cli` feature imports solver lifecycle directly and retains the application
service for create-only bundles/replay. Default-off dependencies remain separate.
The handoff diagnostic computes through a caller-supplied recorder; the
application owns reading, hashes, destinations and create-only writes. No solver
source contains application repository/provenance or rendering dependencies.

All 62 solver tests moved with their algorithms. Retained capture parity remains
in the application because acceptance and frozen pack resolution are application
services. The injected finalization failure stays a solver test; create-only
partial capture coverage stays in application bundle/output tests. The early-exit
query bindings stay with the solver; maintained rich projection coverage stays
with application report tests. Solver test helpers read unchanged, unmodified
frozen base recipes directly; they do not depend on the application or outputs.

The first complete gate reached strict Clippy after all workspace/CLI tests
passed, then rejected redundant references introduced at the diagnostic boundary.
Those references were fixed; strict workspace Clippy and the gate's Node tests
passed. Logs are retained as `policy3-extraction-gate.log`,
`policy3-extraction-clippy.log` and `policy3-extraction-tests.log` under the run
root. The complete rerun passed all 18 checks, including exact numerical parity for
all 44 retained cases (`policy3-extraction-gate-v2.log`). All 207 pure terrain
Python tests passed. Six unchanged protected non-document paths and all 81
frozen JSON fixtures match the starting byte identities. Ballistic execution and
complete configuration remain the next separate extraction.

An intermediate direct parity invocation omitted its required environment
variable and failed before any mission execution (`policy3-extraction-parity.log`).
The gate supplies the pinned path explicitly; its accepted result is the rerun
above. A preservation probe initially included the migrated fixtures README as
a frozen input; the actual JSON input identity check passed without edits.

## F2: Ballistic execution and complete configurations

Ballistic acquisition, correction, recovery, clearance, phase transition,
coast/terminal composition and the saved-prefix capability computations now
belong to `pd-planner`. The application retains file frontdoors, provenance,
create-only artifacts and rich report adaptation. All legacy experiment names
remain selectable; ridge remains the default and cap 6 remains the default.
The v18 acquisition comparator and v19 combined-safety reference stay opt-in.
No guard, ranking, deadline, correction cap or controller tuning changed.

Public ballistic execution takes an opaque `BallisticOriginV1`, constructed
only from the original scenario plus an optional source-replayed retained
prefix. Callers cannot supply a restored snapshot/live plant. The origin binds
its request, rejects changed inputs and disallows a fresh terrain-neutral
mission. Existing diagnostic queries remain distinct from mission execution.

`TerminalPdgResolvedConfigV1` records tuning and all six constructor adapter
flags, including countdown's duration effect and coordination's braking/body
centering effects. A fresh constructor consumes that configuration and excludes
runtime plan state. Tests cover all 64 builder combinations and exact command
frames after reset. Legacy `ControllerSpec` and tuning JSON are unchanged.
`BallisticFeedbackConfigV1` records 24 effective switches, actual terminal and
standalone configurations, relevant numerical/effort limits and the existing
60-second wall guard. It validates the frozen supported profiles before execution.
The application consumes it and writes a separate `configuration-v1.json`;
no historical hashed descriptor receives additional fields.
`Policy3ResolvedConfigV1` binds the supported policy, generation and terminal
profile; the optional CLI uses its explicit configured session constructor.

The predeclared ballistic preservation panel ran before and after extraction:
13 attempts, including three procedural diagnostic inputs across ridge/v18/v19,
a cap-6 default control and three unsupported terminal inputs. All ten complete
captures and three typed rejections retain identical outcomes. All 53 physical,
proof and result files match exact bytes with **no fields excluded**. All ten
rich HTML reports also match exact bytes. Every completed capture has integrity,
source replay and reproduced-decision evidence. These inputs were already
exposed diagnostics; this is extraction preservation, not generalization evidence.
The request, raw attempts and comparisons are retained in
`ballistic-panel-request.json`, `ballistic-before/`, `ballistic-after/`,
`ballistic-preservation.json` and `ballistic-rich-preservation.json` under the run
root. Each proof capture held source and binary identities stable.

The complete maintained gate passed all 18 checks, including all 44 retained
Policy 3 comparisons (`solver-final-gate.log`). The migrated controller and solver
suites passed; strict workspace Clippy passed. Focused new configuration checks
also compare configured Policy 3 execution with the legacy path and reject
historical policies, changed effective flags, mismatched requests and invalid
origins. Early compile diagnostics from the ownership/import split and test field
names are retained; they were corrected without changing numerical inputs or
comparison rules. Final workflow consolidation and current documentation remain.

## G: Workflow consolidation and final acceptance

The ownership map and current guidance/development/evaluation instructions now
point to implementation owners. The installed native binary remains `pd-eval`;
its Cargo package is `pd-eval-cli`. The concrete solver has its own module/API
map in [pd-planner](../pd-planner/README.md). Compatibility exports contain no
algorithm copies. The generic runtime/evaluation/report path uses open agents
and opaque configuration, with no concrete-strategy dependency.

The source staging audit found two omissions: the new example workspace member
and embedded report JavaScript. The maintained source inventory now includes
both; copied-source build selectors inspect their own manifest layout rather
than assuming the current package name. A create-only 354-file staged snapshot
resolved Cargo metadata and passed an offline locked check of the built-in
application (`source-staging-check/`, `source-staging-metadata.json`,
`source-staging-build.log`). No research campaign or publication was run.
The archived departure executor remains explicitly retired; saved evidence
verification is retained. Frozen protocol/fixture inputs were not rewritten.

Final validation also exercises the independent checkpoint consumer after solver
extraction, including read-only replay of earlier sealed evidence. The demo is
kept untrained; its crash outcomes remain integration records. The current
roadmap points to a separate reward/training experiment on declared procedural
collections, with observed sets treated as regressions rather than fresh tests.

Final acceptance passed all 18 maintained development checks and the explicit
44-case exact retained comparison (`final-gate.log`), plus all 207 pure/synthetic
terrain Python tests (`final-python-tests.log`). Final independent integration
repeated 66 attempts on 64 unique inputs: 66 stable-integrity captures, 66 complete
native replays and rich reports, and two exact repeats after excluding measured
compute/performance timing (`final-independent-validation.json`). All retain
crashed/failed mission outcomes; none is labeled a verified landing. A read-only
replay of the earlier sealed capture succeeds under the current binary
(`final-old-capture-replay-v2.log`). The first replay command pointed at a
nonexistent nested capture directory and failed without reading or changing
retained evidence; its diagnostic log is preserved.

`final-preservation.json` confirms all ten initially dirty/protected paths retain
starting bytes plus explicitly declared package/path/ownership-table edits.
The shared README was staged from transformed clean HEAD content, retaining the
other writer's hunks unstaged. All 81 frozen JSON fixtures and the original
common rich template match starting byte identities. Historical outputs,
selectors, receipts and publication state were not rewritten or cleaned. No
server lifecycle, push or publication operation was performed.

| Completion condition | Accepted evidence |
| --- | --- |
| Independent generate/execute/capture/replay/rich report | Neutral integration tests, checkpoint package, final 66-attempt integration and dependency gate |
| Self-contained generation/collections without solver feedback | Seven generator/collection tests, 10k deterministic structural characterization and immutable receipt verification |
| Shared online/prepared host, reset/step and summary equivalence | Native-boundary parity tests, captured/summary episode tests and sustained held-command measurement |
| Solver ownership and complete configuration | Policy 3/ballistic extraction, configured constructors, 64 terminal builder combinations and solver dependency gate |
| Existing workflows/serde/proof/report preservation | Full maintained gate, retained 44-case numerical parity, exact 13-attempt ballistic panel and ten identical rich reports |
| Current docs and an explained checkout | Ownership/workflow docs, this run record, protected-file checks and scoped milestone commits |

The completion check concerns the usable platform and preservation scope above.
Generator coverage is input characterization. The policy checkpoint is an
untrained consumer. Summary outcomes support training but do not carry saved
command landing proof. Sustained trace timing describes this local synthetic
workload and is not a trained-policy throughput prediction.
