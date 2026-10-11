# Saved campaign presentation editions

[Report workflow](reports.md) · [Development](development.md) ·
[Refinement results](report_presentation_refinement_results.md)

The common campaign and comparison views are presentation components in
`pd-report`. They consume neutral saved projections, without importing an agent,
planner, solver or learning package. Existing projections remain readable;
optional versioned preview metadata adds terrain, recorded flight and endpoint
context when available. Missing data stays explicitly unavailable.

## Read a procedural comparison

Outcome bars count every scheduled attempt, including unsupported and failed
attempts. Integrity and complete replay are reported separately. Only the complete
physical/mission/execution/integrity/replay tuple qualifies as a verified landing.
Click an outcome or a review shortcut to select missions, then expand a coverage
group and a mission. Every approach remains available for that input. Compare A/B
selects two previews with shared axes; it does not select the best repeat.

Coverage cells focus their exact family, direction, stratum and approach. Search
accepts a mission number, terrain label or physical identity. The URL retains
filters, comparison choices, open groups and the selected mission. Detail links
restore that context; previous/next follows matched missions for the same
approach. Unsupported details show terrain and the target without inventing a
flight. Recorded but unstarted episodes are a separate outcome.

Rich details lead with outcome and endpoint context, then the trajectory and time
plots. Run summary, phases and capture provenance remain expandable. Spatial
modes, linked hover, events, planning cycles, decision navigation and executed H
annotations remain in the rich payload. Static SVG trajectory, velocity and
sampled commanded-throttle plots remain available when Plotly cannot load.

## Create an edition from retained evidence

Rendering is a separate, explicitly requested write operation. The adapter checks
saved capture byte seals, physical identities, summary/projection agreement and
the retained independent-verification summary hashes. It preserves the original
rich payload, including control-update presentation semantics. It neither loads a
checkpoint nor simulates or reruns replay. Its receipt identifies the saved proof;
this is not fresh replay evidence.

```sh
rtk proxy cargo build --release -p pd-report --example refine_comparison
rtk proxy target/release/examples/refine_comparison \
  SAVED_COMPARISON_DIRECTORY NEW_EDITION_DIRECTORY \
  /SOURCE_DIRECTORY_ON_REPORT_SERVER/ /NEW_EDITION_ON_REPORT_SERVER/
```

Use a new directory outside the original capture. Outputs are create-only:
`report.html`, `comparison-input.json`, `navigation.json`, `render.json`, compact
`previews/` and rich/status `details/`. The source must contain the completed
comparison projection, campaign summaries, sealed evaluations, original rich
details and `independent-verification.json`. All originals stay in place.

For an explicitly labelled inspection slice, append `--mission-limit N`; this
limits mission indices within each constituent campaign and does not become a
full-population result. Use `refine_campaign` for a single saved campaign; its
three arguments are the source directory, new output directory and source URL
base. A campaign-only edition keeps original detail links.

Branches materialize on demand and previews load after they become visible. The
comparison carries compact outcome/metric projections, without full trajectory
arrays, checkpoint reads or capture fetches at page startup. Its navigation index
contains the same mission occurrences and is fetched only by a detail page.

## Validate presentation

Ordinary tests use source fixtures and synthetic projections, without saved
captures:

```sh
rtk proxy cargo test -p pd-report --all-targets
rtk proxy node --test scripts/test-report-presentation.mjs
rtk proxy node scripts/check-planner-development.mjs
```

For a real saved edition, supply the already running report server and an
explicit disposable local Chrome debugging endpoint. This check creates and
closes its own tab; it does not manage the report server or regenerate evidence.

```sh
rtk proxy node scripts/check-report-presentation.mjs \
  --cdp-url http://127.0.0.1:PORT/ \
  --edition-path /NEW_EDITION_ON_REPORT_SERVER/ \
  --output-dir NEW_BROWSER_RECEIPT_DIRECTORY
```

It checks desktop/mobile accounting, lazy previews, exact coverage focus,
filter/search behavior, shared mission review, context round trips, rich tools,
unsupported status and deliberately blocked Plotly fallback. Screenshots and
five warm interaction timings per viewport are retained with the receipt.
Numerical preservation, archived compatibility and source identity checks remain
separate from this browser evidence.
