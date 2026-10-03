# Waypoint V2 report presentation results

## Result and user feedback

The 2026-10-02 presentation pass implements the
[earlier approved plan](waypoint_v2_report_presentation_plan.md). The retained policy 3
suite now has a curated index, 30 self-contained actual-flight pages and two
unsupported entries without invented flights. Planner behavior, commands,
policies, safety margins and defaults are unchanged. No new mission matrix ran.

Implementation, evidence consistency and real headless-browser checks passed,
but the user rejected the standalone presentation as too large a departure
from the detailed report and missing useful information. The user clarified
that clutter referred primarily to navigation; rich plots and inspection tools
should evolve, not be removed. The
[revised design](report_navigation_and_waypoint_annotations_plan.md) supersedes
this UI direction. Automated success is not user acceptance, and the existing
worker goal is not achieved.

The [prototype entrypoint](http://192.168.1.110:8000/reports/waypoint-v2/presentation_20261002_v1/)
remains available as historical presentation evidence. All 32 cases remain
available in its grouped lists; its useful mapping/validation work can support
additive annotations in the original full report. No prototype or capture is
deleted or relabelled as a newly accepted view.

## What changed

The V2 pages show one actual trajectory with equal metre scales, source and
target pads, blue initial nominal flight, orange local corrections and teal
replanned nominal flight. H1, H2 and H3 mark the exact ends of executed
corrections. Selecting a chronological piece highlights it and its handoff;
the other flown pieces remain visible.

The selected correction explains its recorded obstruction and what happened
after replanning. Exact states, velocities and audit details are collapsed.
Friendly times are limited to six decimals; exact retained times and marker
positions are preserved in the data and detailed state displays. Nearby label
text alternates above and below markers without moving their physical positions.

The initial late-ridge browser review exposed excessive numeric detail and an
overlapping axis label. The revised view closes exact-state tables, rounds only
display labels and removes the optional hypothetical conflict marker from the
chart. Its recorded state remains explicitly labeled **not flown** in audit
detail. No rejected arc or unexecuted continuation certificate is drawn.

Zero-step NoClearing shows the source and terrain, no path and no crash or landing
symbol. Unsupported cases say **Not simulated** and have no flight page. The
normal report home links to the completed V2 presentation without adding it to
controller scorecards or changing their denominators.

## Implementation and ownership

Luna owned the bounded renderer and its focused tests. The primary owned the
typed display contract, evaluator evidence mapping, archive reader, CLI,
future-capture wiring, navigation, browser checks and final review. This kept
flight semantics and coupled evidence decisions with the primary while giving
Luna one independently reviewable presentation slice.

The implementation is split across:

- `pd-report/src/waypoint_v2.rs` and its `data.rs`: display-only HTML/SVG and
  small selection JavaScript, with no framework or CDN.
- `pd-eval/src/waypoint_v2_report.rs`: exact endpoint projection, consistency
  guards, retained reader, create-only output and render provenance.
- `pd-eval/src/waypoint_v2_output.rs`: future V2 captures use the same adapter
  and renderer; non-V2 report APIs are unchanged.
- `pd-eval/src/main.rs`: explicit `waypoint-v2-report`, with an optional `--case`
  for staged review. It does not plan or simulate.
- `pd-report/src/site.rs`: independent opt-in presentation navigation; staged
  single-case reviews are not advertised as complete suites.
- `scripts/check-waypoint-v2-report.mjs`: native Node browser checks through a
  caller-owned, isolated Chrome session. No browser packages were installed.

The reader rejects missing evidence, bad paths, duplicate identities, policy or
summary disagreement, inconsistent counts, discontinuous boundaries and handoffs
owned by a different selected proposal. It checks presentation consistency, not
physical flight integrity anew.

Two archive-format differences needed explicit handling: the JS suite aggregator
spells integral floats as integers, and wraps unsupported run summaries in three
null fields. Comparisons admit exact numeric equality, never tolerances; the
three-null wrapper is admitted only when there is no manifest or actual flight.
Tampered endpoints and large distinct integer values remain rejected. All
declared evidence is checked before the output root is reserved.

## Evidence and preservation

The source remains pinned to
`outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a`.
The refreshed views are under
`outputs/reports/waypoint-v2/presentation_20261002_v1`.

The final receipt records all 127 input-file hashes, 31 rendered-page hashes,
renderer version, observed renderer source hashes and the actual rendering
process hash. Its historical flight provenance is separate from the renderer's
new build. Every recorded input, output and renderer hash verified after rendering.

All 127 retained JSON/HTML files match the pre-pass aggregate SHA-256:

```text
524632b1560ea647e0cfe0b869d3f18b9f099676ee12548185b54dea67b5b61d
```

The 30 existing evaluation/guidance HTML files also retain their pre-render
aggregate SHA-256:

```text
accc75cd1c79dafdcd036dd345e690fedaf7b722b85ab54ddc1dd2a1e3b36a4a
```

The normal home and outputs indexes gained navigation; the retained captures and
controller reports were not overwritten. Earlier presentation/browser attempts
remain preliminary artifacts, not alternate flight evidence. Only the completed
all-case root is advertised by normal site navigation.

## Validation

The final-source workspace gate passed **932 tests with five ignored** after the
unsupported-summary compatibility adjustment. All six adapter/archive tests
pass, including the unsupported-wrapper regression; the explicit retained
presentation gate also passes. That gate reads seven existing captures without
executing missions and verifies:

- Flat, uphill and downhill direct controls have no correction or handoff.
- Late ridge has H1 at step 2426 and exact time 2426/120 s, with segment ranges
  0–12.6, 12.6–2426/120 and 2426/120–34 s.
- Successive rising has handoffs at steps 2006 and 2774.
- Reference plateau has handoffs at 2820, 3136 and 3226, and ends at 4886.
- Near target stops at time zero, with no executed segment.
- In-memory tampering of continuity, ownership, next-cycle state and missing
  selected corrections produces explicit errors.

Nine renderer tests cover escaping, safe links, exact off-sample endpoints,
synthetic odd/partial/empty states, repeated handoffs, selection hooks and
unsupported indexing. CLI and normal-navigation tests pass in the workspace
gate. Format, whitespace and browser-script syntax checks pass on final source.

Strict Clippy reports only the unchanged `single_element_loop` baseline at
`pd-eval/src/route_capability.rs:274`. The final workspace/all-target Clippy gate
passes with that lint alone allowed and all other warnings denied.

Chrome 154 was available at `/opt/google/chrome/chrome`, despite not being on
PATH. Final browser evidence is in
`outputs/reports/waypoint-v2/presentation_20261002_v1_browser_checked`:
16 real page/width checks and 16 screenshots cover the index plus the seven
selected missions at 1280 and 390 px. Selection, SVG marker switching, native
keyboard activation, disclosure, exact H steps/clocks, landing-symbol honesty,
link availability and absence of external assets or page errors pass.
All 153 unique navigation/source links across the 31 rendered pages return
successful HTTP responses through the LAN address.
Primary inspected actual desktop/mobile screenshots before expanding the suite
and checked the plateau, index and no-departure views afterward.

These are presentation and browser checks of retained physical captures, not a
new flight proof or evidence that the user accepted the UI. The four older workspace ignores
remain unchanged; the added ignore is the explicit local-capture presentation gate.

## Reproduce and next step

Use a fresh output root; existing roots are never overwritten:

```sh
rtk proxy target/debug/pd-eval waypoint-v2-report \
  --suite-root outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a \
  --output-dir outputs/reports/waypoint-v2/NEW_PRESENTATION
```

The existing `pdlab-reports` tmux server still serves `outputs/` on
`0.0.0.0:8000`; it was not restarted. No commit, push, deployment, planner/default
promotion or numerical tuning occurred. The worktree retains the earlier plan
and progress edits plus this implementation.

The next step is review of the revised navigation and additive-annotation design.
After approval, preview the familiar rich late-ridge report and navigation before
expanding the suite or switching future output. The rejected standalone UI is
not ready for acceptance merely because its native/browser checks pass. Review
and commit remain separately requested actions; planner research is not reopened.
