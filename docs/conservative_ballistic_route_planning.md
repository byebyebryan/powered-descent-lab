# Conservative Ballistic Route Planning

## Status

CB0 is complete and found no current gameplay repair problem: the exact `36`
maintained and `24` already-seen diagnostic inputs produced `60 / 60` target
touchdowns and zero fuel-depletion diagnostics in two byte-identical runs.
That result is evidence about the current corpus, not a reason to add a new
planner to it.

The current forward-looking checkpoint is the V2 direct-ballistic bridge
certificate plus one bounded ridge canary. It is a controller-independent,
setup-only analytical model for a small video-game mission family. The canary
selects the shortest robust direct certificate on a flat twin, treats the end
of its source bridge as the direct-flight commitment point, and proves that a
derived mesa blocks that nominal lane even after an intentionally optimistic
local-correction allowance. A small terrain-derived search then finds a
one-waypoint analytical witness under the same policy.

This is a scoped nominal-lane red/one-waypoint-green result, not proof that
every direct route is impossible. Higher global direct replans still clear the
mesa. The canary does not change the production planner or run a controller or
simulation.

F5 extends this checkpoint with a fresh, held-out pair from the same ridge
family. Its purpose is to test whether the already-defined analytical and
runtime-V2 behavior is repeatable on nearby inputs, not to tune the policy or
to promote the experiment into production. F5 is now complete as **NARROW**:
`ridge_progress_056_probe` passed its analytical, runtime-V2, and three frozen
controller expectations, while `ridge_progress_072_probe` stopped at finite
waypoint-search exhaustion and did not run a controller lane. This is useful
bounded evidence, not an overall F5 PASS/full green or production authority.

F6 is now design-locked as a limited, opt-in integration checkpoint for the
demonstrated `056` derived-mesa mission. It does not reopen F5 or search for a
replacement held-out case. The sealed `072` record remains immutable
historical evidence, but its derived blocker leaves an unrepresentative
terminal-recovery region and it is rejected from all F6 fixtures, acceptance
gates, tuning, and capability-boundary claims.

The research-grade D2 distinction remains unchanged. The W4 finite proposal
backend found no useful development coverage, but that is `unknown` coverage
evidence, not proof of physical infeasibility. This document is about a simpler
gameplay-oriented analytical certificate, not a D2 feasibility claim.

The setup report is intentionally versioned:

```text
command: cargo run -p pd-eval -- conservative-ballistic-report
setup:  conservative-ballistic-direct-bridge-v2
summary: outputs/setups/conservative-ballistic-direct-bridge-v2/summary.json
report:  outputs/reports/setups/conservative-ballistic-direct-bridge-v2/index.html
```

Those files are written when the command is run. They are analytical setup
evidence only; they contain no controller run, event stream, sample stream, or
simulation result.

## Product and evidence boundary

The direct certificate asks a deliberately modest question:

> Can a direct ballistic transfer be certified with simple, conservative
> source and terminal bridges that a game controller could plausibly execute?

The certificate may reject a route that a controller could fly. It must not
accept a route by copying controller thresholds, controller outcomes, or
per-case exceptions into the planner. “Not certified” means that this bounded
analytical policy found no certificate; it never means that the mission is
physically impossible.

The ridge canary adds a second, equally bounded question:

> After a robust nominal direct lane has committed to coast, can a terrain
> feature defeat even a generous local correction bound while a simple route-
> level one-waypoint replan still has an analytical certificate?

The current checkpoint therefore separates four concerns:

- `pd-plan` derives and checks the analytical certificate from world, vehicle,
  mission, and versioned policy inputs;
- `pd-report` renders the setup geometry and evidence without simulating it;
- `pd-eval` recomputes and identity-validates the report; and
- a real controller and full simulation remain a later validation step, after
  a planner policy and any waypoint solver are frozen.

## CB0 gameplay audit

The evaluator-owned audit applied the ordinary `landing_on_pad` goal to the
exact current inputs without changing their scenarios, route plans,
controllers, or execution artifacts. It recorded:

- `36` maintained plus `24` diagnostic inputs;
- `60 / 60` target touchdowns;
- zero fuel-depletion diagnostics;
- byte-identical summaries from two fresh output roots; and
- summary identity `be08b90305456370`.

CB0 closes the claim that the current corpus needs route repair. Any future
conservative-ballistic mission is intentionally new game content, not a hidden
fix for those cases.

## Superseded V1 negative experiment

The first CB1 model is retained only as historical implementation context. It
used fixed absolute geometry, including an `800 m` source gate/exit reserve,
`400 m` intermediate entry/exit reserves, and a `450 m` terminal gate offset
with an `800 m` terminal gate height. It also carried authored one-gate
coordinates and a report that displayed those gates as route-necessity
evidence.

That model manufactured the very property it was supposed to measure: the
arbitrary gate geometry and authored one-gate oracle made staged survivors look
like proof that direct flight was insufficient. Its fixed-distance powered
polylines were not a controller-independent trajectory and were not a sound
basis for waypoint necessity. The V1 fixture and test-only implementation may
remain as historical context, but they are superseded and must not be used as
current evidence or regenerated under the V2 setup ID.

## V2 direct-ballistic certificate

### Inputs and endpoints

The V2 fixture contains four neutral probes using one shared policy and vehicle:

```text
clear_direct_probe
long_span_probe
ridge_probe
long_range_probe
```

There are no authored gates, one-gate expected outcomes, controller labels, or
route-family exceptions.

The direct candidate starts from a derived source release reference. Its center
is the source pad contact center raised by the vehicle touchdown base offset
and the shared minimum free-flight clearance. The candidate ends at the target
touchdown reference on the target pad's legal supported-contact plane. No fixed
`800 m`, `400 m`, or `450 m` gate dimensions enter the construction.

### Exact ballistic coast

For a coast with physics step `dt`, gravity `g`, and `N` semi-implicit steps,
the evaluator uses the exact discrete plant equations:

```text
x_N  = x_0 + N dt vx_0
y_N  = y_0 + N dt vy_0 - 0.5 g dt² N(N + 1)

vx_0 = (x_N - x_0) / (N dt)
vy_0 = (y_N - y_0) / (N dt) + 0.5 g dt (N + 1)
vy_N = vy_0 - g N dt
```

The four existing duration multipliers remain the finite candidate set. Each
candidate retains its exact departure and arrival states, apex, duration, and
identity. Terrain is checked at every discrete state with the vehicle envelope:
coasts use the circumscribed hull radius, powered bridges use the rotated hull,
and airborne states include the shared free-flight clearance reserve.

