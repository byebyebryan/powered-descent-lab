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
