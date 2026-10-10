# Ballistic correction ownership and waypoint energy

[Documentation home](README.md) · [V14 evidence](ballistic_mechanics_results.md) · [Full exit diagnostic](ballistic_exit_diagnostic_results.md)

Status: this design is superseded as the immediate next pass by the completed
[native finite-correction investigation and implementation](ballistic_finite_correction_results.md).
Its optional-query/retention distinction is retained. Section B's waypoint-room
ranking is **deferred**, not implemented or validated: lower horizontal speed
alone can worsen vertical energy and required effort, and the original screen
is not destination/terrain/landing proof. Review the new terminal-entry losses
before reopening that ranking experiment. The historical reasoning below is
preserved rather than relabeled as a successful flight result.

Design only. This pass does not implement another flight mode, change a physical
guard, or require a complete landing suffix from a waypoint. The game-oriented
goal remains: establish a reasonable ballistic continuation, clear one blocking
feature when needed, then replan from actual flight state.

## What is known, and what is not

The early-piecewise adapter bypasses its 24-tick planning cadence whenever a
correction is present. Across the 45 primary traces, 1,879 of 2,002 successive
blocked early queries are only two ticks apart. Waypoint coast acceptance can
discard a queued correction; a failed optional replacement immediately stops
the flight. These are execution/ownership problems, not proof that piecewise
planning is the wrong model. Repeated constructions sometimes retain the same
absolute turn endpoint as attitude progresses; do not claim every query delays
the burn or that all engine-off turn commands are erroneous.

There is also a distinct objective problem. In 044, the destination fit requests
about 69 m/s horizontal motion, but the replacement waypoint fit remains near
80 m/s. Reaching the clearance point cheaply is not the same as improving a
late, high-speed state. In the V13 1k, 124 missing-aim stops occur exactly at H;
their median remaining distance/speed is 80.77 m / 72.88 m/s, versus
561.58 m / 47.63 m/s for the 339 successful last-H states. These associations
do not establish that speed alone caused every failure.

A read-only reconstruction of the existing 32-profile waypoint constructor at
the first blocked early-query states finds lower-speed candidates without a
larger search. Its least-effort selections reproduce the recorded duration,
turn/burn ticks and thrust vector (vector error below 1e-9 m/s²).

| Subject | Existing fits | Chosen arrival vx | Chosen room | First nonnegative-room vx | Added-thrust effort, chosen → alternative |
| --- | ---: | ---: | ---: | ---: | ---: |
| 044 | 32 | 80.28 m/s | -210.44 m | 36.03 m/s | 0.38 → 71.34 m/s |
| 050 | 32 | 78.73 m/s | -196.70 m | 36.95 m/s | 0.70 → 62.64 m/s |
| 062 | 32 | 79.17 m/s | -206.20 m | 34.35 m/s | 2.17 → 68.93 m/s |
| 081 | 31 | 57.90 m/s | -84.25 m | 36.40 m/s | 20.18 → 55.73 m/s |
| 715 | 32 | 72.65 m/s | -201.60 m | 20.98 m/s | 0.54 → 95.59 m/s |

Room uses the existing gravity-supporting braking heuristic, current-origin
mass and upright entry attitude. It ignores vertical energy, terrain, fuel
consumption and realized attitude. Effort is thrust-acceleration magnitude ×
burn time, not fuel or a complete energy measure. The alternatives require
longer flight and much more thrust; none is a terrain-audited proposal, realized
handoff or landing. This supports a bounded selection experiment, not adoption.

Reproduce this read-only screen with
`rtk proxy python3 -B studies/terrain_profiles/ballistic_waypoint_energy_screen.py`.
It reads the sealed early-piecewise capture; it does not write outputs or launch
native commands. Its three pure tests need no retained evidence.

## Recommended small implementation experiment

Start from the source-frozen exit-check reference, now 706/1000 with its eleven
known V13-success losses retained. The broad gain is worthwhile; it does not
establish acceptance or erase those losses.
Keep terminal logic, profile count, waypoint placement, reserves and deadlines
fixed. Separate execution ownership from the optional energy preference.