### Analytical source and terminal bridges

Source and terminal handoffs are discovered on a deterministic grid: handoff
samples are `0.25 s` worth of physics ticks apart, and bridge durations are
`0.5 s` worth of ticks apart, bounded by the mission budget and reserve. Source
handoffs lie on the ascending arc; terminal handoffs lie on the descending arc.
The two powered bridges must not overlap, and the intervening coast must leave
enough time to slew between their thrust directions.

Each bridge uses an exact affine total-net acceleration sequence:

```text
a[k] = a0 + b k,       k = 0 .. N - 1
```

The closed-form coefficients are selected to reproduce both endpoint position
and endpoint velocity under the semi-implicit update. Required thrust is the
net acceleration plus gravity. This is a controller-neutral idealized bridge,
not a command stream or a simulated pilot. The durable report stores the
coefficients and scalar evidence rather than a physics-rate sample log; its
display points are reconstructed exactly from the same closed form.

### Conservative screens

Every candidate records separate margins and reasons for:

- worst-case mass, derated coupled thrust, and the minimum nonzero throttle;
- exact bridge endpoint reproduction;
- per-tick rotated-hull free-flight terrain clearance;
- a legal vertical pad-clearance corridor while the full touchdown footprint
  remains over a declared flat pad, plus exact endpoint contact rather than an
  invented altitude gate;
- powered direction slew and coast slew;
- conservative fuel consumption using maximum burn rate;
- mission time and reserve;
- terminal tangential/normal speed, attitude, and angular rate; and
- the declared nonzero robustness policy.

The robustness value is dimensionless. Bound margins are normalized by their
own declared limit; residual free-flight clearance is normalized by the shared
minimum-clearance reserve; numerical endpoint/contact tolerances retain their
own relative scale. Candidate ranking therefore does not compare metres,
radians, seconds, and fractions as if they were interchangeable raw units.

The evaluator does not invoke `pd-control`, a reference pilot, or the full
simulation. A certified result is therefore an analytical certificate, not a
measured controller success.

## V2 bounded ridge canary

### Flat-derived nominal lane

The canary begins with `ridge_probe`, removes its elevated terrain to make a
flat twin, evaluates the same four duration multipliers, and chooses the
shortest robust certificate rather than a case label or authored preferred
arc. The result is the `1.0x` candidate. The `0.75x` candidate fails the shared
screens, while `1.25x` and `1.5x` remain longer global alternatives.

The nominal source bridge hands off to coast at approximately `(1625.2,
966.4) m`. That state is the explicit commitment boundary. Corrections before
it would change the planned boost and are therefore global replans, not local
reactions to a now-visible obstacle.

### Optimistic local-correction envelope

From the commitment state to the ridge crossing, the canary over-approximates
local correction with full derated thrust acceleration at worst-case mass in
any direction. It ignores attitude slew and throttle granularity, reserves the
nominal terminal-bridge fuel plus the mission fuel reserve, and caps correction
time by the remaining fuel, mission budget, and ridge-crossing horizon. A
`0.758 s` allowance derived from existing analytical sampling intervals is
added at the crossing so the boundary is not a zero-latency knife edge. The
envelope carries the nominal ballistic state through that time shift as well
as adding thrust authority; the delay is not treated as extra thrust alone.

For the current fixture, the ridge is `5.725 s` beyond commitment and the
resulting correction horizon is `6.483 s`. The crossing displacement bound is
about `309 m` horizontally and `314 m` vertically, excluding as much as
approximately `106 m` of separately bounded forward ballistic drift during
the time shift. The blocker is derived from all `318` exact physics ticks whose
reachable horizontal interval contains the ridge center, not from the nine
sparse samples used to draw the report tube. Because this construction grants
more instantaneous directional authority than a real controller, a mesa that
blocks it also blocks the narrower class of ordinary post-commit local
corrections represented by this canary. It does not block a new boost plan
chosen before commitment.

### Derived blocking mesa

The canary expands the existing ridge feature to cover the exact crossing cut
and raises its top by the shared clearance plus robustness margin. The cut
spans approximately `x = 2002.8 .. 2840.4 m`; the resulting flat mesa top
covers the same interval at `y = 1278.3 m`.
Mission-level direct status becomes red only when all three conditions hold:

- the flat-derived nominal candidate is rejected on the derived terrain;
- the correction-envelope evidence is internally valid; and
- the mesa covers the complete declared crossing bound with the shared
  clearance margin.

The `1.25x` and `1.5x` direct candidates remain certified and are reported as
global-replan diagnostics. They are important positive controls: the canary
demonstrates why an already-committed nominal route needs route-level repair,
not physical impossibility or universal waypoint necessity.

### Terrain-derived one-waypoint witness

The witness search generates positions from the derived mesa edges and the
shared vehicle-clearance envelope. It combines those positions with the same
four ballistic duration multipliers and a finite, evenly sampled subset of the
existing bridge-duration policy. There is no stored winning coordinate,
controller threshold, per-case branch, or arbitrary altitude gate.

The current search has four legal waypoint positions and `64` leg-duration
pairs, with a hard cap of `96` evaluated candidates. It examines `12`
candidates before finding a certified witness near `(1983.6, 1301.1) m`. The
two ballistic legs are joined by exact source, intermediate, and terminal
affine bridges; all shared thrust, throttle, terrain, slew, fuel, time,
endpoint, touchdown, and robustness screens pass. A generic ground-track
progress screen rejects looping or materially backtracking joins: both coasts
and the intermediate bridge are strictly forward-progressing, while the two
pad-end bridges use about `1.50 m` total local alignment motion inside their
physical `4 m` touchdown-half-span allowance. The witness uses approximately
`2932 kg` of the declared fuel and `105.1 s` of the `170 s` mission budget.

This search exists only to establish the canary and expose complete evidence.
It is not production planner wiring or a general backward waypoint solver.

## Observed V2 probe results

The current V2 evaluation has four duration candidates per neutral probe:

| Probe | Direct certificates | Observation |
| --- | ---: | --- |
| `clear_direct_probe` | `3 / 4` | the shorter arc fails honestly; longer arcs clear |
| `long_span_probe` | `3 / 4` | the shorter arc fails honestly; longer arcs clear |
| `ridge_probe` | `2 / 4` | higher/longer direct arcs clear the narrow ridge |
| `long_range_probe` | `3 / 4` | the shorter arc fails honestly; longer arcs clear |

