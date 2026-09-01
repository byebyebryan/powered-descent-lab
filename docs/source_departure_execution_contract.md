# Source-departure / acquisition execution contract

## Status and scope

This remains an evidence-first checkpoint, not a production feasibility model.
D0a and D0b are implemented. The versioned input-only development corpus,
neutral point-envelope terrain query, physics-rate source-transition extractor,
route-execution assembler, and sibling fidelity gates now cover the
authoritative source transition and retained route prefix through the final
emitted waypoint handoff. The D1 research contract is design-closed below.
D1a's common capability, prediction, fold, comparison, and gate artifacts and
D1b's fixed Alternative A prototype are implemented. Alternative A
deterministically misses its development gate and is not accepted. An
input-only terrain-equivalence spike also rejects the tested route-relative
relief motif as an A2 design basis; it changes no capability or prediction.
D1c, D1d, every held-out comparison, and every planner-facing phase from D2
onward remain unimplemented. D0a/D0b/D1a/D1b and the spike change no planner
generation or ranking, controller behavior, or maintained fixture semantics.
Simulation remains authoritative for flight outcomes, actual hull clearance,
fuel, handoff success, and landing.

The capability label is deliberately narrower than a mission result. It covers
planner-generated waypoint topology from the initial source-pad state through
the final emitted waypoint handoff. Direct routes and target landing or
terminal descent are outside this capability label. For a scoped post-run
outcome, terminal contact/crash before the final handoff, timeout, or a failed
ordered/handoff contract before the final handoff is a scoped failure. A scoped
pass requires every required handoff to succeed and the checkpoint to terminate
with `MissionOutcome::Success`, `EndReason::CheckpointSatisfied`, and
`PhysicalOutcome::Flying`. Clearance and fuel remain reported physical
evidence, not acceptance predicates, unless a later frozen capability explicitly
sets thresholds.

The rejected center-only quintic witness and scalar reserve/profile retunes are
excluded. They are not to be tuned into a production screen. The existing
waypoint-planning boundary remains authoritative for route geometry and bounded
search; see [Waypoint Planning V1](waypoint_planning.md) and the archival
[Trajectory-tube shadow spike](trajectory_tube_spike.md).

## Prediction, observation, and outcome are separate

`SourceTransitionEvidence` is a post-run, observational `pd-eval` artifact. It
is extracted from raw simulation bundles and records what the run supplied at
the source transition. It is not a pre-run feasibility claim.

The extraction boundary has two stages. The pure crossing kernel accepts only
immutable resolved world/vehicle/initial-state context, the resolved profile
and route geometry, physics-rate raw samples, and an action-audit input. It
derives boundary crossings, physical series, extrema, path-relative values,
backtracking/re-entry, and audit attachments without reading mission outcome,
terminal metadata, controller identity, or labels. Its emitted physical
features exclude those values as well. The post-run evidence assembler consumes
the kernel result and raw-bundle provenance. It may read only terminal
physics-step/reason metadata from the manifest/events to classify a sound
missing-boundary prefix as `censored_before_*` rather than `invalid`; it stores
that metadata as status/audit only and never feeds it back into crossing math
or any pre-run prediction feature.

Scoped pass/fail labeling belongs exclusively to a separate outcome/comparison
artifact. `SourceTransitionEvidence` carries physical evidence and status, not
the scoped outcome label.

A future `SourceDepartureCapability` is predictive. Its inputs are only the
immutable resolved world, vehicle, initial state, plan or candidate geometry,
and a versioned capability. It predicts an admissible tracking-entry state set
and guarantees for the complete initial-state-to-tracking-entry interval. That
interval has a pad-departure phase from the authoritative initial state through
`contact_exit`, followed by an acquisition phase from `contact_exit` through
`tracking_entry`. Observed transition states, controller telemetry, outcomes,
labels, and run IDs never enter that pre-run prediction.

A future `RouteExecutionPrediction` is tri-state:

- `supported`: the versioned capability predicts the scoped waypoint topology
  is supported;
- `unsupported`: it predicts that the scoped topology is not supported; and
- `unknown`: it cannot make a bounded claim from the available physical input.

Each state carries stable reason codes and separate input/capability digests.
`unknown` is an abstention, not a failure or a true negative. A post-run
comparison may join a prediction to an outcome, but that join never changes the
pre-run input boundary.

The route prediction is an ordered composition, not an independent label over
waypoints:

1. The source-departure result must cover `initial_state -> contact_exit ->
   tracking_entry`. Its pad-departure terminal set must be an admissible input
   to acquisition, and its acquisition terminal set must be an admissible input
   to the first outbound leg.
2. Each subsequent route-leg terminal set must be contained in the declared
   admissible input set for its waypoint handoff and next leg. Independent
   waypoint checks do not establish this composition.
3. Evaluate in composition order and stop the authoritative support proof at
   the first phase or containment that is not `supported`. The composed result
   is `supported` only when every required decision is supported;
   `unsupported` only when that first decisive boundary has an explicit
   negative certificate; otherwise it is `unknown`. Later phase calculations
   may be retained as diagnostics but do not change the composed decision.

Stable reasons identify the phase (`pad_departure`, `acquisition`, `route_leg`,
or `handoff`), the waypoint/leg index when present, and the first boundary or
set-containment decision that prevents support. Later phases may be retained as
diagnostic detail, but they cannot overwrite the first decisive reason.

## The initial-state source transition and its two boundaries

The source transition is the complete interval from the authoritative resolved
initial state on the source pad through entry into the planner's full corridor
envelope. The initial state is an anchor, not a synthetic crossing. Two fixed
geometry-derived boundaries divide the transition; there is no single
full-envelope anchor and no omitted liftoff prefix.

1. `contact_exit` is the first directed source-to-target crossing of
   `SafetyProfile.source_transition_start_m`. This is the planner's
   geometry-derived contact-envelope exit boundary and is policy-independent:
   the canonical contact footprint has cleared the source pad. It does not by
   itself prove that a rotated physical vehicle had no contact; actual contact
   and clearance remain separate observations from raw touchdown/hull state.
2. `tracking_entry` is the first directed source-to-target crossing of
   `SafetyProfile.source_transition_end_m`. The policy-defined source taper has
   reached the planner's full corridor envelope at this boundary.

`tracking_entry` is not purely physical and is not merely the vehicle envelope:
it depends on the resolved `endpoint_transition_m` and
`flight_clearance_margin_m` policy/geometry. The current profile construction,
including contact-footprint expansion, taper, and full-envelope extent, is in
[`pd-core/src/planning.rs`](../pd-core/src/planning.rs) via
`build_endpoint_profile` and `SafetyProfile`.

