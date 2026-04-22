# Terminal Suite Design

This document is the maintained design reference for the terminal-guidance
evaluation suite.

It is intentionally not an implementation checklist. The point is to keep the
suite model, coverage philosophy, and selector structure explicit as the lab
evolves, because scenario and eval design directly shape controller
development.

## Purpose

The terminal suite is the main controller workbench in `pd-lab`.

It should answer:

- can a controller land reliably across the intended terminal arrival space?
- where inside that space does it become fragile?
- how much spread exists between seeds within the same physical case?
- how does a new controller compare against the current baseline on the exact
  same resolved runs?

This is why suite design is a first-class artifact, not just a byproduct of
implementation.

## Design Principles

### Physical case first, controller lane separate

The physical test case and the controller under test should not be the same
selector axis.

The suite should separate:

- physical-case selector
- controller lane

That way the same resolved case can be run through:

- `baseline`
- `current`
- future controller lanes

without inventing different scenario identities for each controller.

### Matrix for arrival space, hierarchy for scenario class

Terminal guidance is not a pure tree and not a pure flat list.

Use hierarchy for the class of case:

- `mission`
- `arrival_family`
- `condition_set`
- `vehicle_variant`

Use a dense matrix for the arrival space inside that class:

- `arc_point`
- `velocity_band`

Use `seed` only for small local variation inside a matrix cell.

### Seed variation must stay small

Inter-seed variation is itself a core metric.

That means seed jitter should be strong enough to perturb the control sequence,
but weak enough that the controller is still solving the same physical case.

If a perturbation changes the qualitative reaction too much, it does not belong
in the seed policy. It belongs in:

- a different `condition_set`
- a different `vehicle_variant`
- or a different arrival matrix cell

## Selector Model

The intended resolved selector shape is:

- `mission=terminal_guidance`
- `arrival_family=half_arc_terminal_v1`
- `condition_set=clean`
- `vehicle_variant=nominal`
- `arc_point=a30`
- `velocity_band=mid`
- `seed=0042`
- `controller=baseline_v1`

Interpretation:

- hierarchy axes tell you what class of terminal case this is
- matrix axes tell you where inside the arrival space the case sits
- seed tells you which small local variation was applied
- controller tells you which lane ran that case

## Arrival Family

The first real arrival family should be:

- `half_arc_terminal_v1`

This replaces the current provisional seeded perturbation families.

Reference geometry preview:

![Half Arc Terminal V1](assets/terminal_suite/half_arc_terminal_v1.svg)

Regenerate with:

```bash
uv run scripts/render_terminal_suite.py
```

### Arc points

For symmetric clean terminal cases, top-level matrix cells should not duplicate
both left and right sides.

Instead:

- define one-sided unsigned arc magnitudes
- resolve side deterministically from the seed
- record the resolved side in per-run parameters

Recommended first arc-point set:

- `a00`
- `a15`
- `a30`
- `a45`
- `a60`
- `a72`
- `a84`

This intentionally reduces the clean half-arc suite to seven one-sided points.
That is dense enough to expose controller shape without turning the first matrix
into a needlessly fine grid.

These represent angle-from-vertical magnitudes, not full signed left/right
positions.

Exact `90°` should be avoided in the first suite because it behaves more like a
pathological edge than a useful core coverage point.

`a00` is a deliberate vertical reference case. It should remain in the first
suite, but it is the one cell where side resolution is ignored because left and
right are not physically distinct there.

### Radial distance

The first half-arc terminal suite should use one family-level nominal radius,
not radius as another top-level matrix axis.

Recommended first policy:

- `radius_nominal = 800m`

Rationale:

- old `pylander` terminal scenarios typically started fairly far out:
  - `700..900m` radius
  - about `10..12s` time-to-go
- that gave the controller enough room to react, which is still the right
  design lesson
- but carrying radius as another primary matrix axis in `pd-lab` would make the
  first terminal matrix noisier and harder to interpret

So the first `half_arc_terminal_v1` family should:

- keep radius fixed at the family level
- define velocity bands around that nominal radius
- reserve radius variation for small seed-level radial jitter in the clean case

That keeps `arc_point x velocity_band` readable while still leaving enough
reaction room for the controller.

### Velocity bands

The first suite should use three velocity bands:

- `low`
- `mid`
- `high`

These should be defined relative to a nominal feasible arrival for the chosen
arc point, not as arbitrary independent `vx`/`vy` bins or simple raw speed
multipliers.

The preferred definition is:

1. choose the start point from `arc_point` at the family nominal radius
2. choose a target time-to-go band for that point
3. solve the initial velocity that reaches the target under gravity in that
   time

So the bands are really arrival-style bands, not just scalar speed buckets.

Recommended interpretation:

- `low`: longer time-to-go, lower-energy arrival
- `mid`: nominal time-to-go
- `high`: shorter time-to-go, more aggressive arrival

Suggested first policy:

- `mid`: the exact nominal time-to-go from the family table for that arc point
- `low`: `+12.5%` time-to-go from nominal, unless that would force an
  upward-starting vertical velocity for the cell
- `high`: `-12.5%` time-to-go from nominal

To keep the family implementation unambiguous, `half_arc_terminal_v1` should
carry an explicit `nominal_ttg_by_arc_point` table. `mid` means that exact
table entry, not an implementation-specific heuristic.

Recommended first `nominal_ttg_by_arc_point` table:

- `a00 = 10.50s`
- `a15 = 10.50s`
- `a30 = 10.25s`
- `a45 = 10.00s`
- `a60 = 9.75s`
- `a72 = 9.50s`
- `a84 = 9.00s`

These values are deliberately conservative at the shallow end so the `low` band
still represents a descending arrival, not an upward-starting lob.