The four original rows remain direct-only diagnostics. The derived ridge
canary adds a separate result: flat control green, committed nominal lane red,
and one terrain-derived waypoint witness green. Higher direct arcs still clear,
so the result is deliberately about the selected nominal lane plus local
correction allowance. Rejections remain analytical diagnostics, not physical
impossibility or controller-failure claims.

## Frozen full-controller shadow

The first full-controller shadow deliberately reuses the V2 fixture without
retuning it. It runs at `120 Hz` physics and `60 Hz` control with the built-in
`transfer_pdg` and `transfer_waypoint_pdg` controllers, persists ordinary run
artifacts, and overlays the analytical witnesses with the simulated paths.

The first harness attempt omitted the required direct `TransferRouteSpec` and
therefore entered terminal guidance on the source pad. That step-one crash was
a setup error, not controller or canary evidence. The corrected shadow derives
one identical controller-facing direct route from the two pads (`0 deg`,
`3982 m`, zero waypoints) and attaches it to both direct lanes. Both then begin
in the built-in takeoff phase with an upright command and survive launch.

The corrected paired result validates the **direct red half** of the canary:

- the flat direct control lands on the target in `101.8 s` with approximately
  `3615.1 kg` fuel remaining; and
- the mesa direct lane crashes after `27.692 s` on the derived mesa's rising
  left wall. Simulator-equivalent rotated-hull reconstruction identifies a
  hull vertex near `(1999.07, 427.36) m`, where terrain is approximately
  `536.36 m`; its `-108.99 m` residual exactly matches the recorded minimum
  hull clearance.

The virtual analytical anchor near `(1983.6, 1301.1) m` is not itself traversed
by the certified powered join. The ordinary full-pad `validate_route` contract
also rejects both the zero-waypoint and one-waypoint routes identically on the
fixed source taper: the derived transition spans directed progress `[22, 118]
m`, and the reported terrain point at world `x = 105.597 m` is progress
`87.597 m` from the source. That failure is therefore waypoint-invariant and
is retained as a separate diagnostic rather than relabeled as an adapter
result.

The evaluator-scoped composed preflight closes this mismatch without changing
the ordinary validator. It splits the still-certified analytical witness at
the first exact intermediate-bridge state whose directed `x` reaches the
virtual-anchor plane. The crossing is bracketed by applied steps `971 -> 972`
of `1980`; the selected state is approximately `(1983.821, 1566.110) m` with
velocity `(66.125, 1.347) m/s`. A runtime waypoint at that actual state uses
the existing `95 m` capture/cross-track bounds, canonical route-bisector
tangent, and `104.139 m/s` authority cap. The exact analytical state passes
`TransferWaypointSpec::assess_handoff`. This is explicitly a composition of an
analytical path certificate and a structurally valid runtime handoff contract,
not an ordinary `RouteValidation`, a replayed command trace, or a prediction
that the controller must succeed.

The one permitted frozen mesa-waypoint lane then provides the separate runtime
result: the unchanged `transfer_waypoint_pdg` controller passes the waypoint
contract at `34.467 s`, lands on the target at `86.692 s`, and retains
approximately `4042.6 kg` of fuel. Together with the unchanged mesa-direct
crash, this establishes the scoped controller-level nominal-direct-red/
one-waypoint-green canary without controller, terrain, vehicle, cadence, or
parameter tuning. Higher global direct replans remain possible, so this is not
a universal route-necessity claim.

The shadow is generated with
`cargo run -p pd-eval -- controller-shadow`. Its sealed summary and per-lane
run artifacts live under
`outputs/eval/conservative-ballistic-controller-shadow-v1/`; the HTML/SVG
report lives under
`outputs/reports/eval/conservative-ballistic-controller-shadow-v1/`.

## CB3 route-projection contract (implemented)

CB3 implements the feature-gated, `pd-plan`-owned runtime-route projection over
the already validated `ExperimentalRidgeCandidateProjectionV2` /
`WaypointCandidateV2` decision and its `ExactIntermediateBridgeCrossingV2`
evidence. The projection emits the controller-facing `TransferRouteSpec`,
exact bridge-crossing/handoff state, authority and handoff evidence, structural
validation, stable identities, and typed fail-closed invalid/unsupported
reasons. It remains experimental and does not promote the result into the
production `plan()` API or a `RoutePlan`.

The ballistic certificate remains the sole controller-independent proof of
end-to-end reference-path feasibility. CB3 must neither reinterpret nor weaken
`pd_core::validate_route`, and it must not introduce a certified-prefix
exception to that ordinary full-pad validator. Its known source-taper rejection
is an honest diagnostic about that contract, not an invalidation of the
analytical witness or a reason to alter `pd-core` terrain semantics.

### Ownership and emitted evidence

The feature-gated planner projection will emit only the controller-facing
runtime projection and its planner-neutral evidence:

- the selected `TransferRouteSpec` and exact bridge-crossing/handoff state;
- waypoint authority diagnostics plus canonical handoff kinematics and
  assessment;
- structural route validation;
- stable candidate, selection, crossing, and projection identities; and
- stable typed invalid and unsupported reasons that fail closed when a required
  analytical selection, bridge, or join is missing or altered.

It must not carry controller IDs, controller phases, simulator outcomes, run
artifacts, or evaluator report labels. `pd-core` continues to own neutral route
structures, normalized geometry, endpoint profiles/centerlines, authority, and
waypoint-handoff semantics. A reusable `pd-core` handoff-kinematics helper may
be a useful implementation refinement, but it is neither required by this
contract nor authorization to change `pd-core` now.

`pd-eval` will consume the projection rather than reconstruct planner
certificate validity as an independent decision. The ordinary zero- and
one-waypoint full-route source-taper comparison remains outside the planner
projection and never affects planner acceptance. `pd-eval` retains that
comparison, including evaluator-local error-string parsing, as a frozen-canary
artifact-integrity/preflight gate: a changed diagnostic invalidates v4
compatibility but does not alter the projection's decision. It will map the
result to the unchanged controller-shadow v4 artifact and run the same frozen
controller lanes. `pd-control` remains unchanged.

CB3 therefore claims exactly:

> A certified controller-independent ballistic reference trajectory projects
> to a structurally, authority-, and handoff-compatible controller route.

It does **not** claim ordinary `RouteValidation`, a replayed command stream, a
guaranteed real-controller landing, arbitrary-mission support, or universal
waypoint necessity. Generic source-departure D2 remains blocked; CB3 does not
repair or advance it.

