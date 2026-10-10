# Automatic coast-to-terminal alternative: bounded pass

[Documentation home](README.md) · [Capability witness](ballistic_terminal_coast_results.md)

Status: **closed, all 22 planned records verified**. The
[results](ballistic_coast_terminal_results.md) demonstrate automatic 084 landing
with H1 only and no lost selected landings. This is explicit opt-in evidence,
not permission to run a wider campaign, promote defaults or publish.

## Mechanism

In the experimental ballistic loop, only after an actual waypoint H, when a
new destination ideal arc is blocked and the held command is engine-off:

1. Resolve the first blocking crest using the existing ridge classifier.
2. Query the actual engine-off plant to crest x plus half the conservative body
   diameter, on paired command ticks. Check full-body reserve/contact at every
   physics tick; allow at most 960 ticks / 8 s and the original deadline.
3. At that queried state, require descending, above-target, forward motion,
   the existing 45-degree arrival safeguard and dynamics-only terminal readiness.
   Query two seconds / 240 ticks of the ordinary standalone terminal controller's
   actual feedback commands, including slew and the existing held-command guard.
4. If admitted, keep the destination goal and execute the checked coast. This
   is not a waypoint, new goal revision or H. At the actual clearance crossing,
   recheck terminal readiness/preview and start ordinary terminal control.
5. If admission or revalidation fails, resume the original aiming/waypoint loop
   at the unchanged actual state. Do not re-admit a cancelled proposal at that
   same clock. Existing landing command/contact guards remain active afterward.

The bounded terminal preview replaces the initial-acceleration extrapolation
as this new branch's terrain admission evidence. It does not disable configured
terrain handling in the controller or certify the entire landing suffix.
The first accepted command and complete actual flight remain separately proven.
No saved handoff clocks, case IDs, restored states, new apex constraints, terrain
threshold tuning or terminal-controller parameter changes.

`coast-terminal` inherits combined V9 acquisition/fallback behavior until this
branch is selected. Its new terminal entry uses standalone defaults, with no
duration fallback/countdown/waypoint retention, matching the successful witness.
Old modes and the default `ridge` candidate are unchanged. Policy 3 remains the
sole maintained planner. No behavior change to ordinary controller entrypoints.

## Frozen comparisons and allowance

Two source-identical packs, **11 native records each, 22 total**:

- Same-source `landing-countdown` controls: random 715, 084, 142, 268, 349, 006;
  direct flat/uphill/downhill controls; external repeats of 715 and 084.
- New `coast-terminal` candidate: exactly the same inventory and repeats.

Inputs and complete previous feedback come from
`capture-landing-countdown-preservation-20261009-v1`. Keep six selected random
worlds separate from the three direct controls and two repeats; no new pass-rate
estimate or untouched-sample claim. Every native attempt also repeats its
decisions and replays its actual commands from original source.

Required checks:

- All old-mode controls match complete saved feedback exactly.
- 084 lands with one actual H, no W2, automatic state/terrain-derived entry and
  the full physical/mission/integrity/replay/decision tuple.
- Existing successful 268/349/006 comparisons stay successful; direct controls
  keep exact ordinary flight. Any loss is reported, not repaired by tuning.
- 715/142 may remain honest finite stops. No stopped state is a crash or landing.
- Full external repeats, source/binary/input seals, protected old evidence/site
  seals and common rich reports verify. Coast-selection/entry/rejection records
  bind to actual source states and do not manufacture an H.

## Validation and stop rules

Before measured records: source/boundary/query unit tests, the maintained
development gate and final isolated native/renderer build. Pin source and report
script. Afterward: saved-only collector checks, complete paired outcome review,
report structure/link checks and explicit retained 44-case normal-policy parity.
The normal gate does not require saved local evidence.

Use the existing common batch/detail templates. Extend decision navigation and
the cycle explanation with coast/entry evidence; do not replace rich views or
rewrite accepted navigation/selectors. No server restart or publication.

Finish this inventory and stop. No outcome-driven thresholds, extra native
attempts, retries, full 16-world pack, 1k rerun or default adoption. Implementation
and evidence defects may be fixed before measurement; a late integrity defect
requires an honest retained record and a new explicitly scoped recheck, not a
silent rerun. Commit/push are not authorized by this pass.