The source-departure capability must cover and certify both
`initial_state -> contact_exit` pad departure and `contact_exit ->
tracking_entry` acquisition, then deliver an admissible terminal state set at
`tracking_entry`. It must preserve a reachable terminal set at `contact_exit`
so the two phases compose. It must not certify only a point sample at either
boundary, omit the initial source-contact prefix, or independently screen later
waypoints.

### Normalized progress and crossing rule

For a resolved request, derive source-relative progress from the source pad
center and the planner's `horizontal_sign`:

```text
progress_i_m = horizontal_sign *
               (observation_i.position_m.x - source_pad.center_x_m)
```

Read raw `SampleRecord`s in recorded chronological order. The first adjacent
pair satisfying
`progress_i_m < boundary_m <= progress_(i+1)_m` is the directed crossing for
that boundary. Apply the rule independently to `source_transition_start_m`
and `source_transition_end_m`; do not substitute a route label, route angle,
controller phase, handoff threshold, or controller marker. The source-to-target
direction comes only from the normalized geometry.

For each boundary, preserve both raw records, sample indices, physics steps,
times, progress values, and the bracket fraction. A plane-interpolated time and
state may be calculated for diagnostic comparison, but it is derived evidence,
not a state that occurred in the simulation. Use the shortest angular
displacement for an interpolated attitude and retain the raw attitudes.

### Evidence status and right censoring

Transition evidence has an explicit status independent of the scoped execution
outcome:

- `complete`: both boundaries have sound physics-rate brackets and all required
  evidence is finite, ordered, and auditable;
- `censored_before_contact_exit`: the run terminates before `contact_exit`, but
  the available raw prefix, cadence, ordering, and provenance are sound;
- `censored_before_tracking_entry`: `contact_exit` is established, then the run
  terminates before `tracking_entry`, with a sound available raw prefix; and
- `invalid`: the raw input is malformed, missing, nonfinite, out of order,
  wrong-cadence, or otherwise unverifiable.

Terminal-before-boundary is valid right-censored transition evidence, not an
invalid record. Preserve every available prefix state and extrema plus the
terminal physics step and terminal reason for either censored status. A
terminal-before-boundary run also has a scoped execution failure outcome. A
`supported` prediction joined with either censored terminal failure is a false
accept. A missing crossing in a run that does not provide a sound terminal
censoring point, or a missing/unverifiable bracket, is `invalid` instead.

Only `invalid` contributes to the invalidation count. Invalid records retain
their reason and available provenance; no status may be silently omitted from
coverage or treated as an infeasible physical result.

If a run reaches `tracking_entry` with valid evidence and then has terminal
contact/crash, times out, or fails a required handoff before the final handoff,
its scoped outcome is still a failure. If it terminates before a required
boundary, preserve the right-censored evidence status and terminal reason
separately from the scoped failure; do not call that evidence invalid.

After each boundary, scan the complete raw progress series and record every
backtracking or re-entry event, including the sample/bracket context and the
boundary crossed in reverse. Record interval extrema; never move the boundary,
select a later crossing, or hide nonmonotonicity behind a controller phase.

### Fidelity and raw authority

The evidence pack must resolve `sample_hz == physics_hz`; a missing or different
cadence is `invalid`, not a censoring status. The adjacent raw
physics-step states before and after each boundary are authoritative. There is
no interpolation across missing records and no lower-rate sample is upgraded
to physics-rate evidence. A plane-interpolated state is derived diagnostic data
only and must carry its bracket width and uncertainty context (at minimum the
time/progress span, interpolation fraction, and raw endpoint identities).

Preserve an action-log digest and any held-command or controller-update changes
inside either bracket as audit provenance. These values explain the bracket but
are not pure planner features. The first extractor slice needs no new hot-path
or `pd-core` event; it consumes the existing raw artifacts.

### Shaped-centerline reference

The exact route reference is `RoutePlan.diagnostics.selected_centerline_m`.
Persist the shaped-centerline point/segment indices, world coordinates, and
normalized source-relative frame that contain both boundaries. Persist the
exact first outbound-leg segment/reference from `tracking_entry` toward the
first emitted waypoint, including its unit tangent and route-leg identity.
Do not reconstruct a nominal chord, invent a quintic, or replace the shaped
centerline with controller guidance telemetry.

### Canonical point-envelope clearance

The extractor must not reproduce terrain-heightfield or envelope logic in
`pd-eval`. D0a may add one neutral, read-only `pd-core` query equivalent to:

```rust
TerrainDefinition::exact_point_clearance(
    center_m: Vec2,
    envelope: CorridorEnvelope,
) -> Result<CorridorClearance, TerrainQueryError>
```

It returns the exact minimum clearance between an axis-aligned point-centered
`CorridorEnvelope` and the piecewise-linear terrain over the envelope's lateral
span, including the worst terrain residual and segment identity. It accepts a
single point and therefore covers vertical or zero-horizontal-motion samples;
it is not implemented by perturbing the point or by calling
`exact_corridor_clearance` with an artificial nonzero span. `pd-core` owns the
query and its edge/domain validation. D0a adds no event and no simulation hot
path; `pd-eval` calls the query after the run.

For every retained physics-rate source-transition sample, record these distinct
metrics without substituting one for another:

- observed rotated-hull clearance from `Observation.min_hull_clearance_m`;
- observed touchdown clearance from `Observation.touchdown_clearance_m` where
  applicable;
- derived point clearance using `SafetyProfile.contact_envelope`;
- derived point clearance using the resolved `SafetyProfile::envelope_at`
  result; and
- derived point clearance using `SafetyProfile.full_envelope`, including before
  the taper has reached it.

The transition record must include the authoritative initial state, both
boundary states, separate pad-departure and acquisition extrema,
backtracking/re-entry, the clearance series above, and the exact
shaped-centerline segment/reference.
The source-departure capability consumes the resulting terminal state set at
`tracking_entry`, not a state inferred from a controller phase.

## `SourceTransitionEvidence` artifact

The first implementation should produce a neutral serialized
`SourceTransitionEvidence` artifact and an audit/provenance sidecar. These
names describe the design contract; no public schema exists yet. Its identity
is independent of any capability/configuration or prediction digest; those
digests join the evidence only in the final comparison artifact.

