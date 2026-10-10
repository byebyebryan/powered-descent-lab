# Ridge-aware ballistic waypoint pass

[Documentation home](README.md) · [Previous clearance panel](ballistic_waypoint_clearance_results.md)

Status: complete. The [results](ballistic_ridge_waypoint_results.md) solve 807
but lose three selected landings; the fixed allowance is closed, not permission
to rerun or tune this experiment. The capture retains the original preflight
version of this plan separately.

## Scope and rule, pinned before flights

The user-authorized goal is a simple opt-in placement experiment, not another
1k sweep or accepted-planner replacement. V4 raises rejected nearby waypoint
proposals but does not identify the blocking terrain feature. In 807 its selected
x positions are about 42, 110 and 145 m; the main crest is at 180 m.

V5 will replace the first-conflict-plus-diameter placement with a forward crest
scan. Starting at the first conflict's body-expanded forward region, retain the
highest terrain point until terrain falls by one conservative vehicle diameter.
Smaller dips do not terminate the feature. Equal-height crest points extend the
plateau to its last point. Place the goal one diameter beyond that crest: the
existing half-diameter H window plus half-diameter body extent stay beyond it.
The height uses the existing prefix maximum, body allowance, 5 m reserve and
two-second gravity allowance; V4's bounded full-body height repair still applies.

The scan is clipped before the destination, not a search for the highest point
in the entire world. If no crest resolves before that boundary, or its goal
would pass the destination, retain the old local-climb placement and label it
as a stage, not a cleared ridge. A large separating drop ends the scan; another
ridge can require another waypoint. Narrow/deep valleys remain subject to the
unchanged incoming-arc and continuation checks, not a new valley solver.

Terrain segmentation is only a proposal heuristic. All incoming curved-arc body
checks, actual-state powered correction, short-command recovery and actual H
continuation admission remain in force. It is not a line-of-sight test, a landing
suffix, a controller change or proof that every resolved crest is flyable.
The terrain-blind destination constructor remains unchanged.

## Gates and fixed flight allowance

1. Pure geometry tests: shallow dip merges; substantial valley separates;
   plateau picks its far crest edge; monotonic uphill labels a local stage;
   invalid terrain fails; selector never mutates the plant. Include a synthetic
   807-shaped segment and test ordinary arc admission separately.
2. Build once and run original-source 807 and 928 (two development flights).
   Inspect crest/goal records and actual motion. Do not tune thresholds from the
   observed outcomes or weaken a guard to obtain a landing.
3. With that same source, run the prior 16-world mechanism panel, three direct
   controls and two repeats: 21 native records, 19 primary missions. Compare
   against V4, retaining all gains and losses. The full allowance is 23 native
   invocations; no retries, prefix restart or snapshot injection. Routine
   collection/rendering errors may be repaired using retained records without
   rerunning flights or changing their source attribution.
4. Verify input/source/binary receipts, complete native command replay and
   deterministic decision reproduction. Preserve common rich detail/batch
   views and add crest-selection explanations. Run the maintained workspace
   gate, terrain-study tests and optional retained 44-case default parity.
5. Review and reconcile results. A useful local mechanism is not acceptance or
   generalization; a future 1k rerun requires a separate decision. No commits,
   pushes, accepted-site publication, navigation rewrites or server operations.

The 16 original worlds are 807, 928, 484, 034, 055, 715, 000, 001, 006, 084,
114, 139, 142, 268, 308 and 349. Direct controls are `v2_clear_845`,
`fresh_clear_uphill_805`, and `fresh_clear_downhill_805`; external repeats are
807 and 928. The denominator is selected development evidence, not an unseen
sample. Controls must retain complete ordinary flight parity; corrected worlds
may change flights and must report any lost landing explicitly.
