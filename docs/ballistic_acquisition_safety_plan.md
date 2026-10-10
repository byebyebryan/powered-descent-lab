# Independent acquisition-gate and terminal-safety experiments

[Documentation home](README.md) · [829/1000 reference](ballistic_recovery_consistency_results.md)

This authorized goal tests two simple mechanisms independently. Neither changes
the maintained planner default or claims that a passing query guarantees landing.
The preceding read-only diagnostic found earlier native destination acquisitions
in 19 of 23 room-gated missing-aim cases (five coast states each), but none in
twelve room-gated waypoint-construction cases. At all 23 terminal-command stops,
upright coast/support gives a passing unchanged short guard in fifteen cases.

## Fixed ablations

Both inherit the complete opt-in V17 recovery-consistency candidate. There is no
combined mode or outcome-driven selection in this pass.

- **Acquisition gate:** at the existing 24-tick waypoint-coast query cadence,
  try the existing destination fit regardless of estimated horizontal room.
  Keep the estimate as diagnostic evidence. Admit only after the existing native
  powered-prefix, coast-settling, realized approach and ballistic reserve checks.
  A miss or optional prediction-domain failure retains the active waypoint.
- **Terminal safety:** after the existing nominal command and pad-lift adapter
  fail the native reserve guard, try upright coast, then upright full support.
  Select the first passing command under the unchanged 24-tick terminal guard.
  Reevaluate the normal controller at the next pair. Keep the standalone
  coast-to-terminal branch untouched; record replacements in issued frames.
  No passing replacement means the existing protected stop remains.

Preserve initial construction, 32 profiles, waypoint placement/ranking, four
local proposals, actual handoff contract, recovery order/horizons/episode bounds,
contact/reserves, controller configuration, cap 24, original deadline/fuel/clocks,
proofs, ordinary policy 3 and the experimental ridge default. No extra landing
time, clearance exception, additional arc family or terrain-avoidance solver.

## Frozen evaluation

Reference: `capture-ballistic-recovery-consistency-full-20261009-v1`, receipt
`bc4f4ebffd1699a167cc3fdd58f8901b9cb42e9e3a8654b18d53200630c6728b`, results
`932693f34d1660c925674a52eb42040cde6caf1b5dc7678c8e2ae62948ae3678`.

The focused layout is the union of the prior 48 recovery/transition controls,
the 23 room-gated missing-aim subjects, twelve room-gated waypoint misses and
all 23 recorded terminal short-command stops: 101 unique primary worlds. Repeat
056, 138, 538, 715 and 349 separately. The runner freezes this exact layout
before running. Five stages total 2,328 declared attempts, including repeats.

Stages, on one source/native/renderer/reader freeze:

1. Current-source V17 control on the focused layout: complete feedback must
   reproduce the frozen reference exactly, not merely match landing outcomes.
2. Acquisition-only and terminal-only focused runs on that same layout.
3. Acquisition-only full original 1000 and the five repeats.
4. Terminal-only full original 1000 and the five repeats, independently against
   the same V17 reference, not against the acquisition candidate.

Use four workers, 60 seconds per attempt, two hours per stage, create-only
captures and no retries. Honest finite stops or focused regressions do not cancel
the full paired sweeps. Numerical, integrity or collection failures are retained
and investigated without rewriting outcomes or changing the measured candidate.
Keep direct controls, exact repeats, initial arcs, focused/full overlap, complete
physical/mission/integrity/source-replay tuples and deterministic decisions.

Protect accepted selectors/pages, retained reference inventories and all existing
navigation/server source edits. The already-authorized dynamic report library
may discover new captures naturally; its generated discovery indexes are not
flight evidence and are not frozen as manual publication artifacts. Do not
regenerate historical reports, change navigation sources or restart the server.
New captures retain the existing rich common batch/detail reports.

## Review and verdict

Before measuring, run native tests, synthetic terrain-tool tests, inspect the
diff and run the maintained developer gate. After collection, independently
authenticate receipts, repeats, overlapping records, per-recipe gains/losses,
terminal replacement counts, unchanged preterminal flights for terminal-only,
remaining stop groups and runtime. Check explicit retained 44-case numerical
parity separately; preserve the known untouched-HEAD exact-float discrepancy
rather than weakening its comparator.

Positive paired totals are development evidence on a reused population, not
held-out generalization or default acceptance. Document both experiments even
if one is negative. Combining them, changing recovery, extending deadlines,
running fresh held-out populations and promotion need a subsequent decision.
No commit, push or external publication is authorized by this goal.