| Group | Required contents |
| --- | --- |
| Identity and provenance | Evidence schema/extractor version and digest; immutable request, policy, and route-plan identity; source/target geometry identity; raw bundle, `samples.json`, and action-log digests; opaque post-run artifact identity; and profile/boundary coordinates. These are artifact metadata, not model features. Capability/configuration and prediction digests are deliberately absent and join only in the final comparison artifact. |
| Initial anchor and boundary brackets | Authoritative initial raw sample identity/state plus separate `contact_exit` and `tracking_entry` raw sample indices, physics steps, times, source-relative progress values, bracket widths, progress spans, interpolation fractions, and endpoint identities when each boundary is reached; preserve an absent bracket for a right-censored boundary. |
| Boundary physical state | Raw pre/post physics-step position, velocity, attitude, angular rate, mass, and fuel for each reached boundary; observed phase-terminal summaries kept separate from any predictive reachable set; and any plane-interpolated time/state marked derived diagnostic with uncertainty context. |
| Transition series | Physics-rate raw samples spanning the available prefix from the initial source-pad state through contact exit/tracking entry; separate observed extrema for pad departure and acquisition covering progress, velocity decomposition, attitude/rate, mass/fuel, path error, and clearance; and all backtracking/re-entry events. Preserve sample indices and observed-versus-derived provenance. |
| Progress and path error | Source-relative progress, normalized source-to-target progress, first-outbound-leg along-track and cross/path-relative error, and the normalized frame used for every value. |
| Clearance | Raw `Observation.min_hull_clearance_m` and `touchdown_clearance_m` where present; exact point clearance for `SafetyProfile.contact_envelope`, the resolved `SafetyProfile::envelope_at` result, and `SafetyProfile.full_envelope`, evaluated by the canonical `pd-core` point query. Preserve units, envelope inputs, terrain residual identity, and whether each value is observed or derived. |
| First outbound-leg reference | Exact point/segment indices and coordinates from `RoutePlan.diagnostics.selected_centerline_m`; route-leg identity; normalized start/end; outbound unit tangent; and the geometry version used for path-relative error. |
| Audit fidelity | `sample_hz` and `physics_hz`, raw sample ordering, bracket width/uncertainty, action-log digest, held-command values, controller-update changes inside brackets, and any replay/cadence parity result. Audit fields never become pure evaluator features. |
| Status and comparison join keys | Explicit `complete`, `censored_before_contact_exit`, `censored_before_tracking_entry`, or `invalid` status; stable invalid reason only for malformed/missing/nonfinite/out-of-order/wrong-cadence/unverifiable input; terminal physics step/reason and available prefix extrema/state for censoring; and opaque evidence/raw-bundle keys for a later comparison join. Scoped outcome labels and provenance, prediction state/digest, and capability/configuration digests remain outside this artifact in the final comparison artifact. |

## `RouteExecutionEvidence` artifact

D1 needs neutral route-wide observations in addition to the nested source
transition. Before either alternative is fitted, D0b produces a serialized
`RouteExecutionEvidence` artifact from the same physics-rate bundle. It covers
the authoritative initial state through the final emitted waypoint boundary or
the sound terminal prefix. It is observational evidence, not a capability,
prediction, controller-success label, or mission-outcome label.

A waypoint handoff is a contract opportunity region, not a synthetic point
boundary. Derive its geometry only from the immutable `TransferWaypointSpec`:
record the first directed entry into the capture-radius window and the first
directed crossing of the deadline plane, preserving both adjacent raw brackets
when reached. For every physics-rate sample from the first opportunity through
deadline/terminal, derive `WaypointHandoffKinematics` and evaluate the canonical
`TransferWaypointSpec::assess_handoff`. The extractor may derive the sticky
`window_seen` bit only from earlier raw capture-window entries and may use it
with `contract_pass_in_window`; it must not read controller phase or state.
Compute the kinematics with the resolved source/previous-waypoint anchor,
waypoint position, next target, and contracted handoff tangent, matching the
neutral formulas already used by `pd-eval::review::waypoint_sample_stats`; do
not import a `pd-control` guidance frame.

The capture-entry bracket is the first adjacent pair satisfying
`distance_i > capture_radius >= distance_(i+1)`; an authoritative first sample
already inside the radius is recorded as `initially_inside` rather than given a
synthetic bracket. The deadline bracket is the first adjacent pair satisfying
`plane_progress_i < 0 <= plane_progress_(i+1)`. Raw samples with
`distance <= capture_radius` set `window_seen` from that sample onward. Preserve
later window exits/re-entries and reverse plane crossings, but never replace the
first brackets.

Preserve the first authoritative raw sample that satisfies the neutral contract,
if one exists, plus the entire opportunity series. A plane- or window-
interpolated state is diagnostic only. A controller handoff marker may be
attached in an audit sidecar, but marker presence, controller phase, and
controller identity do not define the neutral opportunity, contract result, or
predictor features. For capability composition, a leg's predicted terminal set
must remain inside the waypoint's declared contract-pass set before or at its
deadline and inside the next leg's admissible input set; merely crossing the
deadline plane is not support.

Map samples to the persisted shaped centerline deterministically by normalized
source-relative horizontal progress, whose selected-centerline points are
strictly ordered for this planner scope. Use half-open segment intervals, assign
an exact interior vertex to its outbound segment, and assign the final scoped
vertex to its incoming segment. Preserve before-source, after-scope, and
nonmonotone/re-entry samples explicitly; never choose a segment from controller
phase or a nearest-segment tie.

The artifact contains:

- the complete `SourceTransitionEvidence` record and its digest;
- every available physics-rate raw sample mapped to the exact shaped-centerline
  segment, normalized route progress, along/cross-path error, full physical
  state, and the same observed/derived clearance metrics;
- per-leg sample ranges and extrema, with the leg's declared admissible input
  and terminal boundary identities;
- per-waypoint capture-window-entry and deadline-plane brackets, the complete
  neutral contract-opportunity series, first contract-satisfying raw state when
  present, immutable contract measurements, and backtracking or
  reverse/repeated crossings;
- explicit `complete`, `censored_before_waypoint_<index>`, or `invalid`
  status, using the same sound-prefix versus malformed-input distinction as the
  source evidence; and
- raw-bundle, route-plan, extractor, source-evidence, and route-evidence
  identities/digests, kept separate from capability, prediction, and outcome
  digests.

`complete` means the raw evidence contains a resolved neutral opportunity for
every required waypoint: a contract-satisfying raw state or a reached deadline
with no such state. It does not assert that the controller accepted a handoff or
that the mission passed. The separate outcome/comparison artifact remains
authoritative for ordered/handoff success and scoped pass/fail labeling. D1 may
use route-wide observations as calibration or validation targets, never as
inputs to a pre-run prediction for the same run.

No physical acceptance threshold is selected here. Clearance and fuel are
reported evidence until an accepted, versioned capability defines a threshold
and the threshold is frozen before held-out outcome reveal.

## Existing evidence and provenance boundary

The current raw simulation path provides most of the first extractor input:

- [`Observation`, `SampleRecord`, and `RunArtifacts`](../pd-core/src/model.rs)
  carry position, velocity, attitude, angular rate, mass, fuel, and per-sample
  observations. [`pd-core/src/sim.rs`](../pd-core/src/sim.rs) writes samples at
  the configured cadence, so the evidence pack must resolve the physics-rate
  cadence explicitly.
- [`ControlledRunArtifacts`](../pd-control/src/lib.rs) preserves the core run
  and controller updates. Waypoint handoff markers and metrics from
  [`pd-control/src/transfer/telemetry.rs`](../pd-control/src/transfer/telemetry.rs)
  are useful corroboration, but are controller-specific and incomplete for a
  neutral transition boundary.
- [`pd-eval/src/runtime.rs`](../pd-eval/src/runtime.rs) writes
  `samples.json`, `actions.json`, `events.json`, `manifest.json`, and controller
  update artifacts. [`pd-eval/src/review.rs`](../pd-eval/src/review.rs) and
  [`pd-eval/src/model.rs`](../pd-eval/src/model.rs) derive current review and
  planner-provenance views, not the new neutral capability.

The repository's [`.gitignore`](../.gitignore) ignores `/outputs`, and
`git ls-files` does not list the local retained summaries or bundles. Some
ignored local spike caches still contain `10 Hz` samples from `120 Hz` physics,
while other retained summaries may reference disposable bundles that are no
longer present. Neither is stable, committed, physics-rate evidence. Fresh
physics-rate bundles from a versioned input-only development manifest are
required before fitting, threshold selection, or any capability claim. The
rejected disposable trajectory-tube data remains diagnostic or development
evidence and is never held out.

The maintained `36 / 36` handoff/ordered contracts are development
non-regression evidence. The `r-60`/`r+60` cases and every previously inspected
outcome remain already-seen diagnostic/development evidence, not held-out
evidence.

## Ownership and data flow

Ownership is intentionally staged:

- `pd-eval` owns the source and route-wide evidence DTOs, post-run extraction,
  physics-rate cadence check, raw/artifact digests, corpus freezing, held-out
  separation, outcome labels, comparison, invalidation accounting, and evidence
  artifacts.
- `pd-core` continues to own neutral physics, existing planning vocabulary,
  `SafetyProfile`, endpoint-shaped geometry, and exact terrain queries. D0a may
  add the neutral exact point-envelope query defined above; it does not add an
  event, evaluator DTO, or simulation hot-path behavior. `pd-core` may gain
  planner-facing neutral DTOs only after a capability alternative stabilizes;
  it does not own post-run evaluator extraction in this slice.
- The production capability-engine home is explicitly deferred until one
  alternative is accepted. Preserving the exact validated pure evaluator at a
  later integration boundary is an architecture gate, not permission to
  duplicate or tune it in another crate now.
- `pd-plan` remains unchanged until acceptance. It may later consume only an
  accepted, versioned physical capability and digest; it never depends on
  `pd-control`, sees controller state, or receives route-family branches.
- `pd-control` is unchanged and remains the evidence subject. Its phases,
  thresholds, markers, and controller identity cannot define the transition
  boundary or enter pure planner prediction.
- `pd-report` is display-only after evidence/capability stabilization. It is not
  an acceptance authority.

The blinded flow is:

```text
seen-input/exposure registry (digest R)
        -> input-only corpus manifest (digest M)
        -> pre-run capability/config (digest C)
        -> sealed tri-state predictions (digest P)
        -> independent held-out comparison simulation
        -> post-run SourceTransitionEvidence (digest S)
        -> post-run RouteExecutionEvidence (digest E, includes S)
        -> separate outcome/comparison artifact with scoped labels (digest O)
        -> final comparison join and gate report
```

These digests stay separate. Do not hash an exposure registry or input manifest
together with a mutable model/config as one identity. Resolved physical inputs
such as route radius may be features; seed and route-family labels may not. Any
simulation seed needed by the harness is sealed external provenance, never a
pure evaluator feature. Predictions are sealed before any authoritative
held-out comparison run; outcomes and labels are joined only after extraction
and sealing. Alternative B's own generated trace is prediction work and is
sealed with that prediction before the independent comparison run.

## D1 locked research contract

D1 remains `pd-eval` research work. It may introduce evaluator-owned DTOs and
prototype engines, but it changes no `pd-core`, `pd-plan`, or `pd-control`
contract. Moving an accepted neutral subset to another crate is a later
architecture decision.

### Common input and artifact boundary

`RouteCapabilityInputV1` is the only input accepted by either D1 predictor. Its
feature payload is reconstructed from a valid `RoutePlanningRequest` and the
selected persisted route and contains only:

- the resolved gravity and terrain, physical source/target pad geometry,
  vehicle, authoritative initial state, physical route-planning policy values,
  and resolved `SafetyProfile`;
- the exact ordered shaped centerline, normalized route geometry,
  `horizontal_sign`, ordered physical `TransferWaypointSpec` values, and
  waypoint topology.

Pad/waypoint string IDs and the request and route-plan digests exist only as
join provenance in the enclosing artifact. They are not feature values and
evaluator formulas may not branch on them. The canonical input digest covers
the canonical resolved physical payload, not its own digest or provenance
identities. Model-specific transforms are computed after input validation,
declared by the capability configuration, and covered by the capability digest;
they never change the common input bytes or input digest.

A whole `ScenarioSpec` is forbidden as predictor input. In particular,
scenario ID, name, description, seed, tags, metadata, `SimConfig`, mission goal,
run/lane identity, route-family labels, outcome, D0 evidence, controller audit,
controller configuration, planner algorithm identity, and planner ranking or
authority diagnostics are not present. A paired prototype resolves its fixed
executor pairing and step budget from its capability artifact, never from
scenario data. A valid direct route still produces a complete prediction, but
with `unknown/scope/direct_route`; it is outside this checkpoint rather than an
invalid input.

The common versioned artifacts are:

- `PhaseStateSetV1`, the conservative state-set representation described
  below;
- `RouteCapabilityArtifactV1`, containing the alternative name, model payload,
  complete numeric configuration, declared physical domain, training-evidence
  digests as audit provenance, a model/config digest, and a capability-artifact
  digest;
- `RouteExecutionPredictionV1`, containing its schema, input, model/config,
  capability-artifact, and prediction digests, ordered phase decisions, and
  first decisive reason, but no observation, outcome, run, or controller
  fields; and
- `DevelopmentComparisonV1`, the only artifact allowed to join a sealed
  prediction with D0 evidence and a separately derived scoped outcome.

