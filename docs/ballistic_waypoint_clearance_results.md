# Ballistic waypoint-clearance repair results

Date: 2026-10-08. Status: bounded opt-in implementation and focused validation
complete; partial improvement, not a complete fix for 807/928 or planner acceptance.
This follows the [401/1000 terrain-correction checkpoint](ballistic_terrain_correction_results.md).

## Verdict

The original `local_replan_proposal_budget` stops in 807 and 928 are removed.
The waypoint repair uses the same full-body terrain clearance query as the
checker, including the incoming arc and a short uphill continuation. Both
subjects execute additional useful flight, but **neither lands**:

- **807:** previously stopped at 6.183 s near x = 42 m after H1. It now executes
  H2 near x = 110 m, then stops at 18.717 s near (145.29, 396.45) m because the
  next actual-state continuation is unsafe. H3 is not admitted. The major crest
  near x = 180 m has not yet been cleared.
- **928:** previously stopped at 7.717 s near x = 44 m with no H. It now executes
  H1 at 15.600 s, clears the early crest and reaches x = 1041 m at 28.917 s.
  It stops on `terrain_recovery_no_safe_command` while acquiring a later waypoint;
  H2 is not admitted. This is not a landing or a physical crash.
- **484:** the same general height repair changes a proposal-budget stop into a
  verified landing at 41.883 s after two actual handoffs.

The focused panel contains 16 selected random worlds, three direct controls and
two external repeats. It lands **10/16 selected random worlds versus 9/16** in
the previous candidate, plus all three controls: **13/19 primary missions**.
All nine previous random successes preserve their complete ordinary flights,
commands/updates and actual handoff/revision bindings exactly. Flat, uphill and
downhill controls still land directly without cutaways. All 21 records verify;
the six primary non-landings remain finite flying stops, not crashes.

This is selected development evidence, not a population estimate. **The full 1k
was not rerun.** The latest measured full-sweep rate remains 401/1000 for the
previous candidate, not for this revision. The ordinary planner remains policy 3.

## Small implementation contract

Candidate: `ballistic_feedback_v4_waypoint_clearance`, explicit opt-in only.

The terrain-blind constructor, source support, landing controller, command
recovery, physical clearance reserve and actual H guard are unchanged. The
waypoint generator still supplies the initial position. An already clear
proposal follows the original admission path without new repair work.

When a waypoint proposal's ideal arc is blocked, keep its x coordinate and scan
the complete geometric arc plus the existing 240-tick unpowered continuation.
Every sample uses the upright full-body terrain query, not just the waypoint
centre or the first warning. At fixed duration N, raising the waypoint by dy
raises the arc at relative tick k by dy*k/N. The largest scaled clearance deficit
estimates a required waypoint-height increase. Include the existing handoff height
tolerance in endpoint/continuation room and a 1e-6 m upward roundoff allowance;
the physical five-metre reserve is not reduced.

Reconstruct from actual state and recheck the repaired incoming arc and ideal
continuation. The constructor can change duration, so the estimate is not accepted
without that recheck. The existing **four total proposals per replan** remains
the bound; there is no expanded apex/profile grid, new retry cap, landing-suffix
solver or seed-specific branch. Only the waypoint height changes, never the
terrain-blind destination arc, actual velocity, clocks or restored plant state.

This geometric continuation estimate does not certify the actual arrival state.
Powered drift and waypoint admission tolerance can produce a different velocity;
the unchanged actual-state two-second guard still decides whether H is allowed.
807 exposes that remaining boundary rather than silently relaxing it.

## Review the common reports

The [local common batch](../outputs/eval/planner_v2_random_terrain/capture-waypoint-clearance-panel-20261008-v1/index.html)
is reachable through the existing LAN server at
`http://192.168.1.110:8000/eval/planner_v2_random_terrain/capture-waypoint-clearance-panel-20261008-v1/`.
Open **807**, **928**, then **484**. The original rich plots/presets remain intact.
Each repaired proposal exposes its changed goal, height increase, limiting
incoming/continuation condition and handoff-height allowance. Proposed revisions
are not actual H markers. Only executed handoffs appear in the flight annotations.

This diagnostic is not published or selected on the accepted report home.
No server lifecycle operation or accepted-site refresh occurred.

## Validation and provenance

- 23 focused native tests pass, including five synthetic geometry tests for
  footprint width, incoming crest clearance, uphill continuation, existing H
  height tolerance and strict domain-error propagation. Synthetic query states
  are not an executable airborne frontdoor or physical flight evidence.
- The maintained eleven-check development gate passes, including strict
  all-target Clippy and 61 Node tests. Both common diagnostic-batch tests and
  all 135 terrain-study tests pass.
- All 44 ordinary planner cases retain exact complete non-timing numerical
  parity with the retained October 7 capture.
- All 21 panel records pass native source-command replay, internal decision
  reproduction, saved verification and the two complete external repeats.
  Static report checks cover all 1,754 embedded cycles, 16 repaired-proposal
  records, the common batch template and every local batch link. LAN response
  is HTTP 200; no browser/pixel-level visual acceptance is claimed.

Four development flights precede the panel. The initial pair exposed that an
exact-height waypoint forecast omitted the existing H height window; the second
pair includes that mechanical correction. All four are retained. There is no
panel-result-driven flight-policy tuning.

The collector completed all 21 native records before its common batch renderer
rejected safe underscores in the control IDs. Two failed renderer invocations
created no index. A display-only identifier/denominator repair, reader-test
strengthening and create-only finalizer are separately sealed in
`renderer-repair.json`; **no panel flight was rerun** and original source
snapshots/results were not rewritten. The finalizer's exact source and renderer
binary are retained alongside the captured native binary and receipt.

A subsequent `format!` to `format_args!` report-only Clippy repair changes the
final Rust/binary seals, not the flight logic or report output. Two additional
final-source development flights reproduce 807/928's **complete feedback,
planning data and report bytes exactly**. There are therefore **27 native
invocations**: six development flights plus 19 panel primaries and two repeats.
The panel remains attributed to its frozen source, not relabelled as a different
final-source capture.

```text
panel Rust tree/manifests: 2a0ffdc49537f89e57d5b3b84685af3ae8e3c3a8976e2ca5863c65c90e2bc04b
panel native binary: b18fc64c017271797598d628fdd2d12c84e0da18e29017b61f6ae4fc773e6e32
panel receipt: b1fce26d22ad3027d46a98f3c0faecd972c320106b9a0003326eac36cc12e0be
panel results: 22fbe216da6b525ec0089cba0400d246bfd3b5d7b35ad1f871e6805c5a9d8e24
panel plan: 299cf63a9a55facb57eec89039323f6f2e101b0178a730b3ac2c9013f5d63441
final Rust tree/manifests: a127c05a62c378b1ebcdf6913baf1f0c181b59a1c4ea852e4e3a39294adb2499
final native binary: bbe865e6b05449a26252b424d22a9a036f79e65a3782251ac1e7f9086dd484cb
```

No frozen inputs, old outcomes, accepted selectors, ordinary release executable
or retained report pages were changed. No commits, pushes, delegation or goal
creation occurred.

## What remains

Keep the geometry repair, but do not describe 807/928 as fully solved. Next
review 807's **actual-state continuation rejection** separately from 928's
**later acquisition/command-protection limit**. The former may motivate another
local correction before H; it must not be relabelled as a passed handoff or fixed
by disabling the continuation guard. The latter belongs to response-room and
correction control, not another waypoint-height retry.

Review those changed flight records before expanding the controller or spending
another full 1k allowance. No broader campaign or default promotion is scheduled
automatically by this completed bounded pass.