### Implementation and evidence gate

The bounded implementation is complete:

1. `pd-plan` constructs the projection and covers valid direct/one-waypoint
   selections, altered or missing certificate joins, unsupported finite search,
   stable reasons, and identities with fail-closed tests.
2. `pd-eval` consumes that projection while preserving the exact runtime route
   bytes, controller-shadow schema `v4`, semantic identity
   `a69de7872ad039fd`, unchanged three lane outcomes, and the ordinary
   source-taper diagnostic.
3. Focused runtime-projection tests (`2`) and controller-shadow tests (`6`)
   pass. Canonical regeneration and summary reload pass with schema `v4`,
   `deterministic_repeat: true`, and the preserved identity and outcomes. The
   committed evaluation and report outputs remain byte-clean.

The resulting canary remains exact: flat-direct is a target landing,
mesa-direct is a derived-mesa terrain crash, and mesa-waypoint passes the
waypoint contract and lands on target. The ordinary zero-/one-waypoint
source-taper rejection remains an evaluator-local artifact-integrity
diagnostic; the composed planner projection remains `supported`.

CB3 changes no generic source-departure contract, fixture, controller,
production planner path, `pd-core::validate_route`, or `pd-control` behavior.
The later H1 implementation preserves those boundaries. H2a froze the two
identity-bound raw inputs and prediction manifest; H2b subsequently ran the
analytical-only evaluation and stopped at the typed runtime handoff error
recorded below. H3 was not run, and no second topology is added.

### H0 held-out design closure (documentation-only)

H0 is now design-closed as the first controller-held-out, pre-simulation
evidence gate. It asks whether the same analytical candidate and runtime
projection remain compatible with a real controller on two uninspected inputs
from the already demonstrated ridge family. “Held-out” means that the input
construction and analytical predictions are frozen before any controller
outcome is inspected. This is execution-compatibility evidence for a small
gameplay experiment. It is not generic source-departure D2, a statistical or
universal proof, production integration, or a second terrain topology.

#### H1 input and API boundary

H1 introduced a separate, versioned, input-only
`ExperimentalRidgeCaseInputV1` schema. Its serialized contract is:

```text
ExperimentalRidgeCaseInputV1
├── schema_id: experimental_ridge_case_input_v1
├── schema_version: 1
├── policy: DirectBridgePolicyV2
├── vehicle: VehicleInputV2
├── probe: DirectBridgeProbeV2       # exactly one case
└── identity                         # canonical input digest
```

The caller supplies only this policy, vehicle, probe, and `identity`. It does
not supply a mesa, candidates, outcomes, crossings, runtime routes, report
labels, controller data, or any other derived evidence. Held-out inputs remain
separate from the fixed four-case `DirectBridgeFixtureV2`; they must not be
smuggled into that fixture or selected by a case-ID branch.

The new generic wrappers are
`ExperimentalRidgeCaseProjectionV1` (`schema_id:
experimental_ridge_case_projection_v1`) and
`ExperimentalRidgeCaseRuntimeProjectionV1` (`schema_id:
experimental_ridge_case_runtime_projection_v1`). They may reuse existing V2
outcome, crossing, and runtime component types, but each must carry the input
identity plus a freshly recomputed **per-case analytical canary identity**.
They must not carry a fixed embedded-canary identity or overload the embedded
V2 `analytical_report_identity`. The generic path must share the existing
internal evaluation/projector logic, recompute the candidate and outcome from
policy + vehicle + probe, and exact-compare that recomputation with the
emitted projection. Structural self-consistency alone is not sufficient.

The embedded V2 APIs remain stricter exact-golden wrappers around that shared
logic. Their existing schemas, values, and controller-shadow v4 semantic
identity `a69de7872ad039fd` remain unchanged. Any invalid derived geometry
must return a typed, fail-closed error and never panic. H1 adds no held-out
case artifacts and runs no held-out simulation; it may rerun the existing
embedded controller-shadow regression.

#### H2 frozen same-family cases

The two numeric cases are frozen here before any controller outcome is
inspected. Each uses the current canary's policy, vehicle, source and target
pad footprints, initial rest state, pad elevation, transfer span, and single
ridge height/shape. The forward transfer has source `x = 18`, target
`x = 4000`, terrain domain `[-40, 4040]`, and base elevation `y = 0`.

For `rho` in `{0.50, 0.68}`, define `c` as the elevated crest midpoint
`c = 18 + rho * (4000 - 18)` and use exactly these terrain points:

```text
(-40, 0), (c - 125, 0), (c - 75, 1200),
(c + 75, 1200), (c + 175, 0), (4040, 0)
```

The frozen raw coordinates and labels are:

| Label | `rho` | `c` | Raw ridge points `(x, y)` |
| --- | ---: | ---: | --- |
| `ridge_progress_050_probe` | `0.50` | `2009.0` | `(1884, 0)`, `(1934, 1200)`, `(2084, 1200)`, `(2184, 0)` |
| `ridge_progress_068_probe` | `0.68` | `2725.76` | `(2600.76, 0)`, `(2650.76, 1200)`, `(2800.76, 1200)`, `(2900.76, 0)` |

The stable labels belong only to the raw H2 manifest; they are descriptive data,
not report labels, and must never switch algorithms, search bounds, thresholds,
or expected outcomes. The current embedded crest progress is approximately
`0.6045`, so these cases exercise earlier and later obstacle timing while
keeping the same family and geometry scale.

#### Freeze order and evidence ownership

The work is ordered as four explicit gates after this H0 design commit:

1. **H1 — generic input/recompute/projector boundary.** Add the versioned
   input-only boundary, generic provenance, exact recomputation checks, and
   typed fail-closed geometry errors. Prove that the embedded canary remains
   byte- and identity-equivalent, including controller-shadow v4. Do not add
   held-out artifacts or run held-out simulation; the existing embedded
   controller-shadow regression may run.
2. **H2 — analytical seal.** Add a separate source-controlled raw input
   manifest for the two frozen probes. H0 has already frozen the analytical
   predictions; materialize them in the pre-simulation prediction manifest
   before any analytical output is accepted or joined, then run analytical
   evaluation only. Record and bind the manifest, per-case input, per-case
   analytical canary identity, candidate, runtime-projection, and prediction
   identities, verify deterministic repeat, and commit that immutable seal
   before H3. No controller outcome may flow back into inputs, planner
   selection, thresholds, search bounds, or controller tuning.