A prediction has artifact status `complete` or `invalid`. Only a complete
prediction carries the tri-state decision `supported`, `unsupported`, or
`unknown`; invalid artifacts have no decision and are counted separately. A
model/config digest covers exactly the behavior-bearing model payload and full
configuration. The capability-artifact digest additionally covers schema,
alternative, declared domain, model/config digest, and ordered training or
calibration evidence identities. Predictor behavior may depend only on the
common input and model/config payload; provenance participates in artifact
identity and audit, never evaluator branching.

### State sets and ordered composition

`PhaseStateSetV1` uses finite closed intervals with `lower <= upper`. Attitude
uses a circular interval `(center, half_width)` with `0 <= half_width <= pi`,
not a scalar minimum/maximum across the wrap boundary. Its route-relative
state is complete enough to carry one phase into the next:

- shaped-route progress or event-boundary interval;
- along-track and cross-track position error;
- along-track and cross-track velocity;
- attitude, angular rate, total mass, and fuel mass; and
- elapsed time only when a phase contract requires it.

World-space hull clearance is a derived constraint over a projected state set,
not another state coordinate. Every state set and containment decision has a
stable identity and digest. `TransferWaypointSpec` defines a necessary handoff
pass set; it is not by itself the complete admissible input set for the next
leg.

Composition has one fixed order: the authoritative initial singleton,
`pad_departure`, `acquisition`, `route_leg_0`, `handoff_0`, then each remaining
route leg and handoff through the final emitted waypoint. There is no target
leg or landing phase. Each terminal set is carried as the next phase's input
set, and every required boundary containment is explicit.

The proof rules are deliberately asymmetric:

- `supported` requires a positive certificate for every phase and every
  required containment;
- `unsupported` requires an alternative-specific negative physical or
  containment certificate; inability to prove support is never a negative
  certificate;
- `unknown` covers a valid input outside the declared domain, missing empirical
  coverage, indeterminate containment, a representation that cannot make the
  required conservative claim, or a bounded solver that returns no
  certificate; and
- `invalid` is reserved for malformed or nonfinite input, schema/digest
  inconsistency, or internal determinism failure.

Reasons use the stable namespaces `invalid/input/*`, `invalid/artifact/*`,
`unknown/scope/*`, `unknown/domain/*`, `unknown/coverage/*`,
`unknown/numerical/*`, `unsupported/physics/*`, and
`unsupported/containment/*`. The first decisive reason in composition order is
authoritative; later diagnostics cannot replace it.

### Development fitting, comparison, and advancement

D1 uses the current `60` input-only development cases: `36` maintained cases
and `24` already-seen diagnostic cases. The design-locked D0 snapshot produced
`10` diagnostic scoped passes and `14` diagnostic scoped failures. The later
D1b implementation rerun, after enabling exact JSON float round-tripping,
produced `11 / 13`; that drift is reported by the unchanged gate rather than
used to alter its criteria. The current physical domain is narrow:
route angles `-60`, `-30`, `0`, `30`, and `60` degrees; route radii `776`,
`800`, and `824 m`; one- and two-waypoint topologies; two fixed vehicles with
dry masses `7200` and `11700 kg` and `6300 kg` initial fuel; Earth gravity;
two fixed terrain shapes; initial rest; and one executor pairing. Only
`horizontal_sign = +1` has corpus evidence. Unit tests for mirrored geometry do
not enlarge the empirical domain.

The fitting API may read `RouteCapabilityInputV1` and neutral, complete or
censored D0 physical evidence. It may not read scoped outcomes, comparison
rows, controller audit, or controller-private prediction types. Numeric
configurations, preprocessing, interpolation, padding, solver limits, and
reason-code mappings are declared before comparison.

Development evaluation is deterministic leave-one-physical-input-digest-out
cross-validation. The grouping identity is the canonical physical
`RouteCapabilityInputV1.input_digest`; each fold removes every row with the same
value, preventing seed, case-ID, or provenance aliases from appearing in both
fit and comparison sets. The `source_transition_resolved_input_digest` carried
as provenance `resolved_input_digest` binds the complete corpus, provenance, and
execution identity for corpus and exposure accounting; it is not the
anti-leakage fold key. D1a records the unique-digest count and fold membership;
it may describe a fold as `59 / 1` only after proving all `60` digests are
unique. Seal each excluded prediction before joining its outcome.
This is development cross-validation, not held-out evidence. After a
configuration is selected, fit the final development capability from all `60`
neutral evidence records and retain the out-of-fold predictions and comparison
report separately.

A development report is gate-conforming only when its out-of-fold results:

- has zero invalid inputs, evidence records, capabilities, or predictions;
- supports all `36` maintained scoped successes;
- supports none of the `14` diagnostic scoped failures;
- supports at least `5` of the `10` diagnostic scoped passes; and
- supports at least one diagnostic pass in every waypoint-topology by vehicle
  stratum that contains a pass.

All-`unknown` therefore cannot pass. Among configurations that satisfy every
gate, select the one that supports the most diagnostic passes, then has the
fewest unknown rows, then has the smaller canonical serialized capability
payload in bytes, with the canonical configuration digest as the final lexical
tie-break. The predeclared candidate set may be selected using the development
comparison; no parameters may vary by case/route family and no candidate or
configuration may change after that joined report is visible. These
finite-corpus gates choose a development prototype; they do not establish a
robust or production safety guarantee. Any D2 input outside the final declared
physical coverage domain must predict `unknown`. A gate-conforming A
report may advance to D2; a gate-conforming B report validates only the oracle
benchmark and does not change its integration status.

## Capability alternatives: compare, do not choose

Both alternatives are research prototypes outside planner selection. Neither is
accepted by this checkpoint.

### A. Progress-indexed route-relative reachable/state-error envelope

Alternative A is `progress_interval_envelope_v1`, a progress-indexed,
route-relative empirical state-error envelope over the persisted shaped
centerline. It is not a time-trajectory tube and does not invent a nominal
quintic or reference trajectory. Its first locked baseline uses `32` equal
normalized-progress bins per phase, half-open except for the final closed bin,
and axis-aligned intervals for the full state above, with circular attitude.
Evidence is pooled only within a physical stratum keyed by terrain-geometry
digest, vehicle-physics digest, and waypoint count. A query is covered only
when its `(signed route angle, route radius)` point lies in the convex hull of
training inputs in that same stratum and every required phase/bin contains at
least three distinct training-input digests. There is no cross-stratum
interpolation or extrapolation.

Each scalar interval is the observed per-bin minimum/maximum rounded outward
by one representable `f64`; attitude is the unique smallest circular interval
containing the samples, or `unknown/numerical/ambiguous_attitude_arc` when no
unique arc below `pi` exists. V1 has no fitted residual padding and no
outcome-selected tolerance. The versioned configuration declares the physical
feature transforms, stratum keys, binning, coverage count, outward-rounding
rule, and fixed geometry comparison tolerances. It must:

- bound cross/path error and velocity decomposition at progress-indexed states;
- bound attitude and angular rate, mass and fuel, and any other declared
  physical state dimensions; and
- sequentially carry the terminal state/error set from one leg and handoff into
  the admissible input set for the next leg.

The envelope covers authoritative physics-step states and raw boundary-bracket
endpoints. It makes no continuous-time claim between integration steps, and a
plane-interpolated crossing diagnostic cannot widen or certify a state set.

The candidate query conservatively projects each interval set into world space
for exact terrain/hull checks and proves every phase-boundary containment.
Coverage outside the declared same-stratum convex hull, an under-covered bin,
or an envelope that overlaps a constraint without proving violation yields
`unknown`.
`unsupported` requires an independent negative physical lower bound or a
proved disjoint containment; a wide empirical envelope is not such a proof.

Its planner-facing output is only a neutral, versioned capability and digest.
The appeal is compositional, interpretable coverage over the exact shaped route,
and a direct way to expose error growth. The risks are model calibration,
reachable-set composition, and optimism: independent waypoint screens are not
enough, and this remains a finite-development empirical certificate rather
than a robust reachable-set proof.

### B. Paired-executor reachability/certification

Alternative B is locked for D1 as `paired_executor_oracle_v1`, an explicit
research upper-bound comparator, not a planner-integration candidate. It runs a
fixed deterministic executor pairing from `RouteCapabilityInputV1` before the
comparison join. Pairing identity, controller/executor configuration, numeric
limits, and step budget belong to the capability artifact and audit
provenance; they never become input features or planner branches.

The oracle's decision is derived by applying the neutral D0 boundary and
ordered-contract extraction semantics to its generated physical trace. It may
not read controller phases/markers, mission outcome, or controller-private
`WaypointGuidancePrediction` state. Completion of every scoped physical phase
and contract supplies `supported`; a neutral decisive deadline or physical
failure supplies `unsupported`; censoring, an out-of-domain input, or exhausted
solver budget without either certificate supplies `unknown`. Trace/schema
failure, nondeterminism, or parity mismatch is `invalid`.

This oracle composes singleton states from one deterministic execution; it does
not certify a neighborhood of possible initial or handoff states. Its
`supported` decision therefore has only exact-pair empirical semantics and is
not interchangeable with Alternative A's set-coverage claim.

When this oracle uses the same authoritative executor/configuration as the
later comparison run, the report must label it `exact_pair_oracle`. That result
measures replay consistency and an empirical ceiling; it cannot count as
independent predictive evidence, make Alternative B eligible for D4, or justify
a production setup-time simulation screen. An offline reusable paired
reachability certificate would require a new capability version and design
gate.

The selection gate is the evidence protocol below. Prototype both alternatives
outside planner selection and compare their neutral outputs; this design
checkpoint accepts neither result in advance. If Alternative A misses its
development gate, stop and compare a small convex-feasibility formulation with
an offline paired-reachability certificate rather than adding scalar reserves
or retuning a profile. The D1 paired oracle remains a benchmark regardless of
its development score.

## Development input and extractor-fidelity gate

Before D0a implementation is accepted, commit a versioned input-only
development manifest. It contains:

- the exact resolved case identities from
  `fixtures/packs/planner_generated_route_contract_smoke.json`, which supply the
  maintained `36 / 36` positive non-regression corpus; and
- a research-only reconstruction of the already-inspected nominal-radius
  `r-60`/`r+60` expansion inputs from the archived `bde34c5` branch material,
  reviewed to contain physical inputs and provenance only.

The reconstructed diagnostic cases are development negatives/positives, not a
maintained acceptance pack and never held out. Do not copy the rejected
trajectory evaluator, expected outcomes, class labels, result summaries, or
controller-derived features into the manifest. The manifest records its source
commit and input digest. A declared evidence overlay may change only artifact
retention and `sample_hz` to equal the resolved `physics_hz`; all physical,
mission, planner, controller, and seed inputs remain identical to the declared
development case.

Each development case receives two deterministic captures: its ordinary
configured-cadence capture and a physics-rate evidence capture. The ordinary
capture is parity evidence only. After normalizing sample-artifact identities,
the two captures must have identical resolved inputs, action and controller-
update sequences, events, terminal step/reason, mission/physical outcomes, and
shared-cadence raw sample states. A mismatch means sampling changed execution
or provenance and blocks D0a; the lower-rate bundle is never promoted to
physics-rate evidence. Re-running either extractor over identical immutable
inputs must produce byte-stable canonical serialization and the same digest.

The minimum D0a/D0b test matrix covers:

- contiguous physics steps and `time == step / physics_hz`, with duplicate,
  missing, nonfinite, out-of-order, and wrong-cadence inputs classified
  `invalid` using stable reasons;
- initial-state anchoring, first directed crossings, an exact-boundary sample,
  both `horizontal_sign` directions, no crossing, reverse crossing,
  backtracking, and re-entry without boundary reselection;
- complete, `censored_before_contact_exit`,
  `censored_before_tracking_entry`, route-level waypoint censoring, and
  malformed-prefix status behavior;
- shortest-displacement attitude interpolation while retaining authoritative
  raw endpoint angles;
- exact point-envelope clearance for vertical/zero-horizontal-motion samples,
  terrain vertices, envelope domain limits, and invalid geometry; and
- capture-window entry, deadline-plane crossing, sticky raw-derived window
  state, first neutral contract-satisfying sample, repeated/reverse crossings,
  per-leg partitioning, and controller-marker audit attachment without
  marker-defined contract results.

## Blinded protocol and hard gates

1. Regenerate complete physics-rate raw bundles for the maintained development
   and already-seen diagnostic corpus from the versioned development manifest.
   Use the `36 / 36` handoff/ordered contracts for development non-regression,
   not for held-out discrimination.
2. Keep `r-60`/`r+60` and all previously inspected outcomes marked
   diagnostic/development. Do not promote them by relabeling or by reusing
   committed summaries.
3. Before running or seeing any outcome, predeclare and hash an input-only
   matrix with genuinely uninspected resolved physical inputs. The
   discrimination matrix uses unseen route angles/radii strictly inside the
   accepted capability's same-stratum convex hull; its terrain, vehicle-physics,
   and waypoint-topology keys remain in the declared D1 domain. A separately
   reported abstention matrix may vary terrain, vehicle, or load outside that
   domain and must predict `unknown`; those rows do not provide in-domain class
   support or enlarge the capability. Predeclare the minimum useful coverage,
   class-support requirements, stratification, and confidence-reporting method
   before execution. The manifest digest must remain separate from
   capability/config digests. First freeze a seen-input/exposure registry
   covering repository fixtures, committed summaries, development manifests,
   retained research inputs, and any manually inspected run known to the study.
   The registry records normalized resolved-input digests, factor values, and
   whether outcomes were exposed. A held-out row is ineligible if its
   resolved-input digest was seen or if an angle/radius declared unseen was
   previously exposed; uncertain exposure is treated as seen. Seal the registry
   digest separately and append newly exposed held-out rows only after the
   comparison is complete.
