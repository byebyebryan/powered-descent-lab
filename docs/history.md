# Development history and archive

[Documentation home](README.md) · [Current guidance](guidance.md) · [Current roadmap](roadmap.md#7-recommended-immediate-next-step)

These documents are dated records, not an executable backlog. Their local
“current”, “next”, accepted counts and stopped experiments refer to their own
checkpoint. V1 search, policy-1/2 execution and superseded research frontdoors
are retired. Keep original fixtures, captures, selectors and receipts intact;
saved evidence and historical decoding do not require the old executables.

For today's tooling use [evaluation](evaluation.md) and [report operations](reports.md).
Links to generated `outputs/` evidence require the local retained artifacts;
those files are deliberately absent from a fresh checkout. Retired source links
use immutable Git revisions where available; they are not current module owners.

## Selected milestones

These records explain how the current design was reached. Their dated "next"
steps and earlier failures are not the current V2 backlog; start with Guidance
Architecture and the accepted session/CLI results for today's boundary.
Their superseded executable research commands and generators have been retired;
the documents, frozen inputs, captures and published pages remain historical evidence.

- [Waypoint direct-route characterization](waypoint_direct_characterization.md)
  records the opt-in continuous-terrain mismatch between V1's exact direct
  chord and the unchanged direct controller; it does not select or implement a
  replacement leg-profile model.
- [Direct-leg primitive research](waypoint_direct_primitive_research.md)
  records the fixed 90 s analytical baseline, 24-cell obstacle sweep, and
  unchanged-controller comparison. It locates a tracking gap on taller terrain
  without demonstrating a need for an operational waypoint.
- [Complete flat direct acceptance](waypoint_direct_complete_flat_acceptance_protocol.md)
  records the known-flat milestone: four complete launch-aware, held-60 Hz
  direct witnesses land on uncut flat terrain. It accepts before ranking and
  does not change planner defaults or establish held-out coverage.
- [Input-driven direct generation](waypoint_direct_generation_results.md)
  records six uncut flat/uphill/downhill cases; the subsequent
  [obstacle discrimination](waypoint_direct_obstacle_discrimination_results.md)
  separates blocked chosen arcs, accepted alternative direct arcs, and finite
  unknowns. The [terminal diagnostic](waypoint_direct_terminal_admissibility_results.md)
  separates inadmissible ideal contact references from cadence-sensitive
  execution. The latest [body-aware terminal prototype](waypoint_direct_body_aware_terminal_results.md)
  accepts all fourteen development and ten newly sealed direct-flight cases
  at held 60 Hz, without cutaways, waypoints or planner/controller/default changes.
- [Nominal direct flight integration](nominal_direct_flight_integration_results.md)
  adds an opt-in mission-input CLI and complete timed-program executor through
  the ordinary controller/simulator path. All 24 now-exposed checkpoint cases
  have exact parity in two release runs. Defaults remain unchanged; robustness,
  useful setup cost and waypoint composition remain separate gates.
- [Nominal direct operational execution](nominal_direct_operational_execution_results.md)
  implements strict saved-command coverage with separate actual outcomes,
  exact nominal comparison and truthful partial-run/replay evidence. All 24
  exposed controls and four presealed uncut flat/uphill/downhill missions are
  Direct, completed-safe and Match in two final-source runs. No continuation,
  robustness claim or default planner/controller change is introduced.
- [One-update terminal completion design](nominal_direct_terminal_completion_reserve_contract.md)
  specifies a separately versioned completion reserve and explicit counterfactual
  segment/replay boundary. Four new physical inputs are input-only sealed; the
  reserve is not implemented or flown, and current execution/defaults are unchanged.
- [Waypoint planner V2 integration](waypoint_v2_airborne_integration_results.md)
  and its [acceptance review](waypoint_v2_airborne_integration_acceptance.md)
  record the first usable supported opt-in checkpoint: terrain-blind direct
  construction, local clearing and replanning from actual handoff state.
  Policy 3 passes eight uncut clear controls and sixteen ordinary terrain
  missions. The [fresh-terrain readiness pass](waypoint_v2_fresh_terrain_readiness_results.md)
  also lands three new direct clear controls and nine new obstructed missions
  in identical repeats, supporting bounded offline use on the tested setup.
  Remaining diagnostics are recorded limits. V2 policy 3 is now the default
  planner **evaluation** workflow described in [Evaluation](evaluation.md); ordinary `pd-cli` controller
  integration remains separate.
- [Conservative Ballistic Route Planning](conservative_ballistic_route_planning.md)
  owns the game-oriented analytical contingency, its completed CB0 gameplay
  audit, the V2 direct-ballistic bridge certificate, and the bounded ridge
  canary. The canary establishes a scoped flat-control-green, committed-
  nominal-direct-red, terrain-derived-one-waypoint-green result while retaining
  higher direct arcs as global-replan diagnostics. Its completed F6 opt-in
  integration lane also demonstrates the typed analytical-decision to real-
  controller route seam for the retained `ridge_progress_056_probe` mission;
  this remains a one-mission development capability, not production planner
  wiring or mission-family coverage.
- [Bounded Trajectory Witness V1](bounded_trajectory_witness.md) owns the
  locked post-R1 controller-neutral exact-witness contract and its implemented
  W1-W4 gates.
- [W3 Finite Proposal Spike](bounded_trajectory_proposal_spike.md) owns the
  fixed untrusted command-template backend and its exact-search budget;
  proposal exhaustion remains `unknown`.
- [W4 Physical/Executor Comparison](physical_executor_comparison.md) owns
  the outcome-isolated already-seen result and its strict selected-route join.
- [Progress](progress.md) is append-only checkpoint history; older results
  there are not current claims.
- [Early Design Scratchpad](early_design.md) is retained exploratory
  history and may contain superseded directions.

## Archive catalogue

This catalogue preserves discovery without moving the original records. Completed
plans and protocols are records of the work, not instructions to rerun retired
commands. Current architecture, guidance, suite designs, roadmap and accepted
session/cleanup results are listed separately on the documentation home.

### Earlier direct-flight design and execution

- [Ballistic-direct-first planning decision protocol](ballistic_direct_first_decision_protocol.md)
- [Canonical initial direct transfer protocol](canonical_initial_direct_canary_protocol.md)
- [Canonical initial direct transfer results](canonical_initial_direct_canary_results.md)
- [Nominal airborne direct regeneration canary protocol V1](nominal_airborne_direct_canary_protocol.md)
- [Nominal airborne direct regeneration results](nominal_airborne_direct_canary_results.md)
- [Nominal direct first-contact and command-coverage study V1](nominal_direct_contact_phase_protocol.md)
- [Nominal direct first-contact and command-coverage results](nominal_direct_contact_phase_results.md)
- [Nominal direct operational completion and saved-command coverage V1](nominal_direct_execution_completion_contract.md)
- [Nominal ballistic-direct flight integration V1](nominal_direct_flight_integration_protocol.md)
- [Nominal ballistic-direct flight integration results](nominal_direct_flight_integration_results.md)
- [Strict saved-coverage nominal direct execution V1](nominal_direct_operational_execution_protocol.md)
- [Strict saved-coverage nominal direct execution results](nominal_direct_operational_execution_results.md)
- [One-update nominal direct terminal-completion reserve V1](nominal_direct_terminal_completion_reserve_contract.md)
- [One-update terminal-completion reserve implementation protocol V1](nominal_direct_terminal_completion_reserve_protocol.md)
- [Source-departure / acquisition execution contract](source_departure_execution_contract.md)

### Uncut direct-route research

- [Body-aware nominal direct terminal prototype](waypoint_direct_body_aware_terminal_protocol.md)
- [Body-aware nominal direct terminal results](waypoint_direct_body_aware_terminal_results.md)
- [Waypoint direct-route characterization](waypoint_direct_characterization.md)
- [Complete flat direct-witness acceptance and selection](waypoint_direct_complete_flat_acceptance_protocol.md)
- [Direct-leg first-contact contract audit protocol](waypoint_direct_contact_contract_protocol.md)
- [Flat direct-leg coupled-thrust audit protocol](waypoint_direct_coupled_thrust_audit_protocol.md)
- [Direct-leg flat third-candidate closure canary](waypoint_direct_flat_candidate_closure_protocol.md)
- [Input-driven nominal ballistic-direct generation](waypoint_direct_generation_protocol.md)
- [Input-driven nominal ballistic-direct generation results](waypoint_direct_generation_results.md)
- [Held-cadence diagnostic for the frozen source-duration canary](waypoint_direct_held_cadence_diagnostic_protocol.md)
- [Direct-leg launch-feasibility canary protocol](waypoint_direct_launch_feasibility_protocol.md)
- [Direct-leg nominal-to-plant characterization protocol](waypoint_direct_nominal_plant_protocol.md)
- [Frozen direct-generator obstacle discrimination](waypoint_direct_obstacle_discrimination_protocol.md)
- [Frozen direct-generator obstacle discrimination results](waypoint_direct_obstacle_discrimination_results.md)
- [Paired-command feasibility for the frozen source-duration canary](waypoint_direct_paired_command_feasibility_protocol.md)
- [Direct-leg primitive research protocol](waypoint_direct_primitive_research.md)
- [Direct-leg source-contact diagnostic protocol](waypoint_direct_source_contact_protocol.md)
- [Flat direct-leg source-duration canary protocol](waypoint_direct_source_duration_canary_protocol.md)
- [Frozen terminal-admissibility and execution-isolation protocol](waypoint_direct_terminal_admissibility_protocol.md)
- [Frozen terminal-admissibility and execution-isolation results](waypoint_direct_terminal_admissibility_results.md)

### V2 development, acceptance and presentation history

- [Planner V2 reliability results](planner_v2_reliability_results.md)
- [Report navigation and waypoint annotations plan](report_navigation_and_waypoint_annotations_plan.md)
- [Report navigation and rich waypoint preview results](report_navigation_and_waypoint_annotations_results.md)
- [Waypoint V2 airborne integration acceptance review](waypoint_v2_airborne_integration_acceptance.md)
- [Waypoint V2 airborne integration plan](waypoint_v2_airborne_integration_plan.md)
- [Waypoint V2 airborne integration results](waypoint_v2_airborne_integration_results.md)
- [Waypoint V2 airborne integration planning review](waypoint_v2_airborne_integration_review.md)
- [Planner V2 batch-tree presentation checkpoint](waypoint_v2_batch_tree_preview_results.md)
- [Planner V2 core loop cleanup plan](waypoint_v2_core_cleanup_plan.md)
- [Planner V2 evaluation activation](waypoint_v2_eval_activation_plan.md)
- [Planner V2 evaluation activation results](waypoint_v2_eval_activation_results.md)
- [Waypoint V2 fresh terrain readiness plan](waypoint_v2_fresh_terrain_readiness_plan.md)
- [Waypoint V2 fresh terrain readiness results](waypoint_v2_fresh_terrain_readiness_results.md)
- [Waypoint V2 goal amendment](waypoint_v2_goal_amendment.md)
- [Waypoint V2 ground diagnostic plan](waypoint_v2_ground_diagnostic_plan.md)
- [Waypoint V2 ground diagnostic results](waypoint_v2_ground_diagnostic_results.md)
- [Waypoint V2 ground diagnostic primary review](waypoint_v2_ground_diagnostic_review.md)
- [Waypoint V2 nominal characterization protocol](waypoint_v2_nominal_characterization_protocol.md)
- [Unified nominal characterization results](waypoint_v2_nominal_characterization_results.md)
- [Unified nominal characterization primary review](waypoint_v2_nominal_characterization_review.md)
- [Practical waypoint planner V2 plan](waypoint_v2_practical_plan.md)
- [Practical waypoint V2 design review](waypoint_v2_practical_plan_review.md)
- [Waypoint V2 initial entry revision](waypoint_v2_practical_policy_revision.md)
- [Practical waypoint V2 implementation results](waypoint_v2_practical_results.md)
- [Waypoint V2 report presentation plan](waypoint_v2_report_presentation_plan.md)
- [Waypoint V2 report presentation results](waypoint_v2_report_presentation_results.md)
- [Planner V2 session and CLI integration plan](waypoint_v2_session_integration_plan.md)
- [Planner V2 session and CLI integration results](waypoint_v2_session_integration_results.md)
- [Waypoint V2 terminal time construction plan](waypoint_v2_terminal_time_plan.md)
- [Waypoint V2 terminal time construction results](waypoint_v2_terminal_time_results.md)
- [Waypoint V2 terminal time primary review](waypoint_v2_terminal_time_review.md)
- [Waypoint V2 unified nominal design and plan](waypoint_v2_unified_nominal_plan.md)

### Alternative planners and local-clearing research

- [W3 finite bounded-trajectory proposal spike](bounded_trajectory_proposal_spike.md)
- [Bounded trajectory witness V1](bounded_trajectory_witness.md)
- [Conservative Ballistic Route Planning](conservative_ballistic_route_planning.md)
- [Local clearing canary plan](local_clearing_canary_plan.md)
- [Local clearing canary protocol](local_clearing_canary_protocol.md)
- [One obstruction local clearing results](local_clearing_canary_results.md)
- [W4 physical/executor development comparison](physical_executor_comparison.md)
- [Trajectory-tube shadow spike](trajectory_tube_spike.md)

### Project and checkpoint history

- [Project Design Notes](early_design.md)
- [Progress](progress.md)