3. **H3 — full-controller shadow.** Run controller simulations only against
   the sealed identities and join each outcome by those identities. No input,
   candidate, policy, or controller tuning changes are permitted in response
   to an outcome.
4. **H4 — decision review.** Review the sealed analytical and controller
   evidence, then decide whether to advance, narrow, or stop. A pass advances
   only the scoped execution-compatibility claim; it does not authorize
   production planner wiring, broader waypoint counts, D2, or a new topology.

#### H2 analytical predictions and entry gate

Each frozen input is evaluated as a paired flat twin and derived-mesa case. H0
freezes the following predictions. During H2, materialize them in the
prediction manifest before any analytical output is accepted or joined, then
run the analytical evaluation for each pair:

- the flat twin chooses the shortest robust certified `Direct` candidate,
  expected unchanged at `1.0x`;
- the derived mesa has a valid robust blocker and rejects that nominal direct
  candidate for terrain clearance;
- at least one longer direct candidate remains certified only as a global-
  replan diagnostic, not as the selected nominal lane;
- the finite terrain-derived search selects exactly one certified,
  forward-progressing `OneWaypoint` result; and
- the generic candidate and runtime projections validate and repeat
  deterministically, with identities bound to the input and per-case
  analytical canary identity.

Any mismatch, error, or `Unsupported` result stops the experiment and is
recorded as such. The cases may not be moved or resized, and policy, screens,
search bounds, and thresholds may not be changed to recover a failed gate.
`Unsupported` or `unknown` is not a controller true-negative.

#### H3 predeclared controller expectations

The controller-shadow expectations are frozen before H3:

| Lane | Expected result |
| --- | --- |
| flat-direct | Target landing |
| mesa-direct | Non-target terrain contact on the derived mesa before target touchdown; exact time and contact point are intentionally not frozen |
| mesa-waypoint | Canonical waypoint handoff contract passes and the vehicle target-lands |

Any mismatch stops advancement with no retuning. An `Unsupported` or
`unknown` analytical result is an experiment stop, not a controller
true-negative.

#### H1-H4 execution status and H4 decision

The four gates now have a concrete, bounded result:

- **H1 — complete.** The generic input, analytical recompute, and
  runtime-projection boundary is implemented with identity-bound validation
  and typed fail-closed geometry errors. The embedded controller-shadow remains
  schema `v4`, semantic identity `a69de7872ad039fd`, and its three frozen
  outcomes are unchanged.
- **H2a — complete.** The raw-input manifest identity is
  `fnv1a64:19217b5811b4f25f`; the prediction-manifest identity is
  `fd55fc7f3f51c65e`; and the ordered input identities are
  `fnv1a64:1906ea1c9d5b51eb` and `fnv1a64:a63ff1716b1cf24f`. These inputs and
  predictions were frozen before the analytical reveal.
- **H2b — stopped at runtime projection.** The detailed analytical artifact
  identity is `72f664a8f4be8b93`. The tracked compact result
  [manifest](../fixtures/manifests/conservative_ballistic_ridge_heldout_analytical_result_v1.json)
  has identity `24804a74cb303625` and is byte-deterministic. The detailed
  `outputs/eval/conservative-ballistic-ridge-heldout-v1/summary.json` artifact
  and HTML report with inline SVG visuals at
  `outputs/reports/eval/conservative-ballistic-ridge-heldout-v1/index.html`
  are ignored inspection outputs; the detailed artifact is not
  source-controlled.

  `ridge_progress_050_probe` passed every frozen analytical and runtime
  expectation. Its analytical canary, candidate projection, and runtime
  projection identities are `fnv1a64:3892e4cee442675f`,
  `fnv1a64:2405ceb25946ef82`, and `fnv1a64:9e257dace1c02cc2`.

  `ridge_progress_068_probe` passed every candidate-level frozen prediction.
  Its analytical canary and candidate projection identities are
  `fnv1a64:b42288a3164f8223` and `fnv1a64:1ddf250808889d3f`. Runtime projection
  stopped with typed `handoff_contract_failed`, failure identity
  `838b5d883a16f24b`, at the exact analytical-to-runtime waypoint handoff.
  It therefore has no runtime-projection identity and no analytical seal.
- **H3 — not run.** All six held-out controller lanes were intentionally
  withheld after the H2b stop. There are zero held-out controller or simulator
  outcomes, so the scoped execution-compatibility claim does not advance.
  Production wiring, D2-D5, new topology, tuning, controller-limit
  relaxation, and requirement relaxation remain out of scope.
- **H4 — STOP/NARROW.** This is not a ballistic-planner failure: the `068`
  ballistic candidate is certified and forward-progressing. The incompatibility
  is at the exact analytical-to-runtime waypoint handoff contract. The compact
  result archive records this reproducible runtime-projection stop, but is not
  a general serializer for every hypothetical mismatch, error, or
  `Unsupported` result. The exactly-one witness is the first selected result
  from the finite search, not an exhaustive uniqueness proof.

#### F0-F3 runtime handoff-selection development result

The bounded controller-free follow-up is implemented without rewriting the H2
result. Historical generic runtime V1 remains crossing-only and still returns
`HandoffContractFailed` for `ridge_progress_068_probe`. A separate generic
runtime projection V2 retains structured evidence for each attempted exact
state and uses a fixed semantic order:

1. try the existing first exact virtual-anchor crossing; and
2. only if its canonical handoff assessment fails, verify and try the certified
   intermediate-bridge exit at the target-leg acquisition/apex seam.

The selector does not scan bridge ticks, optimize against a contract threshold,
branch on a case ID, alter the route-bisector tangent, or relax any controller
limit. Candidate, crossing, attempt, selection, outcome, and projection
identities bind the complete evidence. A broken bridge/target-leg seam and two
failed semantic attempts remain typed fail-closed results.

For `050`, the primary crossing still passes and V2 carries the exact V1 route,
authority, kinematics, and assessment; V1 identity
`fnv1a64:9e257dace1c02cc2` is unchanged. For `068`, the primary crossing fails
only heading (`0.3879286123780121 rad` versus `0.35 rad`). V2 then selects the
intermediate-bridge exit at applied step `3960`, which is target-leg apex step
`1190`; its heading error is `0.19282817224525745 rad`, cross speed
`12.4226137521142 m/s`, speed `64.8242073388695 m/s`, and authority cap
`93.2925267109143 m/s`, so the unchanged canonical handoff passes.

