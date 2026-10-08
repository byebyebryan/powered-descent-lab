# Report workflow

[Documentation home](README.md) · [Evaluation](evaluation.md) · [Development](development.md)

Run commands from the repository root. Serving exposes existing local files;
refreshing rewrites generated pages. Read-only acceptance checking is a separate
operation. None of these pages are checked into Git, and a fresh checkout has
no accepted report site until local evidence is generated or supplied.

## Report Serving

The [paired 1k cap sweep](terrain_cap_sweep_results.md) is published at
`/reports/eval/planner_v2_random_terrain/recheck-cap-sweep-20261007-v6/`:
**Report home → Waypoint planning → 1,000-world terrain sweep**. It uses the
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
rtk proxy cargo run -p pd-eval -- render-terrain-cap-sweep \
  --capture-dir /ABSOLUTE/outputs/eval/planner_v2_random_terrain/CAPTURE \
  --output-dir /ABSOLUTE/outputs/reports/eval/planner_v2_random_terrain/recheck-ID \
  --capture-base-href /eval/planner_v2_random_terrain/CAPTURE/
rtk proxy cargo run -p pd-eval -- refresh-reports --home-only
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

The default bind exposes all of `outputs/`, including raw captures, to reachable
LAN clients without authentication. Use a trusted network; for loopback-only
serving, set `PDL_REPORT_HOST=127.0.0.1` when starting a stopped server. Other
overrides are `PDL_REPORT_PORT`, `PDL_REPORT_SESSION` and `PDL_REPORT_ROOT`.
Changing an environment value does not reconfigure an already-running tmux server.

Start at `/` or `/reports/`, then browse by subject:

- **Waypoint planning** starts with the current V2 batch, followed by historical
  V2 previews, the legacy V1 planner baseline, and related analytical studies.
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

The root and report-home URLs use the same topic hierarchy:

- `outputs/index.html`
- `outputs/reports/index.html`

Start with **Waypoint planning** for terrain-aware route selection, or **Flight
and landing control** for landing, direct transfer and following authored
waypoints. Research/history, the searchable report library and raw data are
secondary destinations. Report type and review status are shown separately.

To apply the current report templates to existing captures without running
simulations:

```bash
rtk proxy cargo run -p pd-eval -- refresh-reports
rtk proxy cargo run -p pd-eval -- refresh-reports --all
```

To refresh navigation and maintained scorecard indexes without rewriting
detailed report bodies, flight captures or outcome summaries:

```bash
rtk proxy cargo run -p pd-eval -- refresh-navigation
```

The narrower home-navigation refresh also regenerates configured topic, library,
history and raw-data indexes, but does not regenerate maintained scorecards:

```bash
rtk proxy cargo run -p pd-eval -- refresh-reports --home-only
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
rtk proxy cargo run --release -p pd-eval -- publish-planner-v2 --dir CAPTURE_DIRECTORY
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
