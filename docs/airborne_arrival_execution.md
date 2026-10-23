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

## Results and interpretation

The full phase completes all 150 Summary occurrences and 55 selected full
captures. It uses 46,466,395 native/model steps. The 36-input core has 34 verified
direct Poly target landings. Across its 96 arrival attempts, 21 fulfill the
arrival contract and 19 then have native Summary target success. The nine
paired reflections have eight verified direct landings and one fulfilled
arrival among nine attempts. Their sample sizes are not independent additions
to the core denominator.

The core's twelve finite-bank cells expose no eligible fuel improvement or
recovery of a direct miss. Many successful final landings follow failed arrival
instructions: 58 core Summary outcomes do this. Local execution and composition
must therefore remain separate gates.

The fresh panel completes 48 Summary occurrences and all 28 fixed captures,
using 10,418,802 native/model steps. Direct landing is verified on 21/24 fresh
inputs. Four arrivals fulfill their instruction and then have native Summary
target success; fifteen other native successes follow failed instructions.
No tested arrival recovers a direct miss. One fulfilled arrival, case 302,
uses 749.226692 kg against 1,034.574484 kg for the first-feasible direct rule,
a saving of 285.347792 kg. This exception is retained rather than tuning the
fixed rule using fresh outcomes.

Three explicitly post-hoc proof probes cover that fresh fuel exception, an
expired arrival that still lands, and a fulfilled arrival that cannot complete
landing. All match their original Summary execution and pass complete native
proof. They use 8,466,693 additional steps and are not new population samples.
Together with the pilot and main phases, there are 92 full captures.

### Command allocation is a confirmed controller gap

`arrival-actuation` executes one unclipped vertical waypoint command through the
actual native plant and compares an algebraic inverse as a diagnostic. The
shared allocator requests a force fraction of 0.574977864 and emits that same
raw engine setting. Native minimum-throttle mapping produces force fraction
0.681233398. The inverse diagnostic uses setting 0.433303818 and produces the
requested force fraction. Source inspection and the native pair agree.

This confirms a force/engine-setting mismatch for intermediate commands with
nonzero minimum thrust. It does not measure its share of the full-flight
failures. The old waypoint commands and the angle limit are preserved in this
pass; Poly Landing already uses its paired native inverse. A safe correction
must also define behavior below minimum thrust and test low vertical authority,
rotation and saturation together. Simply clamping a scalar inverse would not
establish those contracts.

### Timing selection confounds the apparent fuel opportunity

`arrival-timing-grid` forecasts every admissible direct grid choice on fresh
case 302. It predicts a safe target contact using 521.264273 kg after 4 s of
coast and a 12 s burn, cheaper than the fulfilled-arrival flight. This costs
102,654 native forecast steps. It is forecast evidence, not a sealed Episode
landing claim. The first-feasible direct rule is not a sufficient baseline for
attributing fuel savings to trajectory shaping.

The 0.72 rad tilt limit saturates frequently on shallow inputs, but vertical
misses occur without tilt saturation. Removing the limit cannot be the only
answer. Other limits remain explicit: fixed vehicle/fuel, upright entry,
position/velocity goals on one displacement line, no arrival-attitude contract,
and a hard 30 s operation deadline that does not constrain the reused kernel's
internal horizon search. No claim of arbitrary state control follows.

## Review, validation and delivery

The read-only audit verifies all 73 allocated input hashes, 387 runtime source
bindings across the three measured phases, every saved capture/seal/decision/
proof binding, regenerated goal payloads, complete schedule accounting and zero
optimizer steps. It executes no native transition. Its conservative campaign
total is 72,772,718 steps, including the first fault reservation and both
invocations of the four-step command diagnostic.

Final API review also closes admission after native end while retaining the
arrival's reached/missed outcome. This lifecycle guard is outside the frozen
epoch-B campaign. A current-source eight-flight preservation panel matches
every pilot compact result and command/state fingerprint exactly, adding
1,601,059 measured steps. Total measured campaign, diagnostic and preservation
work is 74,373,777 steps; software/unit tests are accounted separately.

The [full HTML report](http://starship:8000/airborne-arrival/loop-20261023T222023Z/report-delivery/report.html)
uses the established common tree and rich flight templates. Each tree occurrence
is one selected captured execution; the comparison table groups Summary choices
by physical input. This uses the maintained single-execution mode so sparse
capture rosters do not acquire misleading approach slots. Common browser code
is unchanged. Core, paired reflections, fresh inputs, pilot and post-hoc probes
remain distinct. Desktop and 390 px browser checks browse all 92 occurrences,
exercise the seven-choice table and rich plots, and finish without page errors.
The report source and failed presentation-driver attempts are retained.

The rich payload exceeds the shared publisher's 100 MiB bundle bound. Its
canonical full report stays on the established port-8000 service; a small
immutable publication index can link it without changing either server's
lifecycle or reducing the rich report. No old report or selector is refreshed.

## Recommended next pass

This recommendation is not an allocation to start more work:

1. Design and validate correct force-to-engine-setting allocation, explicitly
   handling the minimum-thrust gap while preserving legacy behavior by default.
2. Requalify arrival tracking and bounded feasibility/time feedback on these
   unchanged inputs; distinguish genuinely infeasible requests from tracking
   defects and finite-search misses.
3. Establish a stronger deterministic direct timing baseline with explicit
   fuel/contact-margin tradeoffs, then remeasure intermediate-goal headroom.
4. Release a small RL goal-selection experiment only after those gates show
   both reliable local execution and meaningful choices.

No RL training, push, default promotion, tilt relaxation or historical outcome
rewrite occurs in this goal.