### A. Optional replanning is a transaction

1. Keep the active goal and checked correction while previewing a replacement.
2. Commit only an admitted replacement; otherwise record the rejection and
   keep the active leg. The mandatory short-command guard still decides whether
   the actual next command may execute. Optional failure is not command safety.
3. Respect the existing 24-tick cadence for optional destination attempts.
   Safety-triggered replanning and genuine H transitions may run immediately.
4. Refresh valid pending corrections with existing absolute arrival/turn/cutoff
   clocks. Do not reconstruct simply because the current coast passes a
   waypoint's positional window. Release on an achieved objective, infeasibility,
   real obstruction or expiry—not an unconditional promise to finish a bad burn.

The important design distinction is correction objective. A terrain-clear,
target-safe actual ballistic continuation can already satisfy destination
correction, even if higher/steeper than the nominal. A waypoint's position-only
coast does not establish that an intended speed reduction is complete. Any
new energy-directed correction therefore needs a simple, explicit completion
reason; avoid a universal rule that forces all burns to finish.

### B. Prefer room among existing waypoint fits

As a separate opt-in, when the original least-effort fit has negative estimated
room, prefer the least-effort existing fit with nonnegative room. Use the
existing helper and zero crossing, not a new margin, hard landing requirement
or global speed cap. If no estimate/alternative exists, retain the original
selection. Destination arcs remain terrain-blind and unchanged.

Use the predicted cutoff followed by its ballistic continuation to estimate
waypoint entry—not the ideal instantaneous departure arc. The selected fit
still needs the existing incoming/continuation terrain audits and actual
command checks. If it fails, record that explicitly; do not treat the cheap
heuristic as a clearance or recoverability certificate.

A positional coast must not immediately cancel a correction specifically
selected to reduce forward energy. Recheck the same cheap objective on the
actual continuation while retaining valid absolute clocks. If implementing
this requires several new mode/phase exceptions, stop and review the ownership
design instead of adding them piecemeal.

### Recovery remains a separate option

The first recovery choice is greedy: accept its own turn-plus-0.2-second check,
then stop searching. All seven changed warning-mode subjects initially select
the requested direction over 24 ticks despite conflicts 46–102 ticks ahead.
First compare the same four choices over a common existing response horizon at
those recorded origins. Do not add commands or assert a later landing from that
clone check. A longer constant-command warning can also misrepresent a queued
turn-to-burn transition; include that boundary in the diagnostic review.

## Validation that permits learning

Unit tests first: failed optional preview leaves goal/clock/state unchanged;
same-goal preview does not reset a retained turn/burn; optional queries respect
cadence; expired/unsafe corrections still release; safe higher/steeper destination
coasts need no unnecessary correction; and no fake H or weakened command guard.
Ranking tests retain original order when room is unsupported/nonnegative or no
nonnegative alternative exists, and use realized-cutoff estimates.

Use 044/050/062/081/715 plus 084/142/349/974 and the four zero-H controls to explain
what changes. Measure query cadence, completed corrections, actual H speed/room,
fuel and flight duration as well as landing. Do not demand that a local panel
has zero landing regressions before permitting diagnostic breadth.

Then freeze candidate A (ownership) and candidate B (A plus room preference and
objective-aware completion), and run the original 1k for each worthy candidate.
B depends on correction retention; a pure ranking change that is immediately
cancelled by positional coast acceptance is not an informative isolated test.
Keep all losses and unknown outcomes in paired accounting. Full replay,
source/input integrity, finite execution and authentic reports are prerequisites;
zero outcome regressions are a promotion consideration, not the investigation
gate. Reusing this 1k measures development trade-offs, not held-out performance.
An untouched population and maintained core/controller checks follow only after
a promising candidate is frozen for replacement acceptance.

Do not bundle recovery changes, add a general optimizer, enlarge search, raise
more waypoints blindly or extend mission budgets. This document is a proposed
next pass, not authority for those implementations or additional flights now.
