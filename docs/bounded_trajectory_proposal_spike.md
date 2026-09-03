# W3 finite bounded-trajectory proposal spike

## Status and decision

W3 selects a finite, serial, predeclared command-template search as the first
untrusted proposal backend for `bounded_trajectory_witness_v1`. The spike is
implemented in `pd-eval` without a new dependency. It exercises deterministic
proposal configuration, exact attempt budgets, reconstruction, abstention, and
artifact validation; it is not a capability result or a planner/controller
integration candidate.

The alternative sequential-convex direction remains technically plausible.
[Clarabel.rs](https://github.com/oxfordcontrol/Clarabel.rs) provides a pure-Rust
interior-point implementation for quadratic and conic programs, including QP
and SOCP forms. Adopting it would still require a separately reviewed dynamics
linearization, convex contact and handoff approximations, trust-region and warm
start rules, solver tolerances, and exact command reconstruction. Its solver
status could propose a trace or cause `unknown`; it could never replace the W1
exact verifier or create a negative physical certificate.

The finite backend is the narrower W3 choice because the repository already
has everything needed to evaluate it reproducibly. It adds no numerical solver
or model whose approximation boundary would have to be designed before the
proposal plumbing can be tested.

## Locked spike configuration

`finite_template_proposal_configuration_v1` seals the existing
`BoundedTrajectoryProposalConfigurationV1` with:

| Field | W3 value |
| --- | --- |
| Engine | `bounded_finite_template_search_v1` |
| Parameterization | `constant_command_catalog_7_probe_5s_v1` |
| Numerical settings | `serial_rust_f64_exact_order_v1` |
| Reconstruction | `truncate_at_exact_first_final_handoff_v1` |
| Determinism | `serial_template_then_horizon_v1` |
| Search budget | Caller-selected positive exact-verification count, at most `189` |

The maximum follows from seven templates, twenty-six five-second probes through
the fixed `130 s` W1 horizon, and at most one reconstructed terminal attempt per
template. Every exact verification, including reconstruction, consumes one
unit. The constructor and executor reject configuration IDs or budgets outside
this locked backend rather than silently accepting a self-consistent variant.

The serial template order is:

1. upright, full throttle;
2. 15-degree outbound tilt, full throttle;
3. 15-degree inbound tilt, full throttle;
4. 30-degree outbound tilt, full throttle;
5. 30-degree inbound tilt, full throttle;
6. upright, three-quarter throttle; and
7. upright, half throttle.

Outbound/inbound angles use only the physical input's `horizontal_sign`.
Templates do not read provenance, route-family labels, controller state,
planner rank, prior evidence, or executor outcomes.

## Search and exact-replay boundary

For each template, the spike constructs canonical witnesses at `600`-step
intervals and submits each one to `verify_bounded_trajectory`. The first exact
`verified` result wins. When exact replay reports that a candidate continued
past its first final handoff, the spike may construct one shorter witness at
the verifier-reported terminal physics step; that reconstruction is itself a
new budgeted exact-verification attempt.

The spike never treats an edited or reconstructed trace as already verified;
every shorter reconstruction receives its own exact verification. A contact,
missed boundary, or rejected reconstruction advances the finite search. A
direct route or waypoint count outside V1 scope returns `unknown/scope/*`
without running a template. Reaching the configured attempt limit or the end of
the catalog returns `unknown/coverage/bounded_search_exhausted`. Neither result
means physical infeasibility.

`FiniteTemplateProposalSpikeResultV1` retains the sealed proposal
configuration, every witness and exact verification in attempt order, the
optional selected attempt, the resulting physical prediction, and a canonical
result digest. Cheap validation rejects invalid/mixed attempt joins, budget
overruns, inconsistent decisions, and digest changes. Exact validation reruns
the complete deterministic search and compares the reconstructed result.

## Spike result and limitations

The synthetic W2 one-waypoint case is found in three attempts: a five-second
probe, a nonminimal ten-second probe, and exact reconstruction at step `960`.
The two-waypoint case is found in four attempts with reconstruction at step
`1,560`. Repeated runs reproduce equal artifacts and bytes. A one-attempt
budget returns the expected coverage `unknown`, and a direct route abstains
without an attempt.

These results validate proposal orchestration, not useful development-corpus
coverage. Constant commands cannot express a general multi-phase boost, coast,
brake, or handoff strategy. W3 therefore freezes this backend as a deliberately
small baseline for W4; it does not establish that finite templates are the
eventual production proposal method, and it does not reject sequential-convex
or other separately versioned future engines.

No maintained evidence corpus was run or changed. No R1 actions or outcomes
were opened by the proposal engine, and no planner ranking, controller,
runtime screen, dependency, or D2 artifact changed.

## Next gate

W4 may run this already-frozen input-only proposal on the already-seen
development inputs, seal its physical `supported`/`unknown` results before
opening the corresponding R1 frozen-executor artifacts, and then build the
two-axis comparison. W4 must report low coverage honestly and may not retune
this W3 configuration after seeing executor outcomes.
