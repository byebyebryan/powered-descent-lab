# Bounded handoff braking-room preference

Date: 2026-10-07. Approved follow-up to the
[failure-only timing fallback](intervention_fallback_results.md). Current baseline:
65/100 challenge landings, all 57 earlier successful flights preserved.
The [completed results](handoff_room_results.md) close this 171-evaluation
allowance. The sequence below is its executed contract, not an active task list.

## One small change

Keep the terrain-blind nominal arc, current entry stages, 42 local templates,
first eligible handoff per row, five-metre reserve, two-second continuation,
six-correction cap and actual-handoff replanning. Complete each existing stage.
Only if its original rank winner has negative estimated horizontal braking room,
prefer the original-rank-best already accepted row with nonnegative room.
Otherwise retain the original winner. Room cannot reject a candidate or open
timing fallback when a primary candidate was accepted.

Estimate gravity-supporting lateral braking acceleration from the existing
nominal 0.925 thrust fraction at actual handoff mass. Include forward travel
during the shortest attitude turn, rounded up to the existing held-command pair,
and constant-acceleration stopping distance. Compare with distance to target
centre. No tuned comfort margin, weights or additional safety multiplier.
Vertical energy, terrain and braking fuel burn are omitted deliberately: this
is a ranking heuristic, not a feasibility, landing or suffix certificate.
Unsupported/nonfinite estimates leave the original winner unchanged.

## Frozen sequence and closure

The [contract](../studies/terrain_profiles/handoff_room_plan.json) binds original
scenario bytes separately from the current fallback motion baseline. At most
171 campaign evaluations, four workers, 300 seconds per survey case and three
hours per survey capture. Use isolated create-only source-frozen captures and
retain all attempts; no restarted roots or terrain regeneration.

1. Add query-only actual accepted-handoff state and room metadata. Run the eight
   fixed shadow cases without changing selection. Require exact complete-flight
   preservation except three wall timings and additive query diagnostics. Confirm
   actual states support the predicted six switches, with `090` unchanged and
   successful near-zero-room `064` unchanged.
2. Enable the one-sided preference and run the fixed 17-case focus. All six
   successful controls must remain exact verified landings; require at least one
   new verified landing before expansion. NoNominal-to-NoClearing is not a gain.
3. Run the maintained development gate and a fresh 44-case benchmark with
   `--no-publish`: existing acceptance, 36 mandatory core landings, exact
   preservation of all 38 current benchmark landings, 44 integrity and 42 replay
   passes. No historical numerical comparison exceptions are used.
4. Only then run the unchanged 100 challenge cases and fixed repeats `028`,
   `050`. Preserve all 65 previous successful complete flights; no crashes, fuel
   exhaustion, timeouts or integrity/replay failures. Repeats are exact except
   wall timings, including the new diagnostics.

If focus preservation or recovery fails, close the experiment without tuning;
restore only selection, retain truthful diagnostics and evidence. Unit tests
and reversible static checks are separate from this campaign allowance.

Retain common rich native reports and expose the estimate in their collapsed
query diagnostics. No commit, push, report-site publication, accepted-capture
promotion, navigation refresh or server operation is part of this pass.
