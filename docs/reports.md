# Report workflow

[Documentation home](README.md) · [Evaluation](evaluation.md) · [Development](development.md)

Run commands from the repository root. Serving exposes existing local files;
refreshing rewrites generated pages. Read-only acceptance checking is a separate
operation. None of these pages are checked into Git, and a fresh checkout has
no accepted report site until local evidence is generated or supplied.

Input-only V2 calibration uses the same maintained report shell through the
[native input report](procedural_input_audit.md#maintained-local-input-report).
Its [current local edition](procedural_challenge_calibration.md#backend-decision-and-readable-review)
leads with distributions/joint counts, then collection/morphology/barrier-band
browsing with twelve lazy previews in one open band. These local pages have no
flight outcomes and do not refresh accepted editions, home/library navigation or
the report server. A geometry band is a presentation threshold, not difficulty.

## Report Serving

Start at `/` or `/reports/`: **Browse missions** leads with terminal control,
direct transfers and complete procedural terrain. Authored waypoint and planner
probes remain separate approach-specific diagnostics. **Recent reports**
automatically lists saved report entrypoints, newest generated page first.
**Browse all reports** groups them by mission family and provides the
full catalogue with search, topic/type filters, optional missing fixture entries
and newest/name ordering. Topic pages retain their curated benchmarks and review
guides before recent reports. Links use the current server host;
you do not need to copy capture URLs from chat or edit a fixture for each run.

Discovery scans up to three collection/edition directory levels beneath
`outputs/eval`, `outputs/research` and `outputs/reports/{eval,setups,runs,replays}`.
It finds `index.html` and `report.html`, deduplicates existing curated links,
skips symlinks, caches, mission `runs`/`cases`, `details`/`previews` assets,
source clones and hidden paths,
and reads at most 64 KiB of a page header for its reported title/chips. Individual
missions remain accessible inside their rich batches; raw directories without a
report entrypoint are not reports. Unknown subjects remain Unclassified.

**Updated** is the HTML file's modification time, displayed in UTC. It measures
the report view, not the original flight date: regenerating an old view moves it
up. Header summaries are reported metadata, not a new integrity/landing check.
Presence and recency do not promote a capture or prove acceptance. The accepted
**current planner V2 batch**, retained policy-3 sweep and experimental captures
keep their separate labels and selectors. Rich reports and receipts are never
rewritten by discovery.

The retained [recovery pass](ballistic_recovery_consistency_results.md)
has its 829/1000 full batch, 48-world focus and diagnostic probe in this catalogue.
The preceding 797/1000 phase-transition checkpoint retains its pinned mission
review guides. `home_highlights` now owns only those curated guides, not which
report is latest; older captures and their honest limitations remain available.

`scripts/serve-reports start` builds the current evaluator and starts the
navigation-aware [HTTP server](../scripts/report_server.py). On a navigation
GET/HEAD it checks bounded entrypoint metadata (at most once per second) and
runs only `pd-eval refresh-reports --home-only` when sources changed. It does not
execute flights, replays, full report regeneration or accepted publication.
Navigation responses disable caching; rich report requests stay ordinary static
file reads. A refresh failure returns HTTP 503 and logs the error instead of
quietly claiming a stale catalogue is current.

Use `PDL_REPORT_AUTO_REFRESH=0` for static-only serving; custom output roots also
remain static. A server that was already running the old implementation needs
one explicit `scripts/serve-reports restart`. Without the server, a one-shot
`pd-eval refresh-reports --home-only` still generates a static catalogue.

The preceding [retained countdown experiment](ballistic_landing_countdown_results.md)
uses the common rich reports at
`/eval/planner_v2_random_terrain/capture-landing-countdown-preservation-20261009-v1/`.
All 48 records verify, but it adds no landings (7/16). Start with 084/715;
use **Jump to decision → maintained_landing_entry**, then final
**short_command_obstruction** to distinguish actual state from a rejected
prediction. Compare successful early-only 715 in
`/eval/planner_v2_random_terrain/capture-landing-countdown-early-preservation-20261009-v1/`.
No accepted navigation/publication or server restart occurred.

The preceding [landing-duration experiment](ballistic_landing_duration_results.md)
retains the same common rich templates at
`/eval/planner_v2_random_terrain/capture-landing-duration-preservation-20261008-v1/`.
Start with gained 268, then inspect 084/142's late descent failures and 715's
edge-clearance stop. **Jump to decision → maintained_landing_entry** identifies
the newly opened landing transition. The final **short_command_obstruction**
shows the rejected prediction separately from the still-flying actual state;
a predicted crash is not an executed crash. All 44 records verify, but the
three original subjects still do not land. This is local diagnostic evidence,
not accepted-site publication or authorization to start/restart a server.

The preceding [local waypoint experiment](ballistic_local_waypoint_results.md) uses
the common rich templates. Start with 715 at
`/eval/planner_v2_random_terrain/capture-waypoint-local-early-target-20261008-v1/`,
then compare its failed combined flight at
`/eval/planner_v2_random_terrain/capture-waypoint-local-combined-20261008-v1/`.
**Jump to decision → destination_reacquired_before_waypoint** shows the destination
goal, previous waypoint and projected braking room. W2 is superseded, not an
actual H2. The full panel is at
`/eval/planner_v2_random_terrain/capture-waypoint-local-preservation-20261008-v1/`;
inspect lost 084/142. Local-height summaries show both the local basis and former
incoming-corridor maximum, without removing existing panels. These local captures
are not newly published accepted navigation; defaults/site remain unchanged.

The preceding [waypoint-entry experiment](ballistic_waypoint_entry_results.md) uses
the common rich templates at
`/eval/planner_v2_random_terrain/capture-waypoint-entry-preservation-20261008-v1/`.
Review 349's lower-effort ballistic waypoint flight, then 807/139/268's losses.
The focused recovery-only comparison is at
`/eval/planner_v2_random_terrain/capture-waypoint-entry-recovery-20261008-v1/`:
349's **waypoint_acquisition_continues** keeps its higher-energy entry and lands.
The additive teal **Predicted cutoff coast · not flown** trace shows the coast
estimated after the remaining burn, separate from the idealized arc, ballistic
coast now and actual flight. Predictions are not restored states or handoffs.
All previous panels remain. These local captures are not accepted-site
publication or selected by report-home navigation; defaults remain unchanged.

The preceding [ridge-aware waypoint panel](ballistic_ridge_waypoint_results.md) uses
the common rich templates at
`/eval/planner_v2_random_terrain/capture-ridge-waypoint-panel-20261008-v1/`.
Start with 807 (one-H landing), then 006/349 (lost acquisition) and 142/928
(later destination misses). **Jump to decision → waypoint_selected** shows the
chosen crest as a separate terrain marker alongside the desired arc and goal.
Goal revisions are not additional actual H. This selected experiment is not
adopted, published to the accepted site or a new 1k estimate.

The preceding [waypoint-clearance mechanism panel](ballistic_waypoint_clearance_results.md)
uses the common batch/detail templates at
`/eval/planner_v2_random_terrain/capture-waypoint-clearance-panel-20261008-v1/`.
Review 807/928's later finite stops and 484's added landing. This local opt-in
capture is not published/accepted and does not replace report-home selectors.

The earlier opt-in [terrain-correction diagnostic](ballistic_terrain_correction_results.md)
has a local common-template batch at
`/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1/`.
It is not published or selected by accepted-site navigation. Use **Jump to
decision → terrain_recovery_started / terrain_recovery_resumed** to inspect
same-goal correction. Start with `114` (direct recovery and landing), `139`
(recovery toward W1 and landing), and `034/055` (remaining recovery limits).
Selected/rejected commands and their prediction horizons are described separately
from actual H. The earlier [ballistic replan diagnostic](ballistic_replan_results.md) has a
local common-template batch at
`/eval/planner_v2_random_terrain/capture-ballistic-replan-20261008-r2/`.
It is not published as the accepted report or selected by the site navigation.
Within a rich mission detail, **Jump to decision** selects an obstruction,
waypoint selection/replacement or same-goal reacquisition. The purple star is the
selected refresh's goal; the gray star is its previous goal. Revision labels and
proposed-command traces distinguish queries from actual handoffs and flown motion.
All previous detail panels remain. Start with `084/268` for recovered waypoint
flights and `055` for a replacement followed by an unresolved correction miss.

The retained policy-3 [paired early-exit 1k run](terrain_early_exit_sweep_results.md) is
published separately at
`/reports/eval/planner_v2_random_terrain/recheck-early-exit-20261008-cycles-v5/`:
**Report home → Waypoint planning → Retained policy-3 1,000-world sweep · failure review**.
It shows **817/1000 landings and 183 stopped primary missions**, compared with
748 prior landings under the same experimental cap 24. This is not a new
campaign, held-out estimate, accepted benchmark or production-cap promotion.

The **Review remaining stops** panel opens a common review-tree subset directly:

- **Not launched · 65**: `NoClearing` before departure; start with `327/791`.
- **Stopped during obstacle clearing · 83**: one or more actual handoffs, then
  `NoClearing`; start with `280`.
- **No next target trajectory · 35**: `NoNominal` at a saved handoff; start with
  `983/516`.

These are lifecycle groups, not proven root causes or physical impossibility.
Choosing a group shows its terrain recipes, mission links and previews while
successful missions remain collapsed. Group URLs retain their `#tree-...` focus
when opened directly or followed back from a detail page. **Previous / Next
stopped case** stays inside that group, skipping successes, other stop groups
and preservation controls/repeats. The full population and rich detail pages
remain available; plots, telemetry, actual handoffs and raw captures are preserved.
Each terrain/flight preview sits with its mission link in the sticky first cell,
so it remains visible on desktop and mobile without horizontal scrolling. Long
comparison headings wrap; all numerical columns remain available to the right.

Its explicit create-only publication command is:

```sh
rtk proxy cargo run -p pd-eval-cli -- render-terrain-early-exit-sweep \
  --capture-dir /ABSOLUTE/outputs/eval/planner_v2_random_terrain/CAPTURE \
  --output-dir /ABSOLUTE/outputs/reports/eval/planner_v2_random_terrain/recheck-ID \
  --capture-base-href /eval/planner_v2_random_terrain/CAPTURE/
rtk proxy cargo run -p pd-eval-cli -- refresh-reports --home-only
```

The early-exit command uses that study's separate read-only verifier and binds
the source receipt, baseline, review counts and every published page in
`render.json`. It executes no flights or fresh replays and leaves the accepted
selector and previous report editions unchanged. It cannot admit early-exit
evidence through the original cap-sweep contract.

For the [planning-cycle diagnostic edition](terrain_planning_cycle_review.md),
add `--planning-cycles` to that command. This explicit opt-in reenacts fixed
recorded programs against source-matched dynamics; the ordinary command above
remains simulation-free. It does not run a new nominal search or mission flight.
The receipt separately records original-source programs, nominal cycles, fixed
rejected-row reenactments and engine provenance, not zero diagnostic replays.
Each detail gets a `planning-cycles.json` sidecar and a selector above the
existing trajectory plot. Default selection is the last planning cycle; switch
to Launch or After Hn to see how the proposal changes with actual velocity/fuel.
Purple is the recorded nominal, orange is a current-state unpowered ballistic
projection, green is executed E-to-H, and the red cross is a future query
obstruction. The original illustrative launch arc and all other rich views
remain separate. Start with `715`, then compare `280/983/327/999` and positive
controls `000/030/565`. Edition v3 and raw capture pages are retained unchanged.

The [paired 1k cap sweep](terrain_cap_sweep_results.md) is published at
`/reports/eval/planner_v2_random_terrain/recheck-cap-sweep-20261007-v6/`:
**Report home → Waypoint planning → Previous 1,000-world terrain sweep · cap comparison**. It uses the
common batch template, with **primary worlds → terrain recipe → outcome →
mission** branches, separate preservation controls/repeats, and the cap-6 versus
cap-24 comparison. Detail copies preserve all rich plots, telemetry, diagnostics
and actual H annotations; navigation, artifact links and an additive saved
planning summary change, not the embedded flight numerics. `000`,
`030`, `565`, `280` and `611` are linked under **Start here** for a direct
landing, a new corrected landing, a 13-handoff landing, and two honest finite
planning stops respectively. `988`/`980` show unlaunched missions whose proposed
departure is blocked; `999` is unlaunched but blocked on its proposed approach.
These are not executed takeoff crashes.

Common preview markers distinguish **hollow grey circle: not launched**,
**amber pause: saved airborne endpoint**, **red X: physical crash**, and
**green circle: mission success**; other stopped outcomes use an amber square.
Each marker has a tooltip. A native planner detail shows its planning stop/reason,
executed endpoint and completed handoff count separately from the per-cycle
proposed-route obstruction phase/time/location and clearing-search counts.
Planning-cycle summaries open by default for stops. A rejected query's collision
is not an executed crash, and missing nominal generation is not an obstacle
finding. Older report editions and capture HTML remain unchanged; refreshed
navigation points to the new create-only edition.
Unfinished details label landing as **Not reached**, retaining the saved
endpoint/envelope values under **Pre-landing diagnostics** rather than presenting
them as measured impact/landing quality. The dashed idealized reference remains
visible but is explicitly illustrative, not the audited planner proposal.

This is an explicitly isolated cap-24 experiment, not the production default or
accepted benchmark. Its dedicated create-only publication command is:

```sh
rtk proxy cargo run -p pd-eval-cli -- render-terrain-cap-sweep \
  --capture-dir /ABSOLUTE/outputs/eval/planner_v2_random_terrain/CAPTURE \
  --output-dir /ABSOLUTE/outputs/reports/eval/planner_v2_random_terrain/recheck-ID \
  --capture-base-href /eval/planner_v2_random_terrain/CAPTURE/
rtk proxy cargo run -p pd-eval-cli -- refresh-reports --home-only
```

It requires Python 3 for the study's existing read-only saved-evidence verifier
and the authenticated original 1k baseline capture. It runs no flights, builds
no probe, performs no fresh replay and changes no accepted selector. The same
destination and URL protections described below apply. `render.json` binds the
source capture receipt and every published page. Existing destinations are
rejected; refresh-home changes navigation indexes only. Ordinary
`render-terrain-survey` admission remains restricted to its production-policy
survey contract and does not admit cap-probe identities.

The [random-terrain survey](random_terrain_survey_plan.md) has a separate
development collection at `/reports/eval/planner_v2_random_terrain/`, linked under
Waypoint planning when present. It uses the common batch/detail templates, not
the shape-only study gallery, and does not replace the accepted benchmark.
`pd-eval render-terrain-survey --capture-dir ABSOLUTE_CAPTURE --output-dir
ABSOLUTE_SURVEY_SITE --capture-base-href /eval/planner_v2_random_terrain/CAPTURE/`
is an explicit create-only report publication command with no flight execution.
The destination is the repository's exact `outputs/reports/eval/planner_v2_random_terrain`
directory or one direct `recheck-ID` child beneath it (ASCII letters, digits,
hyphens and underscores in ID). Existing targets and symlink ancestors are
rejected; existing pages are never overwritten. Capture-specific rechecks retain
the stopped collection rather than replacing it. `refresh-reports --home-only`
can then connect the published collection to the ordinary navigation without
rewriting flight reports or the accepted selection.
The [completed full sweep](random_terrain_full_sweep_results.md) is at
`/reports/eval/planner_v2_random_terrain/recheck-20261007-v4-full/`: from the
report home choose **Waypoint planning → Full random-terrain sweep**. It contains
100 direct random landings, separate controls with actual one/three handoffs and
five repeats. The earlier stopped collections remain linked and unchanged.

The [harder-terrain challenge results](terrain_challenge_results.md) are at
`/reports/eval/planner_v2_random_terrain/recheck-challenge-20261007-main-v2/`:
**Waypoint planning → Harder terrain challenge**. Coverage distinguishes 21/64
blocked-route landings from 36/36 clear-route landings. The common tree/detail
templates retain actual handoffs, including four- and six-handoff landings and
finite stops after clearing. The separate 24-case calibration has its own batch;
the earlier presentation-defect stop remains under history. For a proven finite
stop between sampling ticks, the last plotted observation is derived from the
exact saved endpoint and labeled display-only; the raw sample ledger is unchanged.

The subsequent [handoff-room results](handoff_room_results.md) improve the reused
challenge to 68/100, but its native captures are unpublished and do not replace
the earlier linked challenge or accepted benchmark. Their common rich detail
pages add accepted query H states and braking-room estimates to the collapsed
clearing diagnostics. These are query heuristics, not executed waypoints or
landing proofs; existing actual-handoff plots and annotations remain intact.

Generated reports live under `outputs/` and can be served locally with:

```bash
rtk proxy ./scripts/serve-reports start
```

The script starts a simple HTTP server inside a named detached `tmux` session
and serves `outputs/` on `0.0.0.0:8000` by default. The root URL now lands on a
generated `outputs/index.html` page, and `/reports/` remains the clean
report-only subtree. The printed LAN URL resolves automatically when available.
To check the navigation server without changing the running server or saved
reports, run `rtk proxy python3 -B scripts/test-report-server.py`; its HTTP tests
use temporary files and an ephemeral loopback port.

The default bind exposes all of `outputs/`, including raw captures, to reachable
LAN clients without authentication. Use a trusted network; for loopback-only
serving, set `PDL_REPORT_HOST=127.0.0.1` when starting a stopped server. Other
overrides are `PDL_REPORT_PORT`, `PDL_REPORT_SESSION` and `PDL_REPORT_ROOT`.
Changing an environment value does not reconfigure an already-running tmux server.

Start at `/` or `/reports/`, then browse by mission. The library retains that
grouping while you search, filter and reorder reports. Subject navigation remains
available below the mission entrypoints:

- **Waypoint planning** retains the current V2 batch, historical V2 previews,
  the legacy V1 planner baseline and related studies, followed by recent reports.
- **Flight and landing control** groups terminal landing, direct transfers and
  following authored waypoint routes.
- **Research and history**, **Browse all reports** and **Raw data** are
  secondary routes. The library exposes type, status and availability; analytical
  setups are not simulation results.

The existing `/reports/guidance/` scorecards remain available within this
hierarchy. `/reports/eval/` is a compatibility entrypoint for the report library.

The guidance catalog treats smoke matrices as the primary controller-iteration
surface. Full-seed packs are supporting reliability evidence; focused frontier
and experimental packs remain visible in the all-reports index without
overstating them as the project scorecard.

Useful commands:

```bash
rtk proxy ./scripts/serve-reports status
rtk proxy ./scripts/serve-reports attach
rtk proxy ./scripts/serve-reports stop
```

This is intentionally explicit. Agent skills or local tooling can call the same
script when they need a report server, but the repo-owned script remains the
canonical entrypoint.

Generated single-run, replay, and batch outputs also maintain `latest` links
under `outputs/` when written through the project CLIs, for example:

- `outputs/runs/latest/report.html`
- `outputs/replays/latest/report.html`
- `outputs/eval/latest/summary.json`
- `outputs/eval/<pack>/runs/latest/report.html`

Stable HTML entrypoints also live under `outputs/reports/`, for example:

- `outputs/reports/index.html`
- `outputs/reports/guidance/index.html`
- `outputs/reports/guidance/terminal/index.html`
- `outputs/reports/guidance/transfer/index.html`
- `outputs/reports/guidance/waypoint/index.html`
- `outputs/reports/guidance/planner/index.html`
- `outputs/reports/eval/index.html`
- `outputs/reports/eval/planner_v2_lab_suite/index.html`
- `outputs/reports/eval/conservative-ballistic-ridge-f6-integration-v1/index.html`
- `outputs/reports/setups/conservative-ballistic-direct-bridge-v2/index.html`
- `outputs/reports/runs/latest/`
- `outputs/reports/eval/latest/`

The root and report-home URLs use the same mission-first navigation:

- `outputs/index.html`
- `outputs/reports/index.html`

Start with terminal control, direct transfers or complete procedural terrain.
Subject pages, research/history and raw data remain available below the mission
entrypoints. Report type and review status are shown separately.

To apply the current report templates to existing captures without running
simulations:

```bash
rtk proxy cargo run -p pd-eval-cli -- refresh-reports
rtk proxy cargo run -p pd-eval-cli -- refresh-reports --all
```

To refresh navigation and maintained scorecard indexes without rewriting
detailed report bodies, flight captures or outcome summaries:

```bash
rtk proxy cargo run -p pd-eval-cli -- refresh-navigation
```

The narrower home-navigation refresh also regenerates configured topic, library,
history and raw-data indexes, but does not regenerate maintained scorecards:

```bash
rtk proxy cargo run -p pd-eval-cli -- refresh-reports --home-only
```

`fixtures/reports/report_navigation.json` describes the explicit topic map.
The library also lists stable report entries with unknown topics as Unclassified;
it does not recursively publish research directories.
`fixtures/reports/navigation_preview.json` pins the historical preview links
relative to `outputs/reports/`. This selection does not change planner defaults
or designate an accepted flight capture. Missing preview files are shown as
unavailable, without substituting historical reports.

The historical V2 navigation edition preserves rich report plots and payloads and
adds return/previous/next links in new copies. Only Late ridge has waypoint
annotations in that edition; the other full reports are labelled as not enhanced. Earlier
report editions and original captures remain unchanged.
New V2 batch mission details use the existing rich report with executed handoff
annotations for every simulated case. The older selected preview remains
presentation history; it is not the current evaluation batch. Retained preview
editions and their original source evidence remain unchanged.

The conservative-ballistic setup and F6 integration pages remain dated research
evidence at their original paths. Their standalone recomputation commands and
report builders are retired; they are not the active planner or a prerequisite
for it. Common controller batch/detail regeneration remains supported.

The default refresh covers packs in the guidance catalog and configured batch
navigation, including the selected native V2 capture. `--all` considers every
fixture-backed pack and skips packs without captures. Controller pages rebuild
from existing JSON bundles and retain a recorded comparison when its basis is
still readable. Native V2 regeneration validates and republishes the selected
accepted capture, including its render receipt; it does not select a new capture
or rerun missions. Both leave raw capture and `summary.json` evidence unchanged.

To deliberately select an accepted saved native capture rather than run missions:

```sh
rtk proxy cargo run --release -p pd-eval-cli -- publish-planner-v2 --dir CAPTURE_DIRECTORY
```

This is a publication command: it checks acceptance and updates the current
selector and common report site. For inspection without those writes, use the
[saved-capture checks](evaluation.md#planner-evaluation-v2-default).

Browser validation is optional and separate from the ordinary developer gate.
With an existing report server and an explicitly supplied disposable local Chrome
debugging endpoint:

```sh
rtk proxy node scripts/check-planner-v2-browser.mjs \
  --cdp-url http://127.0.0.1:PORT/ --root-url http://127.0.0.1:8000/
```

This opens and closes its own tab; it does not start a browser/server or regenerate
reports. Adding `--output-dir NEW_DIRECTORY` writes create-only screenshots and a
receipt. Automated navigation checks are not a substitute for your visual review.
