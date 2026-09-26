# Waypoint direct-route characterization

`waypoint-direct-characterization` is an opt-in, deterministic diagnostic for
the existing waypoint-planning seam. It records the ordinary `pd_plan::plan()`
decision, the exact current direct-chord corridor query, and (when the setup is
supported) an observation from the unchanged `transfer_pdg` controller. The
artifact is evidence for the baseline mismatch; it is not a new planner lane.

Run it with:

```text
cargo run -p pd-eval -- waypoint-direct-characterization \
  --output-dir /tmp/pd-direct-characterization
```

The command writes `summary.json`, `report/index.html`, and
`report/preview.svg`. Without `--output-dir`, the opt-in bundle is written
under `outputs/setups/waypoint-direct-characterization`; no ordinary report or
planner selection is changed.

## Frozen rows

The rows are generated in this order and use one shared vehicle, policy,
controller, radius (`800 m`), pad width, simulation cadence, and fixture
construction protocol:

1. continuous flat `r00`
2. continuous uphill `r+30`
3. continuous downhill `r-30`
4. bounded narrow-ridge control
5. bounded broad-mesa control

The first three use the existing continuous transfer terrain helper. The last
two add fixed bounded terrain controls to the same pad geometry. There is no
per-case planner, controller, or policy tuning.

## Evidence boundary

- **Direct** means zero operational handoffs. Each row is one route composed
  of one leg.
- Future leg profiles and certificates are separate concepts from
  `TransferRouteSpec` waypoints. This checkpoint does not choose or implement a
  V2 profile family.
- Planner acceptance/rejection is preserved as the typed V1 result. The
  direct-chord clearance is computed with the same public core profile and
  exact corridor query used by the planner.
- Controller success is a separate claim. A retained green trajectory is an
  observed `transfer_pdg` trace, not a planner path, selected route, or
  certificate. The report labels global hull clearance as touchdown-inclusive;
  en-route hull clearance is the minimum sampled observation between the
  endpoint profile's source-transition-end and target-transition-start
  progress bounds. Both are sampled observations, not continuous-time
  guarantees.
- The existing conservative-ballistic/direct-bridge analytical result is
  reported as a separate `not_mapped` lane because this checkpoint does not
  adapt its fixture or policy.

The report intentionally makes the baseline mismatch visible: ordinary V1
planner acceptance/rejection and controller outcome can disagree. The command
does not alter `pd-core`, `pd-plan`, planner defaults, controller behavior or
configuration, F6 code/fixtures, or any default report selection.
