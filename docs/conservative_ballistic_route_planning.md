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

The analytical waypoint near `(1983.6, 1301.1) m` still does not map to a valid
existing `TransferRouteSpec`: the endpoint-shaped source leg intersects
terrain near `x = 105.6 m`. No waypoint-controller lane is run after that
validation failure. The full nominal-direct-red/one-waypoint-green controller
claim therefore remains open even though mesa-caused direct failure is now
established.

The shadow is generated with
`cargo run -p pd-eval -- controller-shadow`. Its sealed summary and per-lane
run artifacts live under
`outputs/eval/conservative-ballistic-controller-shadow-v1/`; the HTML/SVG
report lives under
`outputs/reports/eval/conservative-ballistic-controller-shadow-v1/`.

## Plan from here

The V2 analytical contract and corrected direct controller pair are now frozen.
The next work is bounded and staged:

1. Define a small shared analytical-to-runtime route adapter. The analytical
   anchor is a virtual leg junction, while the certified powered intermediate
   bridge passes over it; the adapter must target states actually traversed by
   the witness and represent the powered source/intermediate joins honestly.
2. Reconcile that mapping with the existing source-departure/tracking-entry and
   runtime route-validation contracts. Inflating a capture radius, lowering
   terrain clearance, bypassing validation, or copying a winning command trace
   remains out of scope.
3. Run only the frozen mesa one-waypoint lane after the adapter validates. It
   must pass the waypoint contract, clear the derived mesa, retain fuel, and
   land on the target; otherwise record the exact mismatch without tuning.
4. Extract a private planner-facing candidate API only after the controller
   shadow has both direct-red and waypoint-green evidence. Consider an
   additional topology later, and only when it exercises a materially
   different analytical failure.

No arbitrary waypoint count, production planner wiring, controller-specific
branch, or simulated-pilot claim is part of this checkpoint.

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
