# Airborne arrival execution

This record implements the [authorized plan](airborne_arrival_plan.md).
No RL training or default promotion is included.

## Allocation

- Evidence root: `outputs/airborne-arrival/loop-20261023T222023Z`.
- Original source and foreign-file hashes: `workspace-baseline.json` there.
- Start: Unix 1792793951; outer deadline: 1792804751.
- Maximum native/model work: 150,000,000 steps including forecasts and proofs.
- Maximum originals: 6,000; new evidence: 16 GiB; free-space floor: 40 GiB.
- Native workers: one; optimizer steps: zero.

The implementation and cost pilot precede the frozen full cohort allocation.
Results and any capability limitations will be added here with exact receipts.

## Cost pilot freeze

The new selector is `poly_native_first_feasible_coast_duration_grid_v1`:
first native target touchdown in coast-major order, coasts 0, 0.5, 1, 2, 3,
4 and 6 seconds; burn durations 6, 8, 10, 12, 16, 20, 24, 30 and 36 seconds.
This is neither the old planner's timing algorithm nor a fuel optimizer.

The fixed arrival bank uses positions at 35% or 60% of the entry-to-pad
displacement, with speeds 40%, 70% or 100% of entry speed, floored at 12 m/s.
Velocity points along that displacement. Radius/cross-track tolerance is 30 m;
speed tolerance is ±15%, heading tolerance 0.25 rad, cross-speed tolerance 25%,
and vertical speed tolerance ±15% of the chosen speed. Arrival deadline is
absolute 30 s. New missions retain an original 90 s deadline and upright entry.

The pilot takes four input-defined archetypes, angles 0/30/60/80 degrees with
low/mid/high energy and clean/±75 m discrete ballistic misses represented.
Each compares direct landing with the fixed `p35-v70` goal. Six fixed captures
include all four direct controls and arrival cases 0 and 2, regardless of
outcome. Maximum pilot work is 20 million native/model steps, five minutes and
1 GiB additional artifacts. Its resource costs release the full allocation.
The runtime JSON fixes exact inputs, source epoch and library before flights.

The first pilot completes all eight Summary flights, then stops on a missing
capture-parent directory before any capture is saved. The flight results are
retained in `pilot/summary.json`; this is a harness fault, not a physical miss.
Epoch B creates the parent before flight, charges capture work before saving,
and validates allocation input/library hashes. It changes no controller,
mission, goal or selection logic. The identical pilot repeats in `pilot-b/`;
its Summary fingerprints must equal epoch A. The first attempt carries
1,601,059 counted steps plus an 18,000-step conservative reservation for the
failed capture and three neutral passes, for 1,619,059 carried steps.

## Full characterization freeze

Epoch B completes the identical eight Summary flights and all six fixed proofs:
all eight compact results and fingerprints equal epoch A. Pilot B uses
5,694,787 native/model steps, 24,442,953 capture/result bytes and 1.27 seconds;
its rich report adds 11,760,631 bytes. The cost release reserves 110 million
steps and at most 70 captures for the full phase, plus 30 million steps for a
separately frozen fresh panel. These reservations plus both pilot attempts fit
the original 150-million-step bound.

The core is the full 4 angle × 3 energy × 3 ballistic-miss Cartesian product:
0/30/60/80 degrees, low/mid/high, and −75/0/+75 m. Entry radius is 800 m;
directions alternate by case index. Every one of these 36 originals compares
direct landing and `p35-v70`. Twelve input-defined cells, one per angle/energy
with miss index `(angle_index + energy_index) mod 3`, get all six arrival goals.
Their nine nonvertical reflected partners compare direct and `p35-v70` as
paired diagnostics, giving 45 physical inputs and 150 Summary occurrences.
Reflections are not independent confirmations. Fuel and vehicle remain fixed.

Fixed proof roles are both direct and `p35-v70` on all twelve bank cells;
add the best successful eligible choice on each input, requiring arrival
fulfillment when an arrival is used. Duplicate roles share a capture. Native
Summary headroom and fully verified best choices remain distinguishable.
No goal bank, tolerance, tilt limit or landing selector changes after outcomes.

## Fresh comparison freeze

Every best eligible full-phase choice is direct landing. Freeze the input rule
to direct landing, with no learned selector. A fresh panel contains 24
independent hash-stream inputs: two per angle/energy cell, radius jitter ±15%
and continuous ballistic miss in ±100 m. Generator key is `arrival-fresh-v1`,
seed 231024; no outcome admission/filtering. All compare the frozen direct rule
with `p35-v70` as a separately labelled diagnostic. Fixed captures cover all
24 direct runs and the first arrival run at each angle, regardless of outcome.
The phase receives 30 million native/model steps and at most 28 captures.
This is a comparison of frozen controllers/rules, not an RL generalization test.