The separate `conservative-ballistic-handoff-development-v1` evaluator binds
both exposed inputs, historical V1 results, candidate projections, V2 runtime
projections, deterministic repeat, and explicit controller/simulator/physical
non-execution. Its artifact identity is `aeb7338fd71c73e5`. The HTML/SVG report
plots the recomputed source ballistic prefix, exact affine intermediate bridge,
and target ballistic suffix alongside both handoff attempts and their limits.
This is development evidence, not a reopened held-out result or an ordinary
full-route validation claim.

#### F4 full-controller development result

F4 is complete and green as an exposed, development-only full-controller
execution check for `ridge_progress_068_probe`, using the unchanged `120 Hz`
physics / `60 Hz` controller / `180 s` configuration. The versioned artifact is
`conservative-ballistic-handoff-controller-development-v1`, with identity
`8ecd42d0bf113f5d` and `deterministic_repeat: true`. It binds input
`fnv1a64:a63ff1716b1cf24f`, candidate `fnv1a64:1ddf250808889d3f`, runtime V2
`fnv1a64:48e2f9d91222c096`, and selected attempt index `1` /
`fnv1a64:0ee66a27ce706921`. Regenerate it with:

```text
command: cargo run -p pd-eval -- conservative-ballistic-handoff-controller-development
summary: outputs/eval/conservative-ballistic-handoff-controller-development-v1/summary.json
report:  outputs/reports/eval/conservative-ballistic-handoff-controller-development-v1/index.html
```

All three lanes passed launch preflight. `flat-direct` target-landed at
`101.8 s` / `12,216` physics steps with about `3,615.1 kg` fuel remaining.
`mesa-direct` made the expected terrain crash at `28.2916667 s` / `3,395`
steps; reconstructed contact is within the derived mesa at approximately
`(2087.432251, 423.383761) m`, with residual `-36.767780 m`.
`mesa-waypoint` recorded exactly one captured `contract_pass` at
`46.6833333 s`, approximately `(2627.209931, 1965.178136) m`; cross-track
`78.760390 m < 95 m`, outbound cross speed `3.366558 m/s < 20 m/s`, and speed
`40.564173 m/s < 93.292527 m/s`. It then target-landed at `87.1583333 s` /
`10,459` steps with about `4,023.5 kg` fuel remaining.

The exact runtime waypoint remains the planner-produced
`(2715.400408, 1931.456905) m`; the analytical bridge is report context, not
a controller command trace. This is exposed development evidence, not a
held-out result or physical proof, and does not establish universal waypoint
necessity or robustness. No controller, planner, handoff-threshold, or terrain
tuning occurred. F3 identity `aeb7338fd71c73e5` and frozen
controller-shadow v4 identity `a69de7872ad039fd` remain unchanged.

#### F5 held-out experiment design lock (F5a)

F5 is a new held-out experiment, separate from the historical H2/H4 result and
from the F0-F4 development/regression cases. The `050` and `068` cases remain
development inputs and cannot become held out again. F5 keeps the same
gameplay-oriented claim: a conservative analytical route can expose a
nominal-direct blocker and a simple one-waypoint alternative that a real
controller may execute. It does not claim universal waypoint necessity,
physical feasibility, or production readiness.

##### F5 raw cases

Both cases retain the existing policy, vehicle, source and target pads, domain
`[-40, 4040]`, source `x = 18`, target `x = 4000`, initial rest state, pad
elevation, and the same single-ridge shape and height. For `rho` in `{0.56,
0.72}`, define the elevated crest midpoint as `c = 18 + rho * (4000 - 18)` and
use the following exact ridge points:

| Label | `rho` | `c` | Raw ridge points `(x, y)` |
| --- | ---: | ---: | --- |
| `ridge_progress_056_probe` | `0.56` | `2247.92` | `(2122.92, 0)`, `(2172.92, 1200)`, `(2322.92, 1200)`, `(2422.92, 0)` |
| `ridge_progress_072_probe` | `0.72` | `2885.04` | `(2760.04, 0)`, `(2810.04, 1200)`, `(2960.04, 1200)`, `(3060.04, 0)` |

`ridge_progress_056_probe` is an interpolation within the demonstrated
progress range; `ridge_progress_072_probe` is a mild extrapolation. The labels
are data-only identifiers. They must not select algorithms, thresholds,
search bounds, route branches, or expected outcomes.

##### F5 predeclared qualitative predictions

Each raw case is paired with its flat twin and its derived-mesa case. The
following are frozen predictions for each pair before any analytical or
controller result is inspected:

- the flat twin selects the shortest robust certified `Direct` candidate with
  duration multiplier `1.0`;
- the derived ridge is a valid blocker, and the nominal direct candidate is
  rejected for terrain clearance;
- bounded search yields exactly one certified, forward-progressing
  `OneWaypoint` result;
- runtime V2 yields a structurally valid `OneWaypoint` with the selected
  semantic handoff contract passing;
- repeated evaluation is deterministic;
- `flat-direct` target-lands;
- `mesa-direct` contacts non-target terrain on the derived mesa before target
  touchdown; exact contact time and point are intentionally not frozen; and
- `mesa-waypoint` records exactly one contract-pass capture and then
  target-lands.

The prediction deliberately does not freeze whether runtime V2 selects the
`primary_crossing` or `intermediate_bridge_exit` semantic attempt. It also
does not freeze exact analytical or controller times, contact/capture points,
fuel, or derived identities. Those are revealed evidence, not design inputs.

##### F5 phase order and decision boundary

F5 is ordered as five explicit gates:

1. **F5a — design lock.** Commit this documentation-only case and prediction
   definition before reveal.
2. **F5b — source-controlled seals.** The identity-bound raw-input and
   qualitative-prediction manifests are committed. This gate accepts only
   source inputs and predeclared expectations; it must not contain candidates,
   routes, analytical outcomes, controller outcomes, or simulation artifacts.
3. **F5c — analytical reveal.** Evaluate the sealed cases with the existing
   analytical and runtime-V2 path. A mismatch, error, or `Unsupported` stops
   that case; no input, policy, threshold, search bound, or tuning may change
   in response.
4. **F5d — controller reveal.** Run controller lanes only for a case whose
   analytical result has been committed and is eligible for controller
   evaluation. Join by sealed identities; do not feed controller outcomes back
   into the planner or the other case.
5. **F5e — decision.** Record one of these outcomes: **PASS** if both cases
   pass analytical and controller expectations; **NARROW** if one case is useful
   and the other fails or stops; **STOP** if neither demonstrates the scoped
   capability or both share a common failure; or **INVALID SETUP** if
   provenance, scenario, launch, or determinism is invalid. `INVALID SETUP`
   permits wiring repair only, not outcome-guided retuning.

