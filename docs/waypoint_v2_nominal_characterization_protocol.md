# Waypoint V2 nominal characterization protocol

This checkpoint tests the proposed shared nominal mathematics without changing
any live planner policy. It binds eight clear starts, 27 actual local handoffs
and twelve historical airborne captures. Retained snapshots are comparison
values only: every physical starting state is reconstructed from a fresh
simulation and its original complete command prefix. Synthetic conditions are
separate, explicitly labeled scenarios, not restored physical captures.

## Fixed research family

Use the original 120 Hz physics and 60 Hz held-command clock, fuel and absolute
deadline. Grounded rows first launch upright, at full thrust, until COM height
exceeds the source plane by the body's rotation radius plus 10 m and vertical
speed exceeds gravity times a quarter-turn duration. Limit this adapter to 240
ticks. It is experimental source preparation, not the old launch schedule or a
new runtime policy. Airborne rows do not receive this adapter or source exception.

The shared kernel first offers zero acquisition, using the natural descending
crossing as its virtual time when that crossing exists. It then considers four
virtual-arrival times: the natural descending crossing of target COM height,
the greater of that time and sqrt(2 * lateral distance / gravity), and 1.25/1.5
times that latter time. Three powered acquisition fractions are 0.25/0.45/0.65.
Duplicate seeds are removed; there are at most thirteen seeds, below the
proposed sixteen-seed cap. Natural-profile acquisitions have zero vertical
thrust; other profiles may add upward shaping, never obstacle-driven loft.
The natural crossing is optional: an unpowered trajectory may not reach an
uphill target's height. In that case omit its powered natural-profile seeds,
use the lateral geometry time as the baseline, and still examine the three
upward-shaping times. Zero acquisition is still offered with that baseline
virtual time; the entry screen can reject it. Lack of a natural crossing alone
is not grounds to skip powered acquisition.
Forward at acquisition end means toward the target from the current nominal
attempt's incoming position, not along an archived source-to-target route.
Acquisition may brake and reverse initially opposed lateral motion; its end
must move toward the target. Terminal capture may not overshoot and return.
An overshoot condition may therefore produce a valid bounded recovery. Synthetic
conditions have no predetermined landing outcome. This does not add backward
local-clearing support to the unchanged V2 loop.

For each seed, solve one constant vector of thrust acceleration that makes
the acquisition followed by gravity-only coast reach the virtual target:

```text
ballistic_end = p + v * N * dt + gravity_vector * dt^2 * N * (N + 1) / 2
gain = dt^2 * burn_ticks * (N - turn_ticks - (burn_ticks - 1) / 2)
thrust_acceleration = (target - ballistic_end) / gain
```

Use at most three updates for the coast-turn delay, rounding onto complete
held pairs. Nonconvergence is a finite model miss, not impossibility. Reject
nonfinite demands, inadequate time/fuel, and demand above 0.925 of incoming
max-thrust acceleration. This factor is the existing 0.075 robustness reserve;
the additional historical source derate of 0.81 is not silently imported into
airborne acquisition. These are research choices, not calibrated safety limits.

At a rounded natural crossing, explicitly retain zero vertical thrust instead
of repairing the small virtual-height discrepancy introduced by tick rounding.
Record that discrepancy. Reject backward velocity at the acquisition end in
this forward-flight study. The constant segment's position and velocity must
agree with the existing affine-bridge coefficients, with zero acceleration
increment; this is a mathematical comparison, not a second search family.

## Entry and executable checks

For each seed, examine at most four coast fractions: 0, 0.25, 0.5 and 0.75 of
the remaining virtual flight. An entry must be descending and above target COM
height plus the 5 m nominal reserve. The terminal time is computed from entry
height and speed, not chosen from the old seven-factor grid:

```text
T = (2 * height + (down_speed - target_down_speed) * dt)
    / (down_speed + target_down_speed)
```

This is the constant vertical net-braking duration under velocity-first
integration. It supplies a seed to the unchanged quadratic-horizontal,
affine-vertical terminal reference. Require sufficient coasting time to align
its first thrust, no nominal lateral reversal, and a conservative coupled
thrust bound. Minimum throttle and physical attitude/contact behavior still
need executable checks. There is no invented positive global angle threshold:
the angle is measured at actual terminal entry, while admissibility comes from
descending geometry and combined capture demand. Report angle distributions.

The virtual ballistic arrival and the powered terminal arrival are different
clocks. Braking may extend the latter. Check the actual new finish against the
original deadline, not just the shorter virtual intercept. Reject reference
lateral reversals using acceleration-root extrema; a long terminal duration
must not solve the lateral endpoint by overshooting and returning.

Rank estimates by preservation of vertical motion, upward acquisition impulse,
estimated fuel, finish time and stable seed identity. Interior terrain and
archived suffix outcomes are never rank inputs. Materialize at most three
shortlisted supplied-command witnesses per row. Use the existing paired-throttle
inverse, rate-limited plant and terminal implementation. Replay every realized
program independently from the actual incoming live state. Audit real terrain
separately; a free-space target-plane witness is not a complete terrain flight
or a new mission landing result.
Report raw target contact separately from phase-specific body-reserve violations.
Only ground prep may use the source exception; acquisition and coast do not.
Terrain audit measurements must not feed seed selection or ranking.

## Measurements and exit gate

Record all seed/screen reasons, finite acquisition position/velocity/fuel errors,
estimated and actual entry, command identity, independent replay agreement,
target-plane witness and real-terrain audit. Include labeled safe-descent,
steeper arrival, lateral miss, shallow/high-energy entry, uphill target, low fuel,
large turn and overshoot conditions. Do not require every condition to land.

The checkpoint passes only with all 47 retained inputs accounted for, honest
finite failures, verified live prefixes, repeatable deterministic payloads,
targeted negative/one-sided tests, unchanged frozen inputs/evidence, and a
separate primary review. The review must either freeze a justified runtime
contract or explicitly reject this family and name the remaining model/backend
gap. A favorable four-handoff result cannot substitute for the future 32-case
mission gate. No runtime policy integration, commit, push or default promotion.
The sealed corpus has SHA-256
`c570da97f46cdaa5daf70238964d52afaad663107f42e5ef3205a7d186a7c128`.
The evaluator requires these exact builder-emitted bytes as well as checking
all 107 original source bindings before and after each run. It must not accept
unrelated embedded rows merely because their attached source hashes are valid.

## Research cross-check

NASA/JPL's [powered-descent guidance constraint summary](https://ntrs.nasa.gov/citations/20130009793)
distinguishes thrust magnitude, pointing, speed and ground-avoidance constraints;
its [minimum-landing-error summary](https://ntrs.nasa.gov/citations/20120001230)
also treats fuel and target feasibility separately. These support checking more
than a terminal angle. They do not validate this game's finite seed family,
reserve factor or executable backend, and do not require adopting a convex
optimizer for this checkpoint. The local plant and reproduced commands remain
the authority for this study's physical results.