Band derivation rule:

- compute the cell start point from:
  - `x = radius_nominal * sin(angle_from_vertical)`
  - `y = radius_nominal * cos(angle_from_vertical)`
- let `t_flat = sqrt(2 * y / g)` be the zero-initial-vertical-speed flight time
  to the target height under gravity
- derive:
  - `mid = nominal_ttg_by_arc_point[arc_point]`
  - `low = min(mid * 1.125, t_flat * 0.98)`
  - `high = mid * 0.875`

The key is that the band should preserve the same arrival geometry while
changing the margin.

## Side Handling

For `clean` and other symmetric early conditions:

- side should not be a top-level matrix axis
- side should be resolved deterministically from the seed
- both controller lanes must see the same resolved side for the same seed

The one exception is `a00`, where side resolution should be ignored because the
case is vertically centered.

This keeps the matrix dense without wasting half the cells on mirrored cases.

For later asymmetric conditions, such as terrain or obstacle cases:

- side may need to become explicit
- or the condition itself may encode a directional asymmetry

## Seed Policy

Seed is for small local variation within a cell, not for defining the cell.

### Clean-case seed policy

For the first `clean` terminal suite:

- always resolve `side_sign` from seed
- then apply exactly one of:
  - `radial_jitter`
  - `speed_jitter`
- never apply both in the same clean seed
- do not add:
  - heading jitter
  - fuel jitter
  - mass jitter

This keeps inter-seed standard deviation meaningful instead of inflating it with
stacked disturbances.

### Clean seed schedule

The first full tier should not repeat a five-state cycle. It should use a
canonical twelve-seed schedule so the spread metric is based on a balanced set
of small but distinct nuisance variations.

For non-vertical cells, the canonical side rule should be:

- even seed index: `left`
- odd seed index: `right`

That makes side resolution deterministic and portable across implementations
without introducing a separate randomization policy. For `a00`, the resolved
side should be ignored.

Recommended deterministic full schedule:

- `seed 0`: left, radial `r1` positive
- `seed 1`: right, radial `r1` negative
- `seed 2`: left, radial `r2` positive
- `seed 3`: right, radial `r2` negative
- `seed 4`: left, radial `r3` positive
- `seed 5`: right, radial `r3` negative
- `seed 6`: left, speed `s1` positive
- `seed 7`: right, speed `s1` negative
- `seed 8`: left, speed `s2` positive
- `seed 9`: right, speed `s2` negative
- `seed 10`: left, speed `s3` positive
- `seed 11`: right, speed `s3` negative

Smoke should use a small representative subset of that canonical schedule. It
is intentionally a fast sanity tier, not a symmetry-complete spread probe.

Recommended smoke subset:

- `seed 0`
- `seed 1`
- `seed 6`

### Magnitude guidance

Keep clean-case magnitudes intentionally weak:

- radial amplitude levels:
  - `r1 = 1.5%`
  - `r2 = 3.0%`
  - `r3 = 4.5%`
  - with a hard cap of about `30m`
- speed amplitude levels:
  - `s1 = 1.0%`
  - `s2 = 2.0%`
  - `s3 = 3.0%`
  - of nominal speed for that band

The exact values can be tuned after the first matrix lands, but the suite
should preserve this narrow-variation philosophy.

## Coverage Tiers

The terminal suite should support two practical seed tiers:

- `smoke`
  - small seed count for quick controller iteration
  - recommended first count: `3`
- `full`
  - broader seed count for meaningful spread measurement
  - recommended first count: `12`

The same physical matrix should support both.

For the first clean matrix, that implies:

- `7 arc_points`
- `3 velocity_bands`
- `3 seeds` for smoke
- `12 seeds` for full

## Condition Sets

The first condition sets should be:

- `clean`
- `traj_err_small`
- `traj_err_large`

These belong above seed in the hierarchy because they intentionally change the
kind of problem being solved.

Later conditions can include:

- `terrain_obstacle_low`
- `terrain_obstacle_medium`

but those should only come after the clean matrix is real.

## Vehicle Variants

The first vehicle variants should be:

- `nominal`
- `low_margin`

Later variants can include:

- `heavy_cargo`
- `low_fuel`

Again, these are distinct case classes, not seed jitter.

## Expectation Tiers

Each suite bucket should carry an explicit expectation tier:

- `core`
  - should normally succeed
- `stress`
  - hard but intended to be informative
- `frontier`
  - may be infeasible; crash avoidance matters more than 100% success
- `reference`
  - not part of the main landing lane comparison

This keeps the suite honest about what counts as regression versus difficult but
expected variation.

## Reporting Expectations

The batch report should eventually present the terminal suite as:

- hierarchy:
  - `mission`
  - `arrival_family`
  - `condition_set`
  - `vehicle_variant`
- then matrix structure:
  - `arc_point`
  - `velocity_band`
- then lane:
  - `current`
  - `baseline`
- then seed detail

The current tree/table report is acceptable for the first implementation pass.
There is no need to build a new matrix-specific UI before the underlying suite
is real.

## Current Status

Right now the terminal bot-lab suite is still provisional:

- it approximates the intended selector model with seeded perturbation families
- metadata already carries the selector fields
- the next real step is to make the execution model match the documented suite
  design

## Next Implementation Target

The next concrete milestone for the terminal suite is:

1. implement `half_arc_terminal_v1` in `pd-eval`
2. make `terminal_bot_lab_suite` the smoke matrix
3. add `terminal_bot_lab_full` with the same matrix and a larger seed count
4. thread `arc_point` and `velocity_band` through the batch report tree

Only after that should the suite expand into richer condition sets or more
polished report UX.