Cases advance independently, but any case mismatch or `Unsupported` prevents
the overall F5 experiment from being green. A full pass authorizes only a
production-integration design review; it does not authorize production
planner wiring.

##### F5b source-controlled seal (complete)

F5b is sealed by `pd-eval/src/conservative_ballistic_f5_seal.rs` and the
source-controlled fixtures
`fixtures/manifests/conservative_ballistic_ridge_f5_inputs_v1.json` and
`fixtures/manifests/conservative_ballistic_ridge_f5_predictions_v1.json`.
The ordered raw input identities are
`fnv1a64:1685aab304642342` (`ridge_progress_056_probe`) and
`fnv1a64:b0dcdcbefac383b6` (`ridge_progress_072_probe`); the input-manifest
identity is `5a654dd8762a1438` and the prediction-manifest identity is
`f3d3ff2ee6e83403`.

The module and fixtures validate only raw input and qualitative-prediction
identity, order, and cross-manifest binding. They contain no derived
candidate, route, or outcome evidence. Historical H2/H4 manifests and results
remain immutable. The F5c analytical reveal is recorded below; production
wiring, runtime replanning, multiple waypoints, new topologies, D2-D5, and
tuning remain deferred.

##### F5c analytical reveal (complete)

F5c evaluated both sealed cases through the existing analytical and runtime-V2
path. The tracked compact result is
[`conservative_ballistic_ridge_f5_analytical_result_v1.json`](../fixtures/manifests/conservative_ballistic_ridge_f5_analytical_result_v1.json),
with identity `6ebab8240feaea3e`; it binds input seal `5a654dd8762a1438` and
prediction seal `f3d3ff2ee6e83403`. The detailed ignored summary/report are
under `outputs/eval/conservative-ballistic-ridge-f5-analytical-v1/` and
`outputs/reports/eval/conservative-ballistic-ridge-f5-analytical-v1/`, with
detailed artifact identity `ae9f3d7f01b1afdd`. The root status is
`one_or_more_cases_stopped` and `deterministic_repeat: true`.

- `ridge_progress_056_probe` passed all eight candidate-comparison checks and
  all five runtime checks, so it is `eligible_for_controller`. Its observed
  runtime-V2 handoff was `primary_crossing`, attempt `0`, with the selected
  waypoint approximately `(1913.111172, 1509.228848) m`. The case identity is
  `3a9878d6c810d8ea`; analytical canary, candidate projection, runtime
  projection, and selected waypoint identities are
  `fnv1a64:98ef17e48984553d`, `fnv1a64:72783b1cf7304dd5`,
  `fnv1a64:a84b90a8c7d65bf3`, and `fnv1a64:78734b090f7b7088`.
- `ridge_progress_072_probe` stopped independently at
  `candidate_projection` / `derived_mesa` with typed `Unsupported` reason
  `finite_waypoint_search_exhausted`; its certified waypoint count was zero.
  The flat `Direct`, valid blocker, and nominal-direct terrain-clearance
  rejection checks passed; only the derived-mesa one-waypoint and exactly-one
  certified forward-progressing-waypoint predictions failed. Its case identity
  is `8ddca8a8fd7b921e`, with analytical canary, candidate projection, and
  waypoint-search identities `fnv1a64:719cba774a6697c7`,
  `fnv1a64:e949f651f543c08e`, and `fnv1a64:16861941f6c7608d`.

This was a mixed analytical checkpoint, not `INVALID SETUP`. It prevented the
predeclared overall F5 PASS/full-green result and restricted F5d to the three
controller lanes for `056`; `072` was not controller eligible. The completed
F5d reveal and F5e decision are recorded below.

##### F5d controller reveal and F5e decision (complete: NARROW)

F5d is implemented by `pd-eval/src/conservative_ballistic_f5_controller.rs`
and the display-only report module
`pd-report/src/conservative_ballistic_f5_controller.rs`. Regenerate the
identity-bound evidence with:

```text
command: cargo run -p pd-eval -- conservative-ballistic-f5-controller
result:  fixtures/manifests/conservative_ballistic_ridge_f5_controller_result_v1.json
summary: outputs/eval/conservative-ballistic-ridge-f5-controller-v1/summary.json
report:  outputs/reports/eval/conservative-ballistic-ridge-f5-controller-v1/index.html
```

The tracked compact result identity is `d08eb8ca50f34561`; the detailed ignored
artifact identity is `ef550886127d079d`, with `deterministic_repeat: true`.
Both bind analytical result `6ebab8240feaea3e`, analytical artifact
`ae9f3d7f01b1afdd`, input seal `5a654dd8762a1438`, and prediction seal
`f3d3ff2ee6e83403`. Only `ridge_progress_056_probe` is in the eligible case
list. `ridge_progress_072_probe` remains
`stopped_candidate_unsupported` / `analytically_ineligible_not_run`, with
`controller_lanes_executed: false` and no controller lane or scenario data.

All three `056` launch preflights passed under the unchanged `120 Hz` physics /
`60 Hz` controller / `180 s` configuration. The selected runtime route remains
`primary_crossing`, attempt `0` / `fnv1a64:4f0ea9194c5a2ba9`, with the exact
waypoint `(1913.111172, 1509.228848) m`.

- `flat-direct` target-landed at `101.8 s`, with about `3,615.1 kg` fuel
  remaining.
- `mesa-direct` crashed at `27.2166667 s`; reconstructed contact is valid and
  lies within the derived mesa near `(1928.256606, 430.527720) m`.
- `mesa-waypoint` recorded exactly one `contract_pass` capture at `33.25 s`
  near `(1821.487138, 1532.208664) m`, then target-landed at `85.7 s` with
  about `4,063.9 kg` fuel remaining.

The first invocation exposed a setup-only validation defect after controller
execution but before any result, output, or report was written: compact lane
identities were incorrectly required to carry the planner-style `fnv1a64:`
prefix even though they use the evaluator's plain-hex `canonical_digest`.
Commit `ea23268` repaired only that representation check. No input, policy,
search, terrain, route, controller, threshold, prediction, or acceptance rule
changed before the successful reveal.

The controller status is `controller_predictions_matched`, so F5e classifies
the overall experiment as **NARROW**: `056` demonstrates the scoped capability,
while `072` stopped analytically. F5 is therefore complete, but it is not a
full-green PASS and does not satisfy the frozen full-pass condition for an
automatic production-integration design review. Any decision to accept a
limited `056`-like operating envelope, freeze another same-family confirmation
case, or stop this capability is a separate product/design checkpoint.
Production planner wiring, runtime replanning, multiple waypoints, new
topologies, D2-D5, physical feasibility claims, and tuning remain deferred.