4. Lock the extractor, evidence schema, alternative model/config, capability
   version, and any thresholds before revealing held-out outcomes. No physical
   threshold is selected by this checkpoint; once one exists, its exact value
   and implementation are frozen before reveal.
5. Run each predictor from the common input without route labels, seeds,
   controller phases, or recorded outcomes. Alternative B may resolve only the
   fixed pairing declared by its already-frozen capability. Seal the tri-state
   predictions and B's prediction-generation trace digest before the
   authoritative held-out comparison simulation. Then run the independent
   comparison simulations, extract and seal
   `SourceTransitionEvidence` and `RouteExecutionEvidence`; separately derive
   and seal scoped labels in the outcome/comparison artifact, and only then
   perform the final comparison join.
6. Report confusion counts, useful coverage, discrimination, confidence, and
   every invalidation separately for development and held-out inputs. Unknown,
   invalid extraction, and analytic invalidation are explicit
   abstention/coverage rows, never true negatives. Right-censored transition
   statuses remain valid evidence; their separate scoped terminal-failure
   labels may participate in confusion rows when a prediction exists.

For comparison, `supported` versus a scoped failure is a false accept, and
`unsupported` versus a scoped pass is a false reject. `unknown` makes no class
claim. Zero observed held-out false accepts is necessary for a finite-corpus
rejection claim, but it is not a production safety proof.

The hard gates are all required:

- preserve the maintained `36 / 36` development contracts;
- preserve execution parity between ordinary-cadence and physics-rate evidence
  captures, with the ordinary capture excluded from evidence fitting;
- treat `censored_before_contact_exit` and `censored_before_tracking_entry` as
  valid right-censored evidence; their separate scoped terminal-failure labels
  are comparison data, not invalidations;
- record zero invalidated evidence records in the claimed development and
  held-out set;
- record zero observed held-out false accepts for a finite-corpus production
  rejection claim;
- keep route labels, seeds, controller IDs, controller phases, and outcomes out
  of the pure crossing kernel and prediction features; terminal step/reason
  metadata is limited to the post-run assembler's censor-status audit;
- use no route-family-specific branches or hidden controller/profile defaults;
  and
- demonstrate useful coverage and discrimination with the declared class
  support and confidence report.

If class support is insufficient, make no discrimination claim. Freeze a new
independent confirmatory matrix rather than extending the revealed set
opportunistically. If any hard gate fails, stop planner-facing work and compare
small convex feasibility with paired reachability. Scalar reserve/profile
retuning is not an escalation path.

## Implementation order: D0a-D5

The following order is the implementation gate. Every phase remains outside
production planner selection until the later acceptance gate passes.

### D0a — neutral source-transition evidence

**Entry:** The versioned input-only development manifest is reviewed and can
regenerate both the maintained `36 / 36` cases and already-seen diagnostic
`r-60`/`r+60` cases. Existing route plan/profile and artifact identities are
available; the physics-rate overlay changes no execution input except sample
retention cadence.

**Work:** Add the neutral `pd-core` exact point-envelope clearance query and its
geometry tests. Implement the `pd-eval`-owned physics-rate crossing kernel and
post-run `SourceTransitionEvidence` assembler from the authoritative initial
state through `contact_exit` and `tracking_entry`. Run the declared cadence,
execution-parity, crossing, censoring, clearance, and deterministic-replay test
matrix. Add no capability model, planner change, controller change, or new
hot-path/core event.

**Exit:** Regenerated development bundles produce an authoritative initial
anchor, auditable two-boundary brackets, separate pad-departure/acquisition
extrema, backtracking/re-entry, exact shaped-centerline references, the distinct
observed and derived clearance series, stable invalidation reasons, and separate
digests.
Sound terminal prefixes produce explicit right-censored statuses with terminal
step/reason rather than invalid evidence. Ordinary-cadence and physics-rate
captures pass execution parity. The pure crossing kernel and emitted physical
features exclude labels, seeds, controller identity, outcomes, and terminal
metadata. The post-run assembler may read terminal step/reason from
manifest/events solely to assign a censored status and stores those fields as
audit/status; it never feeds them into crossing math or prediction features.

### D0b — neutral route-execution evidence

**Entry:** D0a is byte-stable and passes its full development fidelity gate.
The same physics-rate bundles contain the resolved shaped route and every
required waypoint contract.

**Work:** Implement the `pd-eval`-owned `RouteExecutionEvidence` assembler.
Map the full raw prefix to shaped-centerline legs; derive capture-window-entry
and deadline-plane brackets plus the neutral contract-opportunity series for
every waypoint; record the first raw contract-satisfying state when present,
full-state, path-error, clearance, backtracking, and per-leg extrema. Attach
controller markers only as audit evidence. Add no predictor, planner screen,
controller behavior, or maintained acceptance threshold.

**Implemented checkpoint:** The sibling `route-execution-gate` regenerated all
`36` maintained and `24` already-seen diagnostic inputs. It preserved `36 / 36`
maintained contracts, passed ordinary-versus-physics execution parity for
`60 / 60` cases, produced `60` complete route artifacts with zero source or
route invalidations, and reproduced deterministic source/route evidence for
`60 / 60` repeat extractions.

**Exit:** The maintained and diagnostic development bundles produce complete or
sound route-censored artifacts with deterministic digests. Every available leg
and waypoint opportunity has raw bracket/state provenance sufficient to
calibrate or validate terminal-set composition. Controller markers, mission
outcomes, and ordered/handoff labels remain outside neutral extraction and
predictor features; malformed input alone is `invalid`.

### D1 — prototype and lock both alternatives on development evidence

**Entry:** D0a and D0b evidence pass the complete cadence, execution-parity,
extractor, and provenance gates for the maintained and already-seen diagnostic
development sets.

**D1a — common contract and development gate:** Implement the `pd-eval`-owned
input, state-set, capability, prediction, and comparison artifacts above.
Derive and seal a D1 input manifest from the existing input-only `60`-row D0
manifest, plus a separate fresh `36 / 10 / 14` outcome overlay. Enforce
forbidden-field tests and implement deterministic
leave-one-input-digest-out fitting, sealing, comparison, and advancement
reports. This slice introduces no model.
It exits only when canonicalization is byte-stable, forbidden identity/outcome
mutations leave the feature payload and digest unchanged, physical mutations
change them, malformed state sets and digest mismatches are rejected, and mock
predictions prove the all-unknown and false-accept gates cannot pass.

