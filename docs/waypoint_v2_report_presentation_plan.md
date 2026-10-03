# Waypoint V2 report presentation plan

## Purpose and decision

This design is superseded by the
[report navigation and waypoint annotations plan](report_navigation_and_waypoint_annotations_plan.md).
The user rejected replacing the detailed report: clutter referred primarily to
navigation, not to its rich plots and inspection tools. Keep the design below
as the historical contract of the implemented prototype, not the next execution
plan.

This 2026-10-02 design proposes a small planner-focused presentation for the
accepted opt-in V2 captures. A reader should understand where the rocket flew,
where corrections ended, why planning restarted and whether the mission landed
without opening JSON or needing a narrated walkthrough.

Start with the late-ridge mission, then check direct controls, repeated
corrections and honest partial outcomes. Reuse retained evidence; do not change
the planner, run another flight matrix or redesign every historical report.
Implementation and browser checks are now recorded in the
[presentation results](waypoint_v2_report_presentation_results.md). Automated
checks passed, but the user did not accept this standalone presentation.
The design below records the earlier approved scope and gates.

## Preimplementation findings

Use the accepted source capture at
`outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a`.
It contains all 32 case entries, with 30 actual complete/partial flights and two
unsupported preflight results. Keep this source pinned rather than choosing a
different repeat or following a mutable latest link.

The current V2 writer in `pd-eval/src/waypoint_v2_output.rs` sends samples and
events to the generic run report with no controller update records. It does not
send the V2 cycles, correction programs or segment boundaries. The generic
renderer obtains waypoint positions from an authored transfer route or
controller markers, neither of which this route-free mode supplies.

The missing information already exists in `flight.json`: actual segment entry
and end states, local correction selections, handoff steps, fixed nominal audit
results, stop reasons and the complete executed source evidence. In the retained
late-ridge capture, one correction runs from 12.6 to 20.216667 s and ends near
(-288.849785, 318.285445) m; a new direct flight then lands at 34 s.

V2 waypoints are dynamically selected handoff states, not authored route points.
The position marks where planning restarts, but velocity, attitude, fuel and time
also matter. A handoff need not lie beyond the obstacle's far edge. Do not draw
it as a fixed point that the vehicle stops at, or claim that reaching it alone
guarantees landing.

The generic refresh command handles standard batch bundles. V2 captures use
`scenario.json` and `flight.json`, rather than those bundles' separate controller,
manifest and telemetry files. They need a narrow retained-capture rendering path,
not an invocation of generic refresh or a new mission execution.

## Default mission view

Use a small V2-specific page with this reading order:

1. A friendly mission title and one sentence stating the result, such as
   "Landed after one correction" or "Stopped before departure: no clearing
   maneuver found." Show correction count and actual elapsed flight time.
2. One large trajectory diagram with terrain, source, target, the actual flown
   path and numbered handoffs. Keep physical x/y units and consistent scaling.
3. A short chronological list of flown pieces and a selectable explanation for
   each correction. Selecting an item highlights its segment and handoff.
4. Collapsed diagnostics and links to the original capture. Detailed velocity,
   thrust, clearance, fuel, commands and identities are supporting information,
   not the opening screen.

The first version should use a self-contained HTML/SVG diagram and small plain
JavaScript interactions. Reuse existing report helpers where useful, but avoid
copying the entire generic dashboard or adding a frontend framework. Core
mission understanding must not depend on a plotting CDN, hover or animation.
Pan/zoom, playback and synchronized telemetry cursors are deferred.

## Trajectory and handoff semantics

Use three stable visual categories, with a text legend as well as color:

- Blue: the initial nominal flight actually executed before any correction.
- Orange: local correction programs actually executed to a handoff.
- Teal: nominal flight actually executed after replanning, including any powered
  acquisition, coast and terminal braking. Do not label the whole piece coast.

Mark the end of each executed local correction as H1, H2 and so on, with the
caption "Waypoint handoff: replan from actual state." Selection shows its exact
time, position and velocity. Mark start and actual finish separately. Use a
landing symbol only when the recorded physical and mission outcomes establish
success. Otherwise label the actual stop and its planning reason.

Construct the path from actual samples plus exact stored segment endpoints.
Some boundaries fall between the 0.1 s sample times; inserting their recorded
positions into the display is allowed, moving or interpolating the handoff is
not. Adjacent colored segments share the same exact boundary point. Preserve
odd final contact steps and use stored times or the scenario rate, not a rounded
clock or a hardcoded display cadence. These display points do not modify the
source samples, actions or events.

Keep only short handoff labels on the default chart. Show correction entry E1
and detailed state values when that correction is selected. If labels overlap,
use the adjacent chronological list and one expanded annotation; never move
physical marker positions or hide a correction from the count. The list must
also work with keyboard input and at narrow screen widths.

## Explain why a correction happened

For each recorded local-clearing cycle, provide one short explanation:

- The selected direct proposal was blocked by terrain contact or insufficient
  body clearance, using the actual recorded audit classification.
- A local correction began at its recorded entry and ended at the selected
  handoff, where the next planning cycle started.
- The next cycle landed directly, needed another correction or stopped with
  its recorded finite reason. Do not claim an eventual landing at every handoff.

For late ridge, the original proposal's body clearance first falls to about
4.612 m against the 5 m reserve. That is a rejected hypothetical continuation,
not an impact experienced by the active rocket. Make that distinction explicit
in the correction explanation. Show any diagnostic conflict marker only in the
selected correction detail, with a distinct shape and "not flown" label.