#### F6 limited integration design lock

F6 turns the already-demonstrated analytical-to-controller path into one
explicit evaluator integration seam. It is development/regression work, not a
new held-out experiment. Its positive input is the existing
`ridge_progress_056_probe` raw input, copied into a single-case F6 fixture with
its existing input identity. `ridge_progress_072_probe` is not an active F6
input or negative test.

The F6 mission is the conservative **derived mesa**, not an arbitrary raw
heightfield. That is the terrain on which the frozen direct-red and
one-waypoint-green controller evidence was established. Supporting an
existing arbitrary mission terrain would require a separate terrain-adapter
contract and new evidence.

##### Integration boundary

`pd-plan` remains the owner of the input-only analytical candidate and runtime
V2 route projection. F6 adds an evaluator-owned adapter that consumes the
generic projection chain and returns one typed derived-mesa decision:

- `supported_direct`, with the selected direct `TransferRouteSpec` and exact
  provenance;
- `supported_one_waypoint`, with the selected one-waypoint route, handoff
  selection, and exact provenance; or
- `unsupported`, retaining the finite-search reason and rejection reasons.

Malformed input, identity mismatch, invalid runtime evidence, scenario/input
mismatch, or cadence mismatch is an error. A valid bounded-search exhaustion
is `unsupported`, not an error and not evidence of physical infeasibility. An
unsupported or invalid F6 lane stops before controller execution. Fallback is
owned by a future caller; F6 does not silently retain an authored route or
invoke the ordinary planner.

The adapter must bind its decision to the input, analytical projection,
runtime projection, selected outcome/candidate, canonical route, topology,
waypoint count, derived-mesa identity, and explicit disposition. It may inject
the supported route into `ScenarioSpec::mission.transfer_route` only through a
new opt-in F6 evaluator lane. It must not emit `RoutePlan`,
`RoutePlanProvenance`, or claim compatibility with ordinary full-pad
`pd_core::validate_route`.

##### V1 applicability and invariants

F6 deliberately accepts the narrow contract already exercised by `056`:

- forward `source` to `target` transfer, with those exact V1 pad IDs;
- vehicle at rest and upright on the source pad with zero angular rate;
- one conservative derived mesa and at most one selected waypoint;
- scenario physics cadence equal to the analytical policy cadence and the
  frozen controller-shadow controller cadence and mission-time bound; and
- a structurally valid runtime-V2 route whose selected waypoint attempt passes
  the canonical handoff contract.

These are explicit applicability checks, not case-ID switches or newly tuned
distance/altitude thresholds. The analytical certificate remains the only
eligibility decision. `pd_plan::plan()`, its default behavior and identities,
`pd-core` route validation, `pd-control`, controller parameters, and all F5
fixtures/results remain unchanged.

##### F6 execution and exit gate

F6 is ordered as four committed checkpoints:

1. **F6a — disposition and design lock.** Preserve F5 history, retire `072`
   from future gates, and commit this contract before implementation.
2. **F6b — typed adapter.** Add the single-case fixture, typed decision and
   provenance, strict applicability validation, deterministic identity, and
   fail-closed tests without executing a controller.
3. **F6c — integrated controller evidence.** Resolve the `056` derived-mesa
   route through the adapter, inject that exact route into the scenario, run
   deterministic controller evidence, and render a visual report. Control
   lanes may be retained for interpretation, but the selected waypoint lane
   must bind to the adapter decision rather than reconstructing it.
4. **F6d — closure.** Confirm ordinary planner non-regression, complete
   workspace tests/lints/formatting, record the result, and decide only whether
   the opt-in development seam is sound. Enabling a default production route
   source or expanding the mission family remains a later product checkpoint.

F6 passes only if the adapter deterministically selects the one-waypoint route,
no controller runs on unsupported/invalid setup, the injected `056` route
captures exactly one passing waypoint contract and lands on target, the
interpretive flat/direct controls retain their established outcomes if run,
and the ordinary planner remains unchanged.

#### Same-family invariant and explicit deferrals

Both cases remain forward source-to-target transfers with one contiguous
elevated ridge feature, flat pad footprints, and the vehicle initially at rest
on the source pad. The following are deliberately outside H0/H1-H4:

- reverse direction (current validation is forward-only; reverse is new
  capability work);
- width or height transforms, which are likely to clip into a no-op or become
  an authored blocker rather than an informative held-out variation;
- span or long-range changes, payload or vehicle-radius changes, and
  `Unsupported`-targeting cases;
- a second topology, runtime replanning, broader waypoint counts, and
  production planner wiring; and
- generic source-departure D2 and D4-D5.

A second topology is deferred, not rejected forever. The production planner
already retains direct, single-ridge, and double-ridge topology evidence, so
another ridge alone would not settle the planner/evaluator ownership boundary.
Terminal-energy or approach-shaping cases require new candidate-generation
concepts rather than a small variation of the current terrain-derived search.
Mission-matrix expansion, runtime replanning, broader waypoint counts, and
production planner selection remain later work.

## Research and expansion boundary

The existing bounded trajectory witness and W1-W4 artifacts remain research
history. D2 remains blocked because the finite proposal backend did not provide
useful physical coverage; the V2 direct certificate does not repair or promote
that result.

Only after a useful bounded waypoint capability is demonstrated should the
project consider two-waypoint search, target craters, source-side walls, broad
mesas, payload/radius variation, or larger mission matrices. Those expansions
must measure useful game routes, not resurrect the rejected fixed-gate proof.

## Stop rules

Stop or narrow the work when:

- a current neutral probe is relabeled red merely to justify waypoint work;
- a direct-red/one-waypoint-green pair cannot be built without per-case
  thresholds or authored route goldens;
- fixed altitude/distance gates reappear as route-necessity evidence;
- the analytical model needs controller IDs, phases, or outcomes;
- accepted routes regress maintained landings once simulation validation opens;
- the flat control cannot establish an ordinary target landing;
- a powered analytical join is silently treated as a runtime spatial capture;
- search becomes unbounded or requires more than the declared waypoint bound; or
- repeated outcome-guided retuning is needed to make the capability appear
  useful.

The target remains one conservative path that could work in the game, with a
clear evidence boundary around what the analytical certificate does and does
not establish.
