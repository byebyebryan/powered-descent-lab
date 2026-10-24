# Arrival actuation calibration and qualification

The authorized pass corrects physical command allocation for an opt-in airborne
arrival backend, then measures what that correction changes. Existing terminal,
transfer, waypoint and Poly Landing defaults retain their commands and identity.
See the [previous characterization](airborne_arrival_execution.md) and the
[execution record](arrival_actuation_execution.md).

## Frozen contract

Keep the same one-pass waypoint capture predicate, plane-miss termination,
absolute operation deadline, local horizon/recovery rules, tilt bound and
deterministic Poly continuation. The planner supplies an optional arrival goal;
the controller owns physical commands. Admission accepts an attempt and does
not certify reachability.

The opt-in allocation profile keeps the existing directional/authority
projection, then maps its desired thrust acceleration to the native held engine
setting over two 120 Hz physics steps. Use the existing mass-aware paired
inverse. For demand below minimum thrust, select the closer aligned mean
acceleration from engine off or minimum thrust, breaking ties toward off.
Record projection error and engine switching. No hidden duty cycle or new
hysteresis is introduced. Insufficient fuel for a complete powered pair is
reported explicitly; a conservative fuel cap must not be called physical
infeasibility. Rotation remains rate-limited by the original plant.

The same profile applies to local candidate forecasts and live commands.
Forecasts retain their existing integration cadence and finite horizon; they
are predictions, not complete native feasibility certificates.

## Evidence gates

1. Calibrate zero/minimum/interior/maximum demand, changing mass, scarce fuel,
   and aligned/limited-rotation cases against native command pairs. For aligned
   realizable demands, require mean acceleration error below 1e-8 m/s².
2. Preserve legacy command/state fingerprints on a fixed retained panel and
   keep ordinary maintained controller checks passing.
3. Construct 12–24 independent native witness trajectories for vertical motion,
   mild lateral motion, braking and modest attitude preparation. Demonstrate
   each goal under the unchanged fly-through predicate and deadline. Report
   construction exclusions separately; witnesses are controlled diagnostics.
4. Run legacy and corrected controllers on those witness-backed goals. Replay
   witness commands independently and distinguish physical reachability from
   backend fulfillment. A missed witnessed goal remains a controller gap.
5. Compare the corrected profile on the unchanged 198 retained core/reflection/
   exposed-fresh Summary programs. Retained results remain the legacy baseline;
   the formerly fresh inputs are now exposed regression inputs.
6. Selected full captures need physical/mission/integrity/replay proof,
   controller reconstruction, goal-rule reconstruction and Summary equality.
   Extend the established rich tree without altering old reports or services.

Finish with calibrated allocation and an honest bounded qualification result.
Remaining witness failures stop advancement to RL; do not relax tolerances or
tune individual goals to manufacture a pass. Timing optimization, horizon or
recovery tuning, tilt changes, arbitrary state contracts and RL are separate
future passes.

## Resource boundary

Two hours from Unix 1792807909, 100 million campaign native/model steps
including forecasts, witnesses and proof, 1,000 original occurrences, 64 full
captures, 4 GiB new artifacts and a 40 GiB free-space floor. One native worker;
zero optimizer steps. Freeze exact phase schedules and capture roles before
their outcomes, after a small cost pilot. Local commits are authorized; push
and default promotion are outside the pass. Software/unit checks are separate
from measured campaign counters.