**D1b — Alternative A:** Implement `progress_interval_envelope_v1` with its
locked phase/bin representation. Predeclare the candidate configurations, fit
only neutral D0 physical evidence, emit out-of-fold predictions, and evaluate
the development advancement gate. Fit an all-development capability only after
configuration selection.

**D1b result:** Implemented. Fresh D0 evidence passed `36 / 36` maintained
contracts, `60 / 60` cadence parity, and `60 / 60` deterministic replay with
zero invalidations. The fixed candidate then produced 60 byte-stable
out-of-fold `unknown/domain/missing_physical_stratum` predictions: the exact
terrain-geometry key creates one-member strata, so excluding each query digest
removes the only training member. The advancement gate failed, the candidate
was not retuned after the outcome join, and Alternative A cannot advance to D2.
The prescribed next design checkpoint is a comparison of small
convex-feasibility and offline paired-reachability formulations; D1c remains
useful only as the already-declared oracle benchmark.

**D1b terrain-equivalence follow-up:** Implemented as a non-normative,
input-only support spike, not a new predictor. The existing exact key again
produces `60` singleton groups. A source-to-target chord-relative relief motif
with fixed `0.1` location buckets, `10 m` height buckets, and only `1e-12`
normalized arithmetic-boundary snapping produces eight groups of
`15, 15, 9, 9, 4, 4, 2, 2`. Its minimum leave-one-out group support is one;
only `30 / 60` rows satisfy the combined support check, eight maintained rows
are outside their leave-one-out route-angle/radius hull, and 139 required cells
have zero support. The exact query terrain remains bound and mandatory. The
byte-stable artifact `5fd070e32738f4b4` therefore records
`viable_for_a2_design = false`; no A2 model, outcome-guided regrouping, or D2
entry is authorized.

**D1c — Alternative B oracle:** Implement `paired_executor_oracle_v1` through
public neutral contracts, with a fixed pairing resolved by the capability.
Prove trace determinism/parity and emit the same common prediction schema. Keep
the `exact_pair_oracle` result visibly separate from integration eligibility.

**D1d — comparison lock:** Publish both development reports, input,
model/config, capability-artifact, prediction, and comparison digests, coverage
by declared domain and stratum, and all first-decisive reasons. Alternative A
advances to D2 only if it passes every development gate. Alternative B remains
an oracle benchmark; if A fails, no predictor advances and the next design
checkpoint compares an offline paired-reachability certificate with the small
convex-feasibility escalation.

**Exit:** Both alternatives expose byte-stable predictions, reasons, and
digests. Terminal state-set composition is explicit from the initial source
state through every handoff, out-of-fold comparison is outcome-isolated, and
phase-indexed reasons preserve the first decisive unsupported/unknown boundary.
No per-candidate production screen or planner integration exists.

### D2 — freeze held-out inputs and seal predictions

**Entry:** D1 development prototypes are reproducible, the seen-input/exposure
registry is reviewable, and the predeclared class-support/stratification/
confidence plan is reviewable.

**Work:** Freeze and separately hash the exposure registry and genuinely
uninspected input-only matrix. Reject contaminated rows before simulation.
Freeze the capability/config digests and seal both alternatives' tri-state
predictions, including Alternative B's prediction-trace digest, before any
authoritative held-out comparison simulation runs.

**Exit:** Exposure-registry, manifest, capability/config, and prediction digests
are separate and immutable. No held-out outcome, label, seed, route family, or
controller ID has entered the common input or evaluator branching; B's one
fixed pairing remains isolated in its capability.

### D3 — reveal held-out runs and compare

**Entry:** D2 predictions are sealed and the held-out matrix is immutable.

**Work:** Run the independent authoritative held-out comparison simulations,
regenerate raw physics-rate bundles, extract both boundaries or the sound
available prefix for a censored status,
seal `SourceTransitionEvidence` and `RouteExecutionEvidence`; separately derive
and seal scoped outcomes in the outcome/comparison artifact, then publish
confusion, coverage, invalidations, class support, stratification, and
confidence reports.

**Exit:** The hard gates are evaluated without counting `unknown`, invalid, or
analytic-invalidation rows as true negatives; censored terminal failures are
valid outcome rows and a `supported` prediction joined to either one is a
false accept. Alternative A may be accepted only if the evidence justifies its
capability; B remains an oracle report and this document itself makes no
selection.

### D4 — bounded candidate exposure

**Entry:** Alternative A, or a separately designed offline certificate that is
not the D1 oracle, has passed the declared development and held-out gates and
has an accepted versioned capability/digest.

**Work:** Expose a bounded set of existing planner candidates for research
replay. Determine whether an existing candidate passes the richer capability;
keep any paired executor or simulation oracle outside planner selection.

**Exit:** The report distinguishes selected-route capability from candidate-set
limitations. Candidate generation and ranking, route topology limits, and
controller behavior remain unchanged.

### D5 — later planner integration decision

**Entry:** D4 provides an accepted capability, complete provenance, and an
architecture review of its neutral DTO and capability-engine home.

**Work:** In a separate checkpoint, decide whether `pd-plan` should consume only
the accepted versioned physical capability and digest. Preserve the validated
pure evaluator at the integration boundary; do not duplicate or retune it.

**Exit:** Either a narrowly scoped integration contract is approved, or the
work stops and returns to convex-feasibility versus paired-reachability design.
`pd-plan` never acquires a `pd-control` dependency.

## Ground references

- [`pd-core` planning contracts, profile, and shaped centerline](../pd-core/src/planning.rs)
- [`pd-core` simulation models and raw artifact structures](../pd-core/src/model.rs)
- [`pd-core` sample capture and physics authority](../pd-core/src/sim.rs)
- [`pd-core` terrain envelopes and exact clearance queries](../pd-core/src/terrain.rs)
- [`pd-plan` deterministic bounded planner](../pd-plan/src/lib.rs)
- [`pd-eval` bundle writer](../pd-eval/src/runtime.rs)
- [`pd-eval` review and comparison models](../pd-eval/src/review.rs)
- [`pd-eval` resolved run/provenance models](../pd-eval/src/model.rs)
- [`pd-control` run/update artifact boundary](../pd-control/src/lib.rs)
- [`pd-control` waypoint markers](../pd-control/src/transfer/telemetry.rs)
- [`.gitignore`](../.gitignore)
- [Waypoint Planning V1](waypoint_planning.md)
- [Trajectory-tube shadow spike](trajectory_tube_spike.md)
- [Architecture](architecture.md)