Do not draw a dashed rejected arc in the first implementation. The current
capture has its commands and audit endpoint, but not a complete spatial trace.
A future overlay would require checked offline command propagation and matching
the stored audit from the original live origin. That is separate work; never
substitute a fitted ballistic parabola or a straight line between endpoints.
Likewise, do not draw the unexecuted continuation certificate as actual flight.

## Browse the suite without a directory listing

Add one curated V2 presentation index for the pinned final policy 3 capture.
Lead with five examples: flat direct, late ridge, successive rising, reference
plateau and a near-target limitation. Group uphill/downhill direct controls
with the flat example. Each card says what to inspect and the recorded result.

Keep all 32 cases available in a compact grouped list: eight clear controls,
sixteen ordinary terrain missions and eight diagnostics. Show the exact policy
and source capture, without treating diagnostics as ordinary coverage or mixing
preliminary and final repeats. Use friendly names; archive IDs and hashes belong
in capture details. Breadcrumbs return to this index.

Unsupported cases stay visible with "Not simulated" and their recorded reason.
Do not fabricate a trajectory or change their existing lack of a run report.
NoClearing at H0 shows a start point and "Stopped before departure," not a crash
or a flown rejected arc. A partial airborne stop ends at its actual retained
state. Add a normal site navigation link without promoting this opt-in mode into
the legacy controller scorecards or changing their success denominators.

## Implementation boundaries

Keep ownership and dependency direction simple:

- `pd-eval` translates and checks the saved V2 result into a small typed report
  payload: outcomes, actual segments and endpoints, corrections, handoffs and
  recorded reasons. A focused `waypoint_v2_report` module can own this adapter,
  retained-capture reader and presentation index orchestration.
- `pd-report` owns a small `waypoint_v2` presentation module consuming that
  payload. It must not depend on `pd-eval`, invoke a planner or create simulation
  state. Share existing escaping and display helpers without a broad refactor.
- `pd-eval/src/waypoint_v2_output.rs` uses the same renderer for future captures;
  existing report APIs and non-V2 pages remain unchanged.
- Expose one explicit report-only CLI taking a retained suite root and a fresh
  output directory. Validate case paths remain inside that root and match its
  declared cases. Do not make the generic refresh scan arbitrary research roots.

The reader checks summary/result identity and policy agreement, correction
counts, segment continuity and handoff ownership. Missing or inconsistent
evidence produces an explicit rendering error, not guessed annotations or an
apparently accepted flight. It does not attempt to re-prove flight integrity;
recorded simulation evidence and presentation validation remain different gates.

Write refreshed views and their input/render provenance to a new root beneath
`outputs/reports/waypoint-v2/`. Preserve every retained research JSON and HTML
file byte for byte. Record input hashes and renderer version separately; the new
renderer does not become the build that originally flew the mission. The running
report server can serve the new directory without another server or restart.

## Execution sequence after approval

1. Freeze the retained source root and hashes. Define the display payload and
   prove handoff/segment extraction on late ridge. Review its labels and a single
   readable diagram before expanding the layout.
2. Implement the lean renderer, correction explanation and collapsed details.
   Render late ridge from saved data into a new root. Review it in a browser
   before processing the rest of the suite.
3. Render clear flat/uphill/downhill, successive rising, reference plateau and
   the near-target partial outcome. Correct presentation issues only; do not
   tune planner behavior to improve the screenshots.
4. Generate the curated index and all 32 recorded case entries. Wire the same
   adapter into future V2 capture output and add the report-only CLI.
5. Complete data, legacy-report, browser and source-preservation checks; then
   hand over one LAN entrypoint for user review. Commit only when requested.

If useful during a separately approved worker loop, delegate the bounded
renderer/tests while keeping evidence mapping and final acceptance with the
primary. This design pass does not start agents or a goal.

## Validation and acceptance

Use these retained physical captures as presentation acceptance fixtures:

- Clear flat/uphill/downhill: no handoff markers, zero corrections and direct
  landing; uncut terrain remains visible.
- Late ridge: one H marker at H2426 and 20.216667 s, the three actual segment
  ranges 0–12.6, 12.6–20.216667 and 20.216667–34 s, and one recorded obstruction.
- Successive rising: H1 at H2006 and H2 at H2774, with both corrections visible
  and the intervening replanned segment retained.
- Reference plateau: H1/H2/H3 at H2820/H3136/H3226, followed by actual landing
  at H4886. Selecting a correction must not display all hypothetical arcs.
- Near-target diagnostic: NoClearing, zero executed segments and elapsed time
  zero, with no crash symbol or imaginary journey.
- Unsupported gravity/vehicle: visible index entries, no simulated path.

Add focused extraction and HTML tests for exact off-sample boundaries, a
synthetic odd contact endpoint, malformed/missing segment evidence, escaping
and empty/partial states. Test the future writer and retained reader share the
same display data. Check all index links and source digests before and after
rendering; no full flight matrix is needed for this presentation-only work.
Run workspace, format and Clippy checks proportionate to the integrated source,
disclosing the existing baseline lint rather than silently changing it.

Browser validation must confirm the diagram actually renders, the initial view
is uncluttered, selection and disclosure work, and labels remain usable at
approximately 1280 px and 390 px widths. Current inspection found no
Chromium/Firefox executable on PATH or repo-resolvable Playwright/Puppeteer. Do not claim
screenshots or browser acceptance from HTML tests; use a subsequently available
browser or the user's LAN review, and report that gate pending if unavailable.

The decisive human check is whether the late-ridge page answers, without a
narrator: Did it land? Why was a correction needed? Where did that correction
hand off? What happened after replanning? Native tests alone cannot accept the
presentation. Planner coverage, margins, commands, defaults and acceptance
criteria remain unchanged throughout.
