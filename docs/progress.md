# Progress

## 2026-07-14

### Report information architecture checkpoint

- Centralized stable report-site generation in `pd-report::site` so
  `pd-cli` and `pd-eval` publish through the same path, latest-link, and
  index rules.
- Added a curated guidance scorecard for terminal, direct-transfer, and
  waypoint responsibilities. Primary smoke evidence is distinct from
  supporting full-seed evidence; the complete eval index groups maintained,
  diagnostic, and fixture reports without treating them as one verdict.
- Reorganized batch pages around outcomes and selector coverage. Context and
  guidance diagnostics are collapsed by default, comparison state remains
  explicit, and dense review trees keep a sticky selector column with
  horizontal containment on narrow screens.
- Polished single-run reports with readable mission titles, batch/run
  breadcrumbs, mission-first outcome cards, phase bands, and separate
  `Mission | Guidance | Speed | Throttle | Vectors` trajectory modes.
  Waypoint geometry remains available without obscuring the default actual
  trajectory.
- Added `pd-eval refresh-reports [--all]` to regenerate existing batch and run
  HTML without simulation. The catalog refresh rebuilt 15 batch pages and
  9,675 run pages in parallel; before/after hashes confirmed every captured
  `summary.json` remained unchanged.
- Browser checks covered terminal and direct-transfer desktop pages plus a
  narrow waypoint page. This slice changes report organization and
  presentation only, not controller, scenario, evaluation, or persisted
  evidence semantics.

## 2026-07-13

### Documentation alignment checkpoint

- Reconciled the active README, architecture, roadmap, and suite docs with the
  closed guidance baseline and focused structural cleanup.
- Replaced the hypothetical report/repo layout with the implemented five-crate
  workspace, explicit `pd-report` ownership, pack/scenario fixture split, and
  generated-output boundary.
- Updated controller module ownership to include telemetry emission and sibling
  test modules without changing any public compatibility claims.
- Refreshed smoke wall-clock provenance and the broader all-radius controller
  compute checkpoint from the latest no-reuse runs. Outcome counts and the
  single known post-contract landing residual remain unchanged.
- Kept historical progress and tuning sections intact; only active status and
  forward-looking statements were changed.

### Guidance structural cleanup checkpoint

- Added a transfer telemetry characterization test that locks the exact base
  metric set, boost-only metric set, selected scoring label, and maintained
  `legacy_endpoint` fallback before moving production code.
- Moved terminal and transfer controller tests into sibling test modules. Test
  names, fixtures, private access, and focused Clippy allowances are unchanged;
  current controller coverage is `146 / 146`.
- Isolated transfer and waypoint metric emission, prediction/audit
  serialization, and waypoint handoff markers in `transfer/telemetry.rs`.
  Direct-transfer diagnostics, waypoint snapshots, lifecycle state, phase
  selection, scoring, thresholds, and candidate ordering remain in
  `transfer/mod.rs`.
- Replaced the duplicate open-loop builder and terminal-handoff metric writers
  with one module-private sink over already-computed guidance products. No
  public controller, config, phase, telemetry, marker, schema, or artifact
  contract changed.
- Fresh eight-worker `--no-reuse` evidence preserves:
  - terminal clean and trajectory-error records exactly after excluding bundle
    paths
  - direct route-angle/radius transfer at `297 / 297`
  - route-wide turn landing/contract at `135 / 135` for both
  - route-wide ordered landing/contract at `45 / 45` for both
  - full nominal turn landing/contract at `540 / 540` for both
  - full nominal ordered landing/contract at `180 / 180` for both
  - all-radius turn contract at `405 / 405` and landing at `404 / 405`
  - all-radius ordered landing/contract at `135 / 135` for both
- On the exact all-radius turn workload, update count remains `1,368,932`.
  Controller compute changes from `285.17us` mean and `784us` p99 at `739a0c4`
  to `271.18us` mean and `719us` p99 at `e0854c0`.
- This finishes the focused cleanup. Further state-machine or terminal API
  decomposition is deferred until waypoint planning establishes a concrete
  shared seam.

### Guidance consolidation checkpoint

- Added an explicit compatibility contract for controller JSON, built-in
  aliases and IDs, phase strings, telemetry keys, markers, and schema `33`
  artifacts before moving code.
- Split terminal guidance into `terminal/` with a typed local terrain-clearance
  request, and split transfer ownership out of the controller registry.
- Isolated pure waypoint geometry/capture prediction in
  `transfer/waypoint.rs`. Frozen pathwise and recoverability-weighted boost
  experiments now live behind `transfer/experimental.rs`; their four compare
  packs are diagnostic-only rather than reference/core/stress evidence.
- Strict Clippy passes for all `pd-core` and `pd-control` targets. Unit coverage
  remains `22 / 22` core and `145 / 145` controller tests.
- Fresh regression-policy checks preserve terminal smoke outcomes, direct
  transfer at `297 / 297`, balanced turn landing/contract at `81 / 81`, and
  maintained ordered landing/contract at `27 / 27`.
- Guidance v1 is now structurally reconciled. Waypoint planning remains the
  next Phase 3 slice; controller tuning should resume only for a specific
  general hypothesis against the maintained gates.

### Waypoint guidance v1 closure checkpoint

- Added paired full-seed nominal closure packs for both final landing and the
  route contract. The turn matrix covers `540` runs per goal and the ordered
  double-bend matrix covers `180` runs per goal.
- Expanded the maintained waypoint corpus across `short | nominal | long`
  radius tiers. Capture radii scale with route geometry, and double-bend speed
  caps scale with the square root of route radius so the same physical
  contracts remain meaningful without selector-specific holes.
- Final contract-valid waypoint states now carry a terrain-blind terminal
  recoverability estimate. A recoverable final handoff enters terminal guidance
  directly; non-recoverable or missing evidence keeps the previous transfer
  fallback.
- Batch schema `33` exposes the final-handoff required acceleration ratio and
  recoverable-run count in JSON and review tables. This is a kinematic estimate,
  not a replacement for final landing evidence.
- Short-radius contract loss was traced to fixed full-throttle source
  clearance, which crossed the first-leg energy envelope before guidance could
  shape the route. Waypoint launch now regulates upright speed from the
  immutable inbound-leg length, with an `8m/s` floor. The rule does not inspect
  route/profile labels, payload, seed, terrain, or mission timeout.
- Fresh eight-worker, `--no-reuse`, no-comparison schema-33 evidence at
  `a5ecbae`:
  - full nominal turn landing and contract: `540 / 540` for both
  - full nominal ordered landing and contract: `180 / 180` for both
  - all-radius turn contract: `405 / 405`
  - all-radius ordered contract and landing: `135 / 135` for both
  - all-radius turn landing: `404 / 405`, with `0` invalidations
- The one retained landing residual is
  `single_gentle_bend_v1/full/r-30/short/seed 02`. It satisfies the waypoint
  contract and then crashes during final recovery, so it remains a visible
  touchdown-reliability frontier rather than weakening waypoint acceptance.
- An actuated reserve-triggered replan experiment did not improve outcomes and
  drove plan-revision churn as high as `211`; it was removed. Fixed launch-speed
  probes established that launch energy needed to scale with route geometry.
- This closes terrain-blind waypoint guidance v1 over the preplanned maintained
  corpus. The next Phase 3 slice is waypoint planning: produce terrain-valid
  route geometry and arrival envelopes for the existing guidance contract.

### Waypoint final-leg recovery checkpoint

- Closed the route-wide post-handoff landing frontier without changing the
  waypoint contracts or adding route/profile branches.
- Fixed two transfer phase leaks found in failed traces:
  - completed waypoint routes keep terminal ownership after final handoff
  - waypoint boost scoring cannot cut thrust while its local corridor is active
    or terminal braking reserve is already exhausted
- Preserved standalone direct-transfer behavior: direct routes may reacquire
  source clearance after a premature terminal entry and retain their existing
  coast-cut policy outside an active corridor.
- Replaced the fixed `12mm` safe-contact penetration edge with the larger of
  `12mm` and one physics step of measured normal plus rotational closing
  motion. Existing speed, attitude, angular-rate, and pad limits remain hard;
  the former `3.286m/s` impact still classifies as a crash.
- Final-waypoint plan creation now evaluates contract-valid actuated handoff
  states against terrain-blind terminal dynamics. Recoverable states rank
  ahead of over-authority states, then by required acceleration ratio, before
  the existing waypoint rollout ordering. If no recoverable alternative
  exists, the original selection is preserved.
- Fresh eight-worker, `--no-reuse`, no-comparison evidence:
  - route-wide turn landing/contract: `135 / 135` for both, `0` invalidations
  - route-wide ordered landing/contract: `45 / 45` for both, `0` invalidations
  - balanced turn landing/contract: `81 / 81` for both
  - maintained ordered landing/contract: `27 / 27` for both
  - direct route-angle/radius transfer: `297 / 297`, `0` invalidations
- Route-wide turn landing mean/max simulated time is `56.63s / 79.64s`;
  ordered landing is `56.06s / 74.36s`. Aggregate turn-pack controller compute
  is `267us` mean, `432us` p95, and `635us` p99 over `463,272` updates.
- The maintained terminal smoke checkpoint is unchanged: its current lane has
  `168 / 168` core scored successes, plus `3 / 12` frontier successes and `9`
  analytically impossible cases.
- The next waypoint slice is corpus expansion, especially deliberate radius
  tiers or waypoint planning. The former full-payload outer-angle final-leg
  failures are no longer the active controller frontier.

### Waypoint route-angle coverage checkpoint

- Added separate route-wide landing and contract packs while preserving the
  fast `81`-run turn and `27`-run sequence gates.
- The new nominal-radius matrices cover
  `r-60 | r-30 | r00 | r+30 | r+60`, all payload tiers, and smoke seeds:
  - balanced turn landing and contract: `135` runs each
  - double-bend sequence landing and contract: `45` runs each
- Fresh schema-32 evidence with eight workers, `--no-reuse`, and no comparison
  basis:
  - turn contract: `135 / 135`, `0` invalidations, `10.73s` wall clock
  - turn landing: `127 / 135`, `8` crashes, `0` invalidations, `26.27s` wall
    clock
  - ordered contract: `45 / 45`, `0` invalidations, `2.85s` wall clock
  - sequence landing: `42 / 45`, `3` crashes, `0` invalidations, `11.34s` wall
    clock
- Every failed landing completed its route contract. The exact frontier is:
  - `single_gentle_bend_v1/full/r-60`: seeds `0-1`
  - `single_medium_bend_v1/full/r+60`: seeds `0-2`
  - `single_sharp_bend_v1/full/r+60`: seeds `0-2`
  - `double_bend_v1/full/r-60`: seeds `0-2`
- The next controller problem is therefore full-payload final-leg recovery at
  the outer route angles, not waypoint capture or acceptance.
- Radius coverage remains a separate design slice. A temporary selector probe
  rejected `single_gentle_bend_v1/r00/short` because `48.0m` terrain clearance
  does not exceed the `48.75m` capture-envelope requirement, and rejected all
  short-radius `double_bend_v1` routes because waypoint zero's continuation
  stop ratio is `0.842`, above the `0.75` bound. The maintained radius matrix
  should not gain asymmetric holes or weaker contracts merely to make the pack
  resolve.

### Evaluation evidence workflow checkpoint

- Replaced aggregate-preview loading of full samples and controller updates
  with a compact position-only sample decoder. Lane SVGs are cached by their
  canonical bundle paths and reused between cache and stable publication.
- Aggregate previews retain terrain, target pad, actual trajectories, waypoint
  markers, and outcome markers. They no longer deserialize the approximately
  `11GB` of controller-update JSON in the 3,024-run trajectory-error cache.
- Batch HTML is rendered once per output. Stable report-site pages reuse that
  render with a relative base URL, and raw cache reports no longer create
  public report-site mirrors.
- The eval report index now includes only entries whose raw `pack.json` ID has
  a current fixture under `fixtures/packs`. Cache internals and orphaned output
  directories remain directly accessible but no longer clutter navigation.
- On the same `terminal_traj_err_full` cache, report-only regeneration improved
  from not completing within `240s` to `32.12s`.
- Fresh or identity-matched schema-32 evidence with eight workers:
  - terminal bot-lab smoke current lane: `171 / 180` scored successes, `9`
    scored failures, and `9` invalidations
  - terminal bot-lab full current lane: `684 / 720` scored successes, `36`
    scored failures, and `36` invalidations
  - trajectory-error smoke: `689 / 720` scored successes and `31` failures
  - trajectory-error full: `2751 / 2880` scored successes, `129` failures, and
    `144` invalidations
  - direct transfer: `297 / 297` wide smoke, `1080 / 1080` full solved, and
    `108 / 108` focused steep-uphill runs
  - waypoint turn landing/contract: `81 / 81` for both gates
  - waypoint sequence landing/contract: `27 / 27` for both gates
- This checkpoint changes reporting and evidence publication only. Controller,
  scenario, fixture, and persisted batch-schema behavior remain unchanged.

## 2026-07-12

### Waypoint mission contract reset

- Replaced implicit next-leg heading targets with a planned handoff tangent on
  each maintained ordered waypoint. `double_bend_v1` uses the normalized
  inbound/outbound angle bisector at both nodes.
- Split spatial entry from contract resolution. Radius entry opens a handoff
  window; the controller and evaluator retain the active leg until the planned
  tangent/energy envelope passes or the waypoint plane becomes the deadline.
- Retiered the maintained paired sequence packs to the 27-run double-bend matrix.
  The full 27-run `late_bend_v1` matrix is retained as diagnostic-only evidence.
- Batch schema `32` reports planned tangent, immutable window-entry state,
  resolution reason, and window duration separately. Terminal handoff fallback
  recovers the entry snapshot from authoritative samples when evaluation ends
  before another controller update.
- Fresh no-reuse gates with eight workers:
  - maintained ordered contract: `27 / 27`, all `54` handoffs resolving as
    `contract_pass`
  - maintained sequence landing: `27 / 27`
  - balanced waypoint landing: `81 / 81`
  - direct route-angle/radius transfer: `297 / 297`
  - late-bend diagnostic landing: `27 / 27`; `27 / 54` handoffs recover after
    entering the window outside the contract
- Controller compute on the maintained ordered pack is `434us` p99 over
  `28,930` updates, with an isolated `11.6ms` maximum.

### Waypoint joint-state oracle checkpoint

- Added behavior-neutral capture-transition auditing. Each handoff marker now
  compares the projected actuated event state with the actual captured
  position, velocity, attitude, mass, fuel, and event time, then reevaluates
  the next leg from the actual state.
- Added a shadow two-leg oracle that ranks at most four physically passing
  current-handoff candidates, projects each into the next leg, and caches the
  best joint result once per plan revision. The oracle does not select commands.
- Batch schema `31` renders planned continuation, projected-to-actual error,
  actual-state continuation viability, and joint-search coverage in a collapsed
  `Waypoint Continuation Audit`, with exact seed evidence underneath.
- Fresh behavior-neutral gates preserve action streams byte-for-byte:
  - focused trackability: `12 / 18`, `0` invalidations
  - ordered contract: `38 / 54`, `0` invalidations
  - sequence landing: `54 / 54`, `0` invalidations
- The ordered pack observed `51` first-to-second transitions. Planned and
  actual-state continuation both pass `26 / 51`; mean projected-to-actual drift
  is `1.22m` position, `0.65m/s` velocity, and `0.92deg` attitude, with at most
  `0.033s` event-time error. The continuation model is therefore stable across
  the actual capture boundary.
- The joint oracle evaluated zero candidates in all `51` observed transitions
  and covers `0 / 16` failed ordered routes. The current geometry-derived
  reachable-event lattice is empty at the first actuated passing forecast, even
  on successful routes, so it cannot support a continuation-recovery command.
  The agreed `4 / 16` behavior gate did not open and no behavior change was
  attempted.
- Controller compute remains below the `1ms` p99 budget: focus `385us`, ordered
  contract `387us`, and sequence landing `550us`. Isolated once-per-revision
  search updates peak below `5.9ms`.
- Next work should change the oracle candidate basis or use a bounded receding
  two-leg objective. It should first prove candidate coverage in shadow mode;
  adding recovery hysteresis or route/profile branches cannot repair an empty
  target set.

### Waypoint continuation-forecast checkpoint

- Added a behavior-neutral one-leg continuation projection. After an actuated
  current-handoff forecast passes, the controller projects that event state
  into the next waypoint and evaluates the existing bounded candidate lattice
  there. The projection carries position, velocity, attitude, fuel, and mass
  forward without changing the selected command.
- Batch schema `30` records the next waypoint index, projected continuation
  contract status and reasons, outbound heading error, peak required authority,
  and count of physically passing next-leg candidates. `Waypoint Plan
  Trackability` renders the result in a dedicated `Continuation` column.
- The behavior-neutral checkpoint preserved focused, ordered-contract, and
  sequence-landing action streams byte-for-byte. A fresh ordered-contract run
  produced `52` first-handoff continuation forecasts: `26` passed, `26` failed,
  and `23` exposed at least one passing next-leg candidate.
- Rejected a bounded center-target recovery experiment that waited for two
  failed continuation forecasts, searched only the existing current-handoff
  candidate lattice, required both current and next actuated contracts to pass,
  and attempted once per plan revision. It improved ordered routes from
  `38 / 54` to `39 / 54` with no prior-success regressions, but missed the
  agreed `41 / 54` retention threshold and left the focus pack at `12 / 18`.
  The behavior change was removed.
- The result narrows the remaining design debt: continuation state is useful
  evidence, but changing only center-target velocity/time choices rarely moves
  the current handoff enough to repair the next leg. A future controller pass
  should use the telemetry to evaluate a bounded joint handoff-state solve or
  receding two-leg objective, while preserving route/profile independence.

### Reachable recovery-plan durability checkpoint

- Batch seed rows now explain failed ordered checkpoints with the failed
  waypoint, measured value, configured limit, signed margin, and whether an
  actuated passing forecast was lost before capture. Existing schema `29`
  artifacts supply the data; legacy artifacts keep the generic outcome label.
- A plan produced by the bounded reachable event-state search now retains
  ownership through transient instantaneous authority saturation while its
  reference and actuated forecasts both still pass. Expired plans, ordinary
  center plans, and never-passing plans keep the existing replacement policy.
- A broader variant that retained every actuated-passing plan reached `36 / 54`
  but regressed two previously passing first-leg `double_bend_v1 / empty / r00`
  runs. Restricting durability to `reachable_recovery` plans avoids that
  regression without route, profile, payload, seed, timeout, or grace-period
  branches.
- Fresh strict-gate results:
  - focused trackability: `12 / 18`, up from `6 / 18`
  - ordered sequence: `38 / 54`, up from `27 / 54`, with no prior-success
    regressions and distribution `0:3 | 1:13 | 2:38`
  - sequence landing: `54 / 54`
  - balanced contract and landing: `81 / 81` each
  - smooth-bend contract and landing: unchanged `21 / 27 | 15 / 27`
  - direct route-angle/radius: `297 / 297`
- The cited `double_bend_v1 / full / r-30 / nominal / seed 0000` case now
  passes both handoffs; its second-handoff heading error falls from `0.365rad`
  to `0.072rad` instead of being replaced immediately before capture.
- Remaining failed-handoff history is `9` reference never-passing and `7`
  reference pass-lost; the actuated split is `12 | 4`. Sequence-landing compute
  remains below the `1ms` budget across `181,946` updates: `243us` mean,
  `442us` p95, `559us` p99, and `2.89ms` isolated maximum.

### Reachable waypoint event-state checkpoint

- Added an actuation-aware fixed-plan forecast that advances the existing
  state-target controller at scenario control rate with gravity, fuel/mass
  change, minimum throttle, thrust and tilt allocation, and attitude-rate
  limits. The Hermite path remains the immutable reference; batch schema `29`
  reports both models and their disagreements.
- `Waypoint Plan Trackability` now summarizes whether the actuated forecast
  ever reaches a passing event plus its peak authority demand and projected
  saturation. The behavior-neutral instrumentation checkpoint preserved every
  focused and sequence action stream byte-for-byte.
- Added a bounded capture-envelope state search for confirmed never-passing
  legs. It samples inbound, turn-bisector, and outbound-aligned points just
  inside the capture circle, combines contract-valid heading/speed states, and
  physically rolls out at most `18` analytically feasible candidates. Existing
  center plans remain the fallback.
- The search runs once per plan revision only after two confirmed reference
  failures and only if the leg has never predicted a passing reference state.
  This boundary is required: allowing the actuated forecast to replace already
  reference-passing plans improved selected focus cells but regressed ordered
  route success to `17 / 54`, so that variant was removed.
- Fresh strict-gate results:
  - focused trackability: `6 / 18`, up from `3 / 18`
  - ordered sequence: `27 / 54`, up from `24 / 54`, with distribution
    `0:3 | 1:24 | 2:27`
  - sequence landing: `54 / 54`
  - balanced contract and landing: `81 / 81` each
  - smooth-bend contract and landing: unchanged `21 / 27 | 15 / 27`
  - direct route-angle/radius: `297 / 297`
- Sequence-controller compute remains below budget across `185,395` updates:
  `243us` mean, `437us` p95, `518us` p99, and `2.94ms` isolated maximum.
- The retained gain is the full `double_bend_v1`, half-payload, `r-30`
  three-seed cell. Pass-lost plans are deliberately not routed through this
  state search; their retained-plan durability remains the next distinct debt.

### Waypoint plan-trackability checkpoint

- Added explicit guidance-plan ownership and reference tracking to controller
  telemetry: plan index, revision, reason, age, reference position/velocity
  error, required acceleration ratio, and thrust/tilt saturation. Capture and
  transition markers now preserve the plan that actually owned the handoff.
- Batch summaries aggregate those values per handoff and render a collapsed
  `Waypoint Plan Trackability` table. Older artifacts remain readable through
  active-index fallback, but new reports no longer infer plan ownership from
  the current route index.
- Added `transfer_waypoint_sequence_trackability_focus`, an `18`-run,
  six-cell diagnostic pack covering representative never-passing and pass-lost
  sequence failures. Its fresh result is `3 / 18` complete routes.
- The focused evidence separates two controller problems:
  - pass-lost `double_bend_v1`, empty, `r+30` second legs require up to `3.45x`
    available acceleration and spend about `0.45-0.55s` thrust-saturated; their
    retained plans are not physically trackable even while prediction reports
    a passing state
  - representative never-passing half/full second legs remain near or below
    `1.0x` required acceleration with no thrust saturation and small reference
    errors; their failure is outbound target-state/candidate feasibility, not
    plan tracking
- Peak reference error alone is not a useful gate: successful controls can
  replan and accumulate comparable maxima. Last-passing state, required
  authority, and saturation history provide the actionable distinction.
- This batch intentionally changes no guidance decisions. Fresh sequential
  gates remain `81 / 81` balanced contracts and landings, `24 / 54` ordered
  routes, `54 / 54` sequence landings, `21 / 27 | 15 / 27` smooth-bend
  contract/landing, and `297 / 297` direct transfer. Sequence-controller cost
  is `224us` mean, `545us` p95, and `638us` p99 across `185,503` updates.
- Next design work should have two generalized parts: authority-aware reachable
  prediction/release for untrackable retained plans, and a broader reachable
  outbound-state solve for trackable legs that never produce a passing
  candidate. Neither should depend on route/profile labels.

### Sequence candidate-history checkpoint

- Batch review now aggregates guidance evidence across every controller update
  for each active waypoint, rather than preserving only the capture snapshot.
  JSON and the sequence `State Debt` cell expose whether any predicted
  handoff passed, the first/last passing time, whether that pass was lost before
  capture, and the best heading/cross-speed margins.
- The unchanged `24 / 54` ordered-route baseline has `30` failed handoffs:
  `26` never produce a trigger-pass candidate and `4` produce one but lose it
  before capture. All `30` fail outbound heading; `11` also fail outbound
  cross speed. Sequence landing remains `54 / 54`.
- Rejected four generic controller experiments after fresh `--no-reuse` runs:
  - a next-turn authority speed cap regressed route success to `21 / 54`
  - minimum normalized envelope-margin ordering reached `27 / 54`, but shifted
    the failure set to `12` never-passing and `15` pass-lost handoffs
  - pathwise cubic authority rejection regressed route success to `18 / 54`
  - a fixed `20%` replan authority reserve caused repeated horizon extension
    and collapsed route success to `2 / 54`
- None of those controller experiments are retained. The fixed fixture-level
  continuation bound already covers analytic energy sanity; another hard cap
  is redundant. The next controller design needs tracking-aware plan
  durability or a genuinely receding reachable-state solve, not a global
  candidate tie-break, fixed reserve, or route/profile branch.
- Restored sequential gates are `81 / 81` balanced contracts and landings,
  `24 / 54` ordered routes, and `54 / 54` sequence landings.

### Retained terminal braking-reserve recovery

- Confirmed the paired landing failures were terminal recovery debt rather
  than waypoint-contract failures: retained absolute arrival horizons kept
  prioritizing lateral cleanup after the remaining vertical braking reserve
  was nearly exhausted.
- Retained waypoint terminal plans now release permanently to the existing
  receding-horizon recovery when an attitude-aware vertical braking margin
  reaches zero. The margin uses remaining touchdown clearance, sink rate,
  current attitude, current thrust-to-weight, gravity, and the existing braking
  altitude/safety settings; it has no route/profile or simulation-time branch.
- Added `guidance.vertical_braking_margin_m`,
  `guidance.plan_release_reason`, and one `guidance/plan_release` marker per
  release. The balanced landing pack records `19` captured-boundary releases
  and `8` braking-margin releases, with no duplicate release events.
- Fresh `8`-worker, `--no-reuse` validation:
  - balanced turn landing and contract: `81 / 81` each, `11.68s | 2.88s` wall
    clock
  - sequence landing: `54 / 54`; ordered route status remains `24 / 54` with
    passed-handoff distribution `0:3 | 1:27 | 2:24`
  - sequence contract: unchanged `24 / 54`, `2.32s` wall clock
  - smooth `r+80` bend landing and contract: unchanged `15 / 27 | 21 / 27`
  - direct route-angle/radius regression: unchanged `297 / 297`, `46.38s`
    wall clock
  - standalone terminal smoke: unchanged `228` successes, `132` scored
    failures, and `18` invalidations
- Balanced mean simulation time rises by `0.18s` and mean fuel use by `0.10`
  percentage points. Transfer shape RMSE and post-handoff apex metrics are
  unchanged. Controller compute remains below budget at `178us` mean, `436us`
  p95, and `561us` p99 across `232,143` updates.
- The terminal recovery gap is closed for the maintained balanced and sequence
  packs. The next waypoint controller slice is second-leg route-contract
  shaping; final landing must remain a separate regression gate.

### Maintained waypoint corpus reset

- Re-audited every maintained waypoint fixture in the source-to-target route
  frame before more controller tuning. Removed world-Y terrain lifting, which
  had changed progress and turn geometry by route angle and had even introduced
  a small first-turn reversal in `late_bend_v1`.
- Maintained geometry is now invariant across route orientation and seed:
  - single gentle, medium, and sharp bends use `p = 0.55` with
    `n = 0.12R | 0.20R | 0.30R`, producing signed turns
    `-27.24deg | -43.95deg | -62.30deg`
  - `double_bend_v1` uses `(0.33, 0.20R) | (0.67, 0.20R)` and two
    `-31.22deg` turns
  - `late_bend_v1` uses `(0.33, 0.13R) | (0.67, 0.26R)`, producing
    `-0.58deg | -59.16deg` without a route-heading reversal
- Added one generic geometry validator for strict route ordering, positive
  source-side offsets, route-forward segments, monotonically decreasing
  route-relative headings, expected signed turns, non-overlapping capture
  regions, capture-volume terrain clearance, and sampled explicit
  multi-waypoint centerlines. Unsafe fixed geometry now fails resolution rather
  than being silently redesigned.
- Parked `single_dogleg_v1` as historical diagnostic geometry. Its four packs
  now require `expectation_tier = diagnostic`, carry `experimental` and
  `maintenance = parked` metadata, and are excluded from maintained reruns.
- Added `continuation_pass_through_v1`: heading, progress, cross-speed, and
  minimum-speed bounds remain `0.35rad`, `8m/s`, `20m/s`, and `10m/s`, while
  maximum speed is `52.5 | 65 | 75m/s` for short, nominal, and long routes.
  The planned `55m/s` short cap was tightened after the full-payload `-9%`
  radius seed exceeded the new continuation gate.
- Every maintained waypoint now records available outbound distance,
  optimistic stopping distance at maximum thrust and initial mass, and their
  ratio. Resolution rejects ratios above `0.75`; refreshed evidence ranges up
  to `0.742` for the full smooth-bend pack, `0.529` for balanced turns, and
  `0.668` for sequences.
- Waypoint Handoff Triage and Waypoint Sequence reports now show the planned
  progress, signed normal offset, signed turn, envelope, maximum speed, and
  worst continuation ratio. Existing unsigned resolved keys remain available
  for cached-report compatibility.
- Fresh `--no-reuse --compare-ref none` baseline, with no controller changes:
  - smooth bend landing: `15 / 27` smoke and `54 / 108` full
  - smooth bend contract: `21 / 27` smoke and `89 / 108` full
  - balanced turn landing: `75 / 81`; the six failures are sharp `r+30`
    half/full post-handoff crashes
  - balanced turn contract: `81 / 81`
  - sequence landing: `46 / 54`; ordered route status is `24 / 54`
  - sequence contract: `24 / 54`, with passed-handoff distribution
    `0:3 | 1:27 | 2:24`
  - direct route-angle/radius regression: `297 / 297`, `0` invalidations,
    `45.69s` wall clock
  - `cargo test --workspace`, `cargo fmt --all --check`, and `git diff --check`
    pass
- This intentionally resets the acceptance baseline around valid, inspectable
  waypoint plans. The next controller work should address sharp-uphill terminal
  recovery and late-bend continuation without moving planning into guidance or
  adding route/profile branches.

## 2026-07-11

### Waypoint envelope boundary tolerance

- Fixed an exact floating-point comparison in waypoint target-envelope
  validation. A candidate constructed at the configured `55 m/s` maximum could
  measure as `55.00000000000001 m/s` after scaling a normalized oblique route
  vector, causing otherwise identical seeds to select different plan classes.
- Speed, outbound-progress, and optional vertical-speed limits now allow only
  `1e-6 m/s` of numerical roundoff. A regression test reproduces the oblique
  max-speed case and confirms a meaningful `0.01 m/s` violation remains
  rejected.
- In `double_bend_v1/empty/r-30/seed 1`, initial guidance changes from the
  accidental `10 m/s`, `53.29 s` plan to the intended `55 m/s`, `12.59 s`
  plan. First-leg apex falls from `881.47 m` to `490.88 m`; the first handoff
  now passes at `0.078 rad` heading error and `3.62 m/s` outbound cross speed,
  and the run completes both waypoint contracts.
- Fresh `6`-worker-per-pack, `--no-reuse` results:
  - `transfer_waypoint_sequence_contract_smoke`: `21 / 54`, passed-handoff
    distribution `0:6 | 1:27 | 2:21`, `2.99s` wall clock
  - handoff strata: double `27 / 27` then `18 / 27`; late `21 / 27` then
    `3 / 21`
  - `transfer_waypoint_sequence_smoke`: unchanged `49 / 54`, `12.29s` wall
    clock
  - `transfer_waypoint_turn_contract_smoke` and
    `transfer_waypoint_turn_smoke`: unchanged `81 / 81`
- The remaining route failures are now more clearly second-leg feasibility
  debt rather than seed-dependent first-plan selection. Future controller work
  should preserve `21 / 54` ordered routes and `49 / 54` final landings.

### Event-aware waypoint handoff selection

- The controller now projects the existing center-target state plan as a cubic
  Hermite reference and finds its first authoritative handoff trigger: capture
  radius entry or waypoint-plane crossing. The endpoint, path correction,
  candidate grid, and terrain-blind contract remain unchanged.
- Batch schema `28` records predicted event timing, center-deadline lead,
  contract status/reasons, and full predicted handoff kinematics. The collapsed
  sequence report adds the prediction to `State Debt`; detailed handoff markers
  retain the same snapshot-owned values.
- The behavior-neutral instrumentation baseline exactly preserved `2 / 54`
  ordered routes and `49 / 54` final landings. At the last pre-trigger update,
  projection agreed with all `86 / 86` observed handoff classifications; p95
  errors were `0.16deg` heading, `0.09m/s` cross speed, and `0.08m/s` speed.
- Long-horizon projection is not reliable enough to replace initial plans: it
  omits future closed-loop path correction. Unrestricted event-aware selection
  reached `26 / 54` routes but reduced landing to `44 / 54`; cruise-first
  passing-candidate ordering recovered only `47 / 54`. Contract-interior and
  global speed/effort preferences were therefore removed.
- The retained controller preserves legacy initial-plan ordering and only
  enables contract-aware replacement inside a local `12s` predicted-event
  horizon. A replacement requires two consecutive predicted failures, a
  dynamically feasible predicted pass, and at least `10%` time-to-go or target
  velocity change. Expiry and authority recovery remain immediate.
- Fresh `12`-worker, `--no-reuse` retained results:
  - `transfer_waypoint_sequence_contract_smoke`: `15 / 54`, passed-handoff
    distribution `0:21 | 1:18 | 2:15`, `2.21s` wall clock
  - handoff strata: double `18 / 27` then `14 / 18`; late `15 / 27` then
    `1 / 15`; three strata improve and double index zero is unchanged
  - `transfer_waypoint_sequence_smoke`: unchanged `49 / 54`, with no per-run
    landing outcome changes, `8.33s` wall clock
  - regression gates: `81 / 81` single-waypoint contract, `81 / 81`
    single-waypoint landing, and `297 / 297` direct transfer
- Spatial misses remain zero. Sequence-contract replan count is `1.17` mean,
  `4` p95, and `12` max. Controller compute remains below budget: contract p99
  is `159us`; landing p99 is `551us`.
- Remaining debt is concentrated after the first failed late-bend handoff and
  at late-bend index one, where the existing candidate set often has no
  feasible trigger-pass state. The next controller hypothesis should be
  upstream/two-leg feasibility, not another global candidate tie-break or a
  route/profile branch.

### Waypoint handoff target-debt diagnosis

- Batch schema `27` adds completed-leg guidance intent to every ordered handoff:
  waypoint center, nominal and active handoff targets, target mode, desired
  velocity, signed target-deadline remainder, velocity error, feasibility,
  handoff-relative turn margin, and snapshot provenance/age. Exact controller
  transitions use marker-owned intent; evaluator-terminal handoffs merge final
  kinematics with the last matching pre-capture controller update.
- The collapsed `Waypoint Sequence` report now presents velocity error, deadline
  remainder, feasibility, and handoff margin as one compact `State Debt` cell.
  Detailed run marker hovers expose the same fields. Single-waypoint triage and
  aggregate trajectory previews are unchanged.
- Fresh `12`-worker, `--no-reuse` instrumentation baselines exactly preserve
  controller behavior:
  - `transfer_waypoint_sequence_smoke`: `49 / 54`, `6.83s` wall clock,
    `65.63s` mean sim time, and `114.61s` max sim time
  - `transfer_waypoint_sequence_contract_smoke`: `2 / 54`, `1.45s` wall clock,
    `26.04s` mean sim time, and `57.25s` max sim time
- The new evidence confirms target debt at the actual radius-entry event: many
  failures arrive with a still-positive plan deadline and substantial desired
  velocity error. A single semantic experiment moved the state target and
  candidate horizon from waypoint center/plane to the fixed inbound
  centerline/capture-radius intersection, with center/plane fallback after a
  missed entry.
- That fixed capture-surface hypothesis was rejected and removed:
  - ordered success improved `2 -> 8 / 54`, and double-bend handoff passes moved
    `18 -> 19` at index zero and `2 -> 8` at index one
  - zero-handoff failures worsened `22 -> 26`; late-bend index-zero passes fell
    `14 -> 9`, while index one remained `0`; only two of four strata improved
  - final landing regressed `49 -> 42 / 54`, mean sim time rose
    `65.63s -> 67.34s`, and max sim time rose `114.61s -> 126.24s`
- The mismatch is therefore real but not the whole controller defect. A fixed
  centerline surface point is too restrictive for a capture envelope. The next
  general hypothesis should align desired velocity and candidate timing to the
  predicted first authoritative trigger while retaining center-seeking spatial
  correction, rather than replacing the envelope with another hard point.
- Pre-event-aware retained-behavior no-regression gates were clean:
  - `transfer_waypoint_turn_contract_smoke`: `81 / 81`, `1.92s` wall clock
  - `transfer_waypoint_turn_smoke`: `81 / 81`, `7.96s` wall clock
  - `transfer_route_angle_radius_suite`: `297 / 297`, `33.65s` wall clock
  - sequence landing controller compute: `159.6us` mean, `454us` p95, and
    `627us` p99 across `215,417` updates; the isolated maximum is `2.39ms`

## 2026-07-10

### Ordered waypoint-sequence baseline

- Added `EvaluationGoal::WaypointSequence`, which evaluates every configured
  waypoint in route order on controller observation boundaries. Intermediate
  passes advance the authoritative sequence index; the first failed contract
  ends the probe; success requires all handoffs. Run schema `4` records passed
  count, total count, and first failed index.
- The controller now emits one `waypoint/handoff` marker per actual route-index
  transition. Marker metadata preserves waypoint identity, capture state,
  kinematics, turn margin, and the completed leg's replan count without
  changing controller commands.
- Batch schema `26` preserves `waypoint_handoffs[]` plus route status, passed,
  total, and first-failure fields. Existing scalar waypoint fields remain
  waypoint-zero aliases. Batch reports add a collapsed ordered-sequence table;
  detailed run plots expose the richer handoff markers.
- Added paired `54`-run smoke packs over `double_bend_v1 | late_bend_v1`,
  `r-30 | r00 | r+30`, `empty | half | full`, nominal radius, and all three
  smoke seeds:
  - `transfer_waypoint_sequence_smoke` scores final landing
  - `transfer_waypoint_sequence_contract_smoke` scores the full ordered route
- Both profiles contain exactly two increasing, non-overlapping waypoints.
  Resolution checks enforce terrain-valid centerlines and capture volumes,
  expected turn ranges, profile speed caps, and exact selector/geometry pairing
  between landing and contract packs. Guidance remains terrain-blind.
- Fresh `12`-worker, `--no-reuse` baseline, before sequence-specific controller
  tuning:
  - landing pack: `49 / 54` landings, `5` crashes, `5.93s` wall clock,
    `65.63s` mean sim time, and `114.61s` max sim time
  - ordered contract pack: `2 / 54` route successes, `52` failed checkpoints,
    `1.50s` wall clock, `26.04s` mean sim time, and `57.25s` max sim time
  - both packs agree on route quality: `22` runs pass zero handoffs, `30` pass
    one, and only the first two `double_bend_v1/r+30/full` seeds pass both
  - failures are not spatial misses. They are dominated by outbound heading;
    a smaller subset also violates outbound cross speed, and two late-bend
    second handoffs lack outbound progress.
- Final landing is therefore not sufficient evidence for waypoint navigation:
  `49 / 54` vehicles land even though only `2 / 54` preserve the planned route
  contract through both handoffs. The next controller pass should target
  general leg-transition state shaping, not profile labels or terrain cases.
- Fresh no-regression gates remain clean:
  - `transfer_waypoint_turn_contract_smoke`: `81 / 81`
  - `transfer_waypoint_turn_smoke`: `81 / 81`
  - `transfer_route_angle_radius_suite`: `297 / 297`
- Landing-pack controller compute remains small: `145us` mean, `423us` p95,
  and `586us` p99 across `215,417` updates. The isolated maximum is `2.32ms`.

### Retained waypoint terminal-horizon checkpoint

- Added post-handoff apex gain, time-to-apex, and apex lateral-offset review
  metrics. `Transfer Handoff Triage` now surfaces post-handoff climb directly,
  sorts successful cells by that signal, and selects the highest-climb seed for
  inspection instead of repeating shape RMSE from the separate shape table.
- Confirmed the shape defect was moving-horizon procrastination: the terminal
  state-target solve repeatedly selected a fresh arrival horizon, so lateral
  velocity died before the target time advanced and the craft lofted over the
  pad before descending vertically.
- Waypoint-enabled transfer controllers now retain an absolute terminal arrival
  time. The commanded horizon counts down, replans only after expiry or dynamic
  infeasibility, targets upright touchdown-center height, and releases once the
  ballistic touchdown projection is captured at the latest-safe braking
  boundary. The admission and release rules use flight state, not route/profile
  labels or the simulation timeout.
- Retention is deliberately disabled for standalone terminal and direct
  transfer. Enabling it for every terminal entry regressed the waypoint pack to
  `5 / 81`; enabling it for every transfer regressed the wide direct matrix to
  `220 / 297`. Those entry states still require receding-horizon recovery.
- Fresh `8`-worker, `--no-reuse` validation:
  - `transfer_waypoint_turn_contract_smoke`: `81 / 81`, `2.07s` wall clock
  - `transfer_waypoint_turn_smoke`: `81 / 81`, `10.35s` wall clock, `52.65s`
    mean sim time, and `70.87s` max sim time
  - `transfer_route_angle_radius_suite`: `297 / 297`, `45.99s` wall clock
  - `terminal_bot_lab_full`: `915` successes, `525` scored failures, and `72`
    invalidations, matching the current standalone-terminal checkpoint
  - `terminal_traj_err_full`: `2751` successes, `129` scored failures, and
    `144` invalidations in the fresh current-tree capture
  - `cargo test --workspace`: all tests pass
- Across the six focused empty-payload `r00/r+30` cells, mean post-handoff
  climb falls from `182.53m` to `56.13m`, mean sim time from `66.69s` to
  `45.22s`, and mean fuel use from `27.69%` to `19.57%`. Gentle `r00` falls
  from `120.85m` of climb to `0m`; the three `r+30` profiles fall from
  `348.19m | 359.12m | 267.03m` to `127.65m | 131.41m | 77.68m`.
- Mean controller-update compute on the waypoint landing pack changes only from
  `156.7us` to `161.5us`, remaining far below the `1ms` budget.
- The two residual `r+30` cells just above `120m` are candidate-density debt:
  the retained mechanism correctly follows the selected `22s` horizon, but the
  selector does not sample shorter feasible arrivals. Candidate generation is
  unchanged in this checkpoint and should be evaluated separately rather than
  hidden behind route-specific terminal tuning.

### Waypoint planner-clearance correction

- Reclassified the last waypoint crashes as a corpus-planning defect rather
  than controller terrain-recovery debt. The balanced profiles were offset from
  the source-to-target terrain chord but had no gravity-aligned clearance floor;
  the gentle `r00` waypoint sat only `80m` above terrain, exactly equal to its
  maximum cross-track allowance.
- Added planner-side minimum terrain-clearance ratios of `20% | 25% | 30%` for
  the gentle, medium, and sharp profiles. At nominal radius, the `r00` waypoint
  centers now clear terrain by `160m | 200m | 240m`; `r+30` uses the same
  gravity-aligned floor over its local rising terrain.
- Resolved parameters now expose waypoint terrain height, actual terrain
  clearance, and the planner floor. Pack-resolution tests require both the
  profile floor and more vertical clearance than the cross-track allowance plus
  vehicle offset.
- Removed the experimental final-descent speed cap after the corrected corpus
  landed `81 / 81` without it. The speed cap modestly shortened mean sim time
  but was no longer needed for correctness and would have preserved controller
  complexity introduced to compensate for bad route planning.
- Made touchdown settle persistent from the low-clearance rescue region through
  contact and require safe angular rate before idle cutoff. This closes the
  independently reproducible unsafe-angular-rate touchdown case without
  changing the translational touchdown envelope.
- Fresh `8`-worker, `--no-reuse` validation:
  - `transfer_waypoint_turn_contract_smoke`: `81 / 81`, `2.29s` wall clock,
    `21.43s` mean sim time, and `32.13s` max sim time
  - `transfer_waypoint_turn_smoke`: `81 / 81`, `11.66s` wall clock, `62.72s`
    mean sim time, and `90.72s` max sim time
  - every route lands `27 / 27`; the worst reported uphill corridor margin
    improves from about `-20m` to `+41m`
  - `transfer_route_angle_radius_suite`: `297 / 297`, `44.51s` wall clock,
    `63.78s` mean sim time, and `86.04s` max sim time
  - `cargo test --workspace`: all tests pass
- Rejected a blanket extra `10%` clearance floor (`160m | 240m | 320m`) because
  the sharp `r00` waypoint became unreachable and regressed the handoff pack to
  `73 / 81`. The narrower explicit tiers preserve all contracts and three
  distinct route shapes.
- The balanced single-waypoint workbench no longer has known landing or handoff
  failures. Next work can move to multiple preplanned waypoints and wider
  route/radius coverage without adding terrain reaction to guidance.

## 2026-07-09

### Fixed-endpoint state-target waypoint guidance checkpoint

- Extracted shared state-target acceleration and thrust/tilt allocation
  primitives so terminal and waypoint guidance use the same control math.
- Split waypoint geometry into a fixed leg endpoint for acceptance/state
  targeting and a moving active-leg lookahead used only for bounded path
  correction. The correction fades near handoff and is capped at 15% of current
  thrust authority.
- Active waypoint guidance now chooses an outbound-envelope target velocity and
  a geometry-derived time to go, then commands the acceleration needed to reach
  the fixed endpoint in that state. Candidate horizons depend on remaining leg
  distance and speed, not `sim.max_time_s`.
- Plans are stable per leg and are replaced only after expiry or dynamic
  infeasibility. The arrival instant, rather than a pre-expiry grace threshold,
  controls expiry so the controller does not replan every update near handoff.
- Rejected the first minimum-effort candidate ordering: favoring low target
  speed and long horizons improved final landing to `78 / 81` but produced only
  `18 / 81` contract passes, dominated by long plane-cross spatial misses.
  Preferring the shortest authority-feasible candidate fixed the actual
  pass-through objective without route/profile branches.
- Fresh `8`-worker, `--no-reuse` balanced validation after cleanup:
  - `transfer_waypoint_turn_contract_smoke`: `81 / 81` passes, `0` failures,
    `1.98s` wall clock, `20.10s` mean sim time, and `32.13s` max sim time
  - `transfer_waypoint_turn_smoke`: `75 / 81` landings and `6` crashes,
    `12.38s` wall clock, `63.56s` mean sim time, and `100.22s` max sim time
  - route landing totals are `r-30 24 / 27`, `r00 27 / 27`, and
    `r+30 24 / 27`
  - every failed landing first passes the waypoint contract; the remaining
    failures are final direct-transfer/terminal recovery debt
  - mean controller-update compute on the contract pack is `16.6us`, down from
    `219.1us` at the `12 / 81` baseline and well below the `1ms` budget
- Fresh direct-transfer regression remains `297 / 297`, with `50.43s` wall
  clock, `63.79s` mean sim time, and `86.04s` max sim time.
- Legacy stress diagnostics are intentionally not acceptance gates:
  - smooth bend: `27 / 27` handoff contracts and `16 / 27` final landings
  - dogleg hairpin: `0 / 27` handoff contracts and `21 / 27` final landings
- Removed the superseded waypoint boost-score tie-breaker, outbound moving-target
  blend, active-waypoint coast preview, and their telemetry/config/test surface.
  Active guidance is now one route-frame state-target mechanism rather than a
  stack of optional local heuristics.
- Next: diagnose the six post-handoff crashes and improve target energy or
  final-leg recovery while preserving `81 / 81` handoffs and `297 / 297` direct
  transfer. Multi-waypoint sequencing follows after that recovery boundary is
  stable.

### Balanced waypoint-turn corpus repair and direct-transfer closure

- Added a route-local uphill-corridor brake that opposes targetward lateral
  speed once the steep source-clearance corridor is tilt-limited. The rule uses
  corridor geometry and velocity, not route labels or waypoint-profile IDs.
- Fresh `8`-worker, `--no-reuse` direct-transfer validation now shows:
  - `transfer_route_angle_radius_suite`: `297 / 297` landed, `0` invalidations,
    `44.12s` wall clock, `63.79s` mean sim time, and `86.04s` max sim time
  - `transfer_route_angle_radius_frontier_full`: `108 / 108` landed across all
    payloads, radii, and full seeds, with `14.11s` wall clock, `64.31s` mean sim
    time, and `79.07s` max sim time
  - the legacy `frontier` pack name remains a useful steep-uphill regression
    label, but `r+80` is no longer a current direct-transfer failure frontier
- Centralized waypoint spatial and outbound-envelope assessment in `pd-core` so
  controller capture, handoff goals, and report classification use one
  contract. Handoff goals are evaluated on controller observation boundaries,
  eliminating physics-step/controller-step discrepancies in paired packs.
- Bumped the run schema to `3` and batch schema to `25` so caches produced
  before the handoff-contract change cannot be reused as equivalent evidence.
- Added an explicit `waypoint_handoff_envelope` selector. The new
  `pass_through_v1` envelope requires at most `0.35rad` outbound heading error,
  at least `8m/s` outbound progress, at most `20m/s` outbound cross speed, and
  total speed from `10m/s` through `130m/s`; it deliberately leaves vertical
  speed unbounded for this first route-relative corpus.
- The maintained balanced corpus now has three one-waypoint profiles at 55%
  route progress with common spatial tolerances:
  - `single_gentle_bend_v1`: `10%` offset, `22.8deg` turn
  - `single_medium_bend_v1`: `20%` offset, `43.9deg` turn
  - `single_sharp_bend_v1`: `30%` offset, `62.3deg` turn
- Removed `single_straight_v1`: placing its waypoint directly on the monotonic
  source-to-target terrain made the capture volume intersect terrain. Resolved
  pack tests now require every maintained waypoint capture volume plus vehicle
  touchdown offset to clear the actual scenario terrain.
- Added paired `transfer_waypoint_turn_smoke` final-landing and
  `transfer_waypoint_turn_contract_smoke` handoff packs. Each has `81` unique
  runs over three profiles, `r-30 | r00 | r+30`, `empty | half | full`, nominal
  radius, and three smoke seeds. Reports group this dense matrix by waypoint
  profile before route and expose the resolved envelope and outbound cross
  speed.
- Fresh `8`-worker, `--no-reuse` paired baseline at `1c88f87`:
  - final landing: `50 / 81` successes and `31` crashes; `r+30` is `27 / 27`,
    `r00` is `23 / 27`, and `r-30` is `0 / 27`
  - profile landing totals are gentle `14 / 27`, medium `18 / 27`, and sharp
    `18 / 27`
  - handoff contract: `12 / 81` passes, `46` spatial misses, `21` outbound
    envelope failures, and `2` incomplete/crash-before-handoff cases
  - all `12` handoff passes are level-route cells; profile totals are gentle
    `3`, medium `3`, and sharp `6`
  - paired landing and contract records now agree on handoff status for all
    `81` selector cells
- Rejected a staged coast/corridor controller experiment after the repaired
  corpus stayed at `50 / 81` landings with no downhill gain while contract
  passes regressed from `12 / 81` to `3 / 81`. Requiring a full predicted
  handoff before coast and suppressing the direct terrain corridor did not add a
  corrective waypoint objective; it prolonged boost against the same moving
  virtual target and shifted failures to `42` spatial misses, `33` outbound
  failures, and `3` incomplete timeouts.
- Legacy regression results remain stable:
  - dogleg smoke landing `27 / 27`, dogleg contract `0 / 27`
  - smooth-bend smoke landing `27 / 27`, smooth-bend contract `15 / 27`
  - smooth-bend full landing `108 / 108`, smooth-bend full contract `57 / 108`
- Interpretation: direct transfer no longer blocks Phase 3. The balanced corpus
  shows that waypoint guidance itself is not general yet: downhill waypoint
  routes fail completely and many recoverable landings violate the handoff
  contract. The next controller slice should separate fixed leg-end acceptance
  geometry from the moving steering/lookahead target before revisiting coast or
  corridor policy; it should not add per-profile or per-route-angle branches.

### Initial waypoint smooth-profile workbench checkpoint

- Added `single_bend_v1` as the first smoother pass-through waypoint profile for
  the `r+80` waypoint corpus. The profile places one waypoint at 55% of the
  route with a 20% route-radius source-side offset, making the nominal turn about
  44 degrees instead of the `single_dogleg_v1` hairpin.
- Kept the bend distance capture tight while widening the plane-crossing
  cross-track envelope, matching the intended pass-through contract without
  making the waypoint a precision stop.
- Added resolved waypoint geometry diagnostics for inbound leg length, outbound
  leg length, turn angle, profile progress, and lateral offset.
- Added `transfer_waypoint_bend_*` final-landing and contract packs for smoke
  and full-seed runs. The existing `single_dogleg_v1` packs remain as stress
  probes; the bend packs are the default waypoint-guidance workbench for the
  next controller slice.
- Updated waypoint triage reports to show the waypoint profile and resolved turn
  angle so smooth-profile results are not mixed up with dogleg stress results.
- Initial smoke validation:
  - `transfer_waypoint_bend_rpos80_smoke`: `25 / 27` final-landing successes,
    with the remaining two crashes both in short-radius payload cases after
    waypoint capture.
  - `transfer_waypoint_bend_contract_rpos80_smoke`: `15 / 27` contract
    successes, up from the initial `12 / 27` with the narrower bend
    cross-track envelope.
  - The remaining failures are controller handoff/recovery debt, not a reason
    to keep reshaping the workbench fixture.

### Waypoint turn-feasibility checkpoint

- Added controller telemetry for waypoint approach feasibility:
  `remaining_to_plane_m`, `time_to_plane_s`, `required_turn_distance_m`,
  `shaping_start_distance_m`, and `turn_margin_m`.
- Verified the telemetry path with `cargo test -p pd-control waypoint` and
  `cargo test -p pd-eval waypoint`.
- Rejected two waypoint-controller tuning experiments after focused smoke runs:
  - turn-feasibility target blending plus bounded waypoint boost scoring still
    left `transfer_waypoint_contract_rpos80_smoke` at `0 / 27`; the best safe
    variant preserved `transfer_waypoint_rpos80_smoke` at `27 / 27` but shifted
    the contract failures to spatial misses rather than producing viable
    handoffs
  - adding low/idle boost candidates when waypoint turn margin went negative
    was worse: both `transfer_waypoint_contract_rpos80_smoke` and
    `transfer_waypoint_rpos80_smoke` regressed to `0 / 27` crashes
- Result interpretation:
  - the new telemetry confirms the core problem: the `single_dogleg_v1`
    waypoint is usually reached with negative turn margin, so local target
    blending is too weak and late for the current pass-through envelope
  - the next waypoint slice should not keep stacking local waypoint-target
    heuristics; it should revisit route/profile shape or introduce an explicit
    corridor/reference objective that makes the pass-through contract feasible
    before first waypoint-plane crossing

## 2026-07-08

### Waypoint contract probe checkpoint

- Added `evaluation_goal = waypoint_handoff` for transfer-matrix entries with
  preplanned waypoints. The probe stops at the first selected waypoint handoff
  and scores spatial capture plus outbound viability directly.
- Added focused waypoint contract packs:
  - `transfer_waypoint_contract_rpos80_smoke`: `27` smoke-seed runs across
    `empty | half | full` and `short | nominal | long`
  - `transfer_waypoint_contract_rpos80_full`: `108` full-seed runs across the
    same payload and radius tiers
- Verified locally with `8` workers and `--no-reuse`:
  - `transfer_waypoint_contract_rpos80_smoke`: `0 / 27` contract successes,
    `0` invalidations, `22.37s` mean sim time, `30.72s` max sim time,
    `15` spatial misses, and `12` outbound-unviable captures
  - `transfer_waypoint_contract_rpos80_full`: `0 / 108` contract successes,
    `0` invalidations, `22.31s` mean sim time, `31.19s` max sim time,
    `56` spatial misses, and `52` outbound-unviable captures
- Result interpretation:
  - the current waypoint controller can still recover and land after the dogleg
    in the final-landing packs
  - the waypoint contract probe confirms the route-quality problem directly:
    no current handoff reaches the next-leg envelope yet
  - next controller work should tune pass-through guidance against the contract
    packs, using final-landing packs as regression gates

## 2026-07-07

### Waypoint guidance implementation checkpoint

- Added first-class transfer route waypoints to the mission contract and wired
  `signed_route_arc_transfer_v1` matrix entries to accept a
  `single_dogleg_v1` waypoint profile.
- Added `transfer_waypoint_pdg_v1` as a waypoint-enabled variant of the staged
  transfer controller. The controller tracks the active leg, prevents terminal
  handoff before waypoint capture, and then resumes the final target leg.
- Added waypoint capture telemetry and a collapsed `Waypoint Handoff Triage`
  report section so route-level reports can show capture status, closest
  distance, cross-track miss, outbound progress, and worst seeds.
- Added focused `r+80` waypoint packs:
  - `transfer_waypoint_rpos80_smoke`: `27` smoke-seed runs across
    `empty | half | full` and `short | nominal | long`
  - `transfer_waypoint_rpos80_full`: `108` full-seed runs across the same
    payload and radius tiers
- Verified locally with `8` workers and `--no-reuse` after relaxing v1 capture
  to a spatial handoff surface, giving waypoint-profile routes a `130s` sim
  cap, and reporting waypoint misses as route-contract warnings:
  - `transfer_waypoint_rpos80_smoke`: `27 / 27` successes, `0` timeouts,
    `0` invalidations, `94.56s` mean sim time, `120.59s` max sim time,
    `15` captured waypoint runs, and `12` contract warnings
  - `transfer_waypoint_rpos80_full`: `108 / 108` successes, `0` timeouts,
    `0` invalidations, `94.56s` mean sim time, `120.59s` max sim time,
    `60` captured waypoint runs, and `48` contract warnings
- Result interpretation:
  - the direct `r+80` frontier was previously `0 / 108`; a single preplanned
    dogleg now lands every waypoint `r+80` payload/radius/seed case
  - the previous `full/long/r+80` timeout cluster now lands at `120.59s`
  - the remaining debt is waypoint-contract quality, not final landing:
    waypoint misses are explicit report warnings, while outbound heading,
    progress, speed, and vertical rate remain diagnostics for the next
    controller pass

## 2026-07-05

### Resume and transfer-radius alignment checkpoint

- Resumed from commit `14ef55e` with the tracked workspace clean.
- Local ignored report outputs currently point the latest eval/report symlinks at
  `transfer_route_angle_radius_suite`, so the active Phase 3 evidence is the
  route-angle/radius matrix rather than the older nominal-radius-only pack.
- Current terminal clean artifacts use schema 21 and show:
  - `terminal_bot_lab_suite`: `171 / 180` scored successes, `9` scored
    failures, `9` impossible warnings, `12` frontier annotations
  - `terminal_bot_lab_full`: `684 / 720` scored successes, `36` scored
    failures, `36` impossible warnings, `48` frontier annotations
  - clean `empty` and `half` remain solved; clean `full` is `180 / 216`
    scored, with the remaining `36` scored failures plus `36` impossible
    warnings and `48` frontier annotations
- At resume, transfer radius-tier artifacts showed:
  - `transfer_radius_tier_suite`: `135 / 135` successes, `0` invalidations
  - `transfer_route_angle_radius_suite`: `264 / 297` successes, `33` crashes,
    `0` invalidations
- The wide transfer failures split into:
  - `27` known `r+80` `near_vertical_transfer_route` frontier crashes across
    payload and radius tiers
  - `6` non-frontier scored crashes: `full/r-80` at `short` and `long` radius,
    all three smoke seeds for each tier
- This set up the follow-up for the slice: refresh the transfer packs from the
  clean checkout, then triage whether `full/r-80` short/long was controller
  debt, corpus policy debt, or a route-shaping/waypoint-layer signal.

### Transfer clean-cache refresh and `full/r-80` triage

- Refreshed the transfer packs locally from clean commit `ed50359` with `8`
  workers:
  - `transfer_bot_lab_suite`: `45 / 45` successes, `0` invalidations,
    `4.76s` wall clock, `44.77s` mean sim time
  - `transfer_route_angle_suite`: `90 / 99` successes, `9` crashes, `0`
    invalidations, `9.64s` wall clock, `43.01s` mean sim time
  - `transfer_radius_tier_suite`: `135 / 135` successes, `0` invalidations,
    `13.32s` wall clock, `44.64s` mean sim time
  - `transfer_route_angle_radius_suite`: `264 / 297` successes, `33` crashes,
    `0` invalidations, `27.25s` wall clock, `41.78s` mean sim time
- Cache provenance for those refreshed reports is clean/fresh under workspace
  key `ed503592f632`; explicit `promote-cache` was not needed because there was
  no dirty cache to promote.
- `r+80` remains the known near-vertical frontier:
  - `27` crashes across `empty`, `half`, `full` and all radius tiers
  - the failures generally occur before terminal handoff or without terminal
    entry diagnostics, so this remains route/waypoint debt rather than terminal
    touchdown debt
- `full/r-80/short` is a source-clearance failure:
  - direct terminal capture starts from the elevated source pad
  - the craft crashes at about `10.43s`, still roughly `47.6m` from target
  - vertical speed is only about `-6.9 m/s`, but hull clearance goes negative
    against source-pad/plateau terrain
- `full/r-80/long` is a terminal braking failure:
  - direct terminal capture reaches the target laterally
  - the craft crashes at about `33.37s`, roughly `0.24m` from target center
  - vertical speed is about `-14.1 m/s`, so the miss is excessive touchdown
    descent rate after a high direct descent
- The next viable fix should stay generalized:
  - add a route-local source-clearance/launch phase before direct terminal
    capture when the source pad/terrain plateau is still a clearance risk
  - tighten direct-terminal braking for high-altitude, full-payload downhill
    starts
  - avoid `r-80` or radius-specific branches

### Transfer `full/r-80` handoff fix checkpoint

- Added focused pack `transfer_rneg80_radius_focus_suite` to isolate the
  heavy-payload `r-80` short/nominal/long radius tiers across smoke seeds.
- Focused pack results:
  - baseline before the controller fix: `3 / 9` successes
  - after route-local source-clearance hold: `6 / 9` successes; all short-radius
    source-pad/plateau crashes were resolved
  - after transfer-scoped terminal gate horizon tuning: `9 / 9` successes
- Controller changes stay generalized:
  - direct-terminal transfer routes now hold takeoff while sampled source-side
    terrain ahead still lacks configured hull clearance
  - `transfer_pdg_v1` extends its embedded terminal gate burn horizon, but
    standalone `terminal_pdg_v1` defaults are unchanged
- Refreshed broad transfer packs from clean controller commit `673954f` with
  `8` workers and `--no-reuse`:
  - `transfer_bot_lab_suite`: `45 / 45` successes, `0` invalidations, `60.44s`
    mean sim time, `76.60s` max sim time
  - `transfer_route_angle_suite`: `90 / 99` successes, `9` crashes, `0`
    invalidations, `56.24s` mean sim time, `76.60s` max sim time
  - `transfer_radius_tier_suite`: `135 / 135` successes, `0` invalidations,
    `59.58s` mean sim time, `79.35s` max sim time
  - `transfer_route_angle_radius_suite`: `270 / 297` successes, `27` crashes,
    `0` invalidations, `55.39s` mean sim time, `83.24s` max sim time
- The wide matrix now has no non-frontier failures. All remaining transfer
  crashes are the known `r+80` near-vertical route frontier across payload and
  radius tiers.

### Transfer full-seed coverage checkpoint

- Added two transfer full-seed packs:
  - `transfer_route_angle_radius_full_solved` covers all non-`r+80` route
    angles, all radius tiers, all payload tiers, and all 12 transfer seeds
  - `transfer_route_angle_radius_frontier_full` isolates `r+80` across all
    radius tiers, all payload tiers, and all 12 transfer seeds
- Verified locally with `8` workers and `--no-reuse`:
  - `transfer_route_angle_radius_full_solved`: `1080 / 1080` successes, `0`
    invalidations, `59.24s` mean sim time, `83.24s` max sim time
  - `transfer_route_angle_radius_frontier_full`: `0 / 108` successes, `108`
    crashes, `0` invalidations, `16.83s` mean sim time, `21.70s` max sim time
- Interpretation:
  - the solved direct-transfer region has no full-seed outliers
  - the known `r+80` near-vertical route failure is still total and should stay
    classified as route/waypoint debt rather than terminal guidance debt
  - the next transfer slice should move to waypoint-style route shaping instead
    of more direct-transfer controller tuning

### Waypoint guidance design checkpoint

- Split waypoint work into two separate problems:
  - waypoint setup/planning chooses terrain-valid waypoint positions and
    arrival envelopes
  - waypoint guidance follows the currently active leg, crosses the waypoint
    envelope, then switches to the next leg or final landing target
- Current priority is waypoint guidance. For the next implementation slice,
  assume the waypoint list is already planned and do not solve terrain-aware
  waypoint generation yet.
- A waypoint is a pass-through handoff contract, not a full-stop terminal
  objective. Arrival should require waypoint-plane progress, bounded
  cross-track miss, and an outbound state that keeps the next leg feasible.
- Waypoint guidance should stay terrain-blind in v1. Terrain avoidance belongs
  in waypoint planning through the chosen waypoint positions and spatial/energy
  envelopes; terrain crashes should diagnose bad plans, not trigger
  scenario-specific controller modes.
- Final `landing_on_pad` remains the primary scored goal. Waypoint capture
  quality should start as report telemetry: active waypoint index, active leg
  index, closest waypoint distance, cross-track miss, capture time, outbound
  heading error, outbound speed, vertical rate, and final-leg handoff quality.

## 2026-05-31

### Transfer projected-overshoot and pre-target capture checkpoint

- Tuned `transfer_pdg_v1` without route-label-specific branches:
  - boost direction now switches from route-anchor direction to projected miss
    direction once a target-y solution is reachable and outside the projected
    `dx` band
  - boost candidate scoring now centers projected `dx` symmetrically instead
    of only penalizing route-direction shortfall
  - uphill coast can hand off just before target-height crossing, but only
    when the next crossing is imminent, the projected miss is centered, terrain
    clearance is safe, and the latest-safe margin is already close
- Regenerated transfer packs locally with `8` workers:
  - `transfer_bot_lab_suite`: `45 / 45` successes, `0` invalidations,
    `44.77s` mean sim time, `52.49s` max sim time
  - `transfer_route_angle_suite`: `90 / 99` successes, `9` crashes, `0`
    invalidations, `43.01s` mean sim time, `63.38s` max sim time
- The remaining scored failures are unchanged: all `9` are `r+80`
  `near_vertical_transfer_route` frontier cases.
- Representative handoff-shape movement:
  - `full/r+30/seed0` projected handoff `dx` improved from about `-136m` to
    `-110m`
  - `full/r+45/seed0` projected handoff `dx` improved from about `-171m` to
    `-86m`
- Follow-up controller work should focus on reducing the remaining ugly
  successful route shapes and the long `full/r+45` landing time, not on
  reclassifying `r+80`.

### Transfer handoff triage report checkpoint

- Added a report-only `Transfer Handoff Triage` section before transfer shape
  triage.
- The section groups current-lane transfer runs by condition, vehicle, route,
  and radius, then sorts cells by failed/frontier status, low handoff height,
  high handoff speed, wide handoff projected `dx`, and wide boost-cutoff
  projected `dx`.
- The table exposes terminal entry kind, handoff gate, handoff height/speed,
  handoff projected `dx`, handoff angle, boost-cutoff quality/projected `dx`,
  shape RMSE, and the worst seed link without changing controller behavior or
  batch schema.
- `Transfer Shape Triage` remains available as a secondary visual-shape read,
  but controller tuning should start from handoff/gate/cutoff diagnostics.
- Current transfer interpretation is unchanged: `transfer_bot_lab_suite` is
  solved, `transfer_route_angle_suite` is solved through `r+60`, and `r+80`
  stays as the scored `near_vertical_transfer_route` frontier.

## 2026-05-30

### Transfer r+80 frontier policy checkpoint

- Classified uphill `r+80` transfer routes as the scored
  `near_vertical_transfer_route` frontier.
- The route stays in `transfer_route_angle_suite`, and frontier runs remain
  scored rather than invalidated so regressions stay visible.
- This locks the current interpretation: `r+80` is near-cliff
  waypoint/corridor debt above the staged transfer controller, not terminal
  guidance debt.

### Transfer shape instrumentation checkpoint

- Added Pylander-inspired transfer shape diagnostics:
  - shape-window curve RMSE against a parabolic boost-window reference
  - apex error
  - projected `dx` abs mean/max
  - projected shortfall ratio
  - boost burn duration, fuel used, and average throttle
- Single-run transfer previews now draw the boost-window reference curve when
  controller telemetry is available; dense lane previews stay actual-trajectory
  focused.
- The transfer controller now freezes the first boost-window route anchor for
  boost-quality and apex-target calculations, avoiding a shrinking intended
  shape as the vehicle approaches the target.
- Regenerated transfer packs locally with `8` workers:
  - `transfer_bot_lab_suite`: `45 / 45` successes, `0` invalidations,
    `47.34s` mean sim time, `61.35s` max sim time
  - `transfer_route_angle_suite`: `90 / 99` successes, `9` crashes, `0`
    invalidations, `43.08s` mean sim time, `61.35s` max sim time
- Current transfer read remains unchanged:
  - all route-angle cells from `r-80` through `r+60` solve across `empty`,
    `half`, and `full`
  - only `r+80` remains failed across payload tiers
  - the new diagnostics make the remaining "landed but ugly" transfer shape
    visible without changing the scored goal

### Transfer boost/handoff tuning checkpoint

- Added ballistic transfer diagnostics and surfaced boost/handoff fields in
  batch review records:
  - projected target-y crossing time and `dx`
  - impact angle
  - ballistic apex over target
  - boost quality and boost cutoff quality
- Tuned `transfer_pdg_v1` without adding route-label-specific branches:
  - boost now gates on ballistic quality instead of along-speed alone
  - boost steering uses projected miss direction when the target-y crossing is
    reachable
  - steep uphill boosts stay more vertical while touchdown clearance is low
  - coast prealignment avoids max retrograde tilt while the craft is still
    climbing
- Regenerated transfer packs locally with `8` workers:
  - `transfer_bot_lab_suite`: `45 / 45` successes, `0` invalidations
  - `transfer_route_angle_suite`: `90 / 99` successes, `9` crashes, `0`
    invalidations
  - `43.08s` mean sim time and `61.35s` max sim time for the 99-run
    route-angle pack
- Current transfer read:
  - all route-angle cells from `r-80` through `r+60` solve across `empty`,
    `half`, and `full`
  - only `r+80` remains failed across payload tiers
  - `r+80` is now best treated as near-cliff launch/waypoint debt, not as
    terminal handoff debt

### Transfer route-angle corpus baseline

- Added `fixtures/packs/transfer_route_angle_suite.json` as the first
  full-route-angle diagnostic transfer pack.
- The pack keeps the v1 transfer scope intentionally narrow:
  - `signed_route_arc_transfer_v1`
  - fixed `800m` nominal route radius
  - current lane only
  - payload tiers `empty`, `half`, `full`
  - smoke seeds `0`, `1`, `2`
  - all route angles from `r-80` through `r+80`
- Kept `transfer_bot_lab_suite` as the fast 45-run smoke gate.
- Added transfer handoff review metrics to batch records and reports:
  - final transfer phase
  - first terminal handoff time
  - handoff target `dx`
  - handoff height
  - handoff speed
- Regenerated `transfer_route_angle_suite` locally with `8` workers:
  - `57 / 99` successes
  - `42` crashes
  - `0` invalidations
  - `4.05s` wall clock
  - `33.22s` mean sim time
  - `72.33s` max sim time
- Current transfer read:
  - downhill and flat routes from `r-80` through `r00` solve across every
    payload tier
  - `empty/r+15` also solves
  - the active gap is uphill transfer control: `half/full r+15` and all
    `r+30`, `r+45`, `r+60`, and `r+80` cells expose boost or terminal-handoff
    failures
  - `75 / 99` runs reached terminal handoff and `24 / 99` ended in boost
- Next transfer work should tune launch/boost/coast/handoff behavior on the
  route-angle pack before adding route radius tiers or full-seed transfer
  coverage.

## 2026-05-06

### Terrain avoidance design pivot

- Pulled terrain avoidance out of the maintained terminal guidance scorecard.
- The terminal controller contract is now narrower: given a reachable,
  terrain-valid approach corridor or target, handle braking, lateral cleanup,
  descent-rate control, attitude, and touchdown.
- General terrain navigation is parked until it can live in a higher-level
  layer:
  - approach-corridor validity checks for target/route selection
  - collision-course warnings for player co-pilot use
  - waypoint/path planning for pure non-human bots
- Renamed the retained backstop packs as experimental diagnostics:
  - `fixtures/packs/experimental_terrain_backstop_suite.json`
  - `fixtures/packs/experimental_terrain_backstop_full.json`
- Reclassified those entries under the `diagnostic` expectation tier and
  `terrain_diagnostic` tag so they are visibly outside the core bot-lab
  scorecard.
- The latest backstop results remain useful as a snapshot, but they are no
  longer blockers for terminal controller tuning or Phase 2 progress.

## 2026-05-02

### Former reactive terrain backstop-only checkpoint

- Removed `terrain_clip` from the then-maintained reactive terrain packs.
- The latest clip calibration made the terrain-blind controller fail
  `194 / 216` focused runs, but it did so by blocking enough of the path to
  force a larger trajectory change than the localized terminal-avoidance
  behavior the suite should test.
- The `terrain_clip` condition implementation remains parked for later
  redesign, but it is no longer part of the active terrain diagnostics.
- Added a diagnostic `terminal_pdg_no_terrain` / `tpdg_no_terrain` controller
  alias that leaves the terminal controller intact but disables candidate-path
  terrain clearance.
- Regenerated the backstop-only terrain reports:
  - `experimental_terrain_backstop_suite`: `57 / 72` scored successes,
    `15` scored crashes, `0` invalidations, `3.24s` wall clock with `8` workers
  - `experimental_terrain_backstop_full`: `228 / 288` scored successes,
    `60` scored crashes, `0` invalidations, `12.73s` wall clock with `8` workers
- The current terrain read is backstop containment only. The next clip attempt
  should be redesigned before it re-enters any experimental terrain pack.

## 2026-05-01

### Generic terminal terrain-clearance guidance slice

- Added a first controller-side candidate-path terrain-clearance evaluator to
  `terminal_pdg_v1`.
- The evaluator samples the planned hull trajectory for each terminal
  gate candidate and feeds minimum clearance / first violation timing back into
  candidate ordering.
- The controller still does not branch on condition names such as `backstop` or
  `clip`. Clearance is derived from terrain geometry.
- Low-relief terrain at the target surface is ignored for the clearance
  constraint, so normal pad contact and flat-ground touchdown do not look like
  obstacle violations.
- Added guidance telemetry:
  - `guidance.terrain_min_clearance_m`
  - `guidance.terrain_first_violation_time_s`
  - `guidance.terrain_clearance_safe`
- Verified the smoke terrain pack locally with `8` workers:
  - `experimental_terrain_backstop_suite`: `57 / 72` scored successes
  - `15` scored crashes
  - `0` invalidations
  - `3.24s` wall clock
- Verified the full terrain pack locally with `8` workers:
  - `experimental_terrain_backstop_full`: `228 / 288` scored successes
  - `60` scored crashes
  - `0` invalidations
  - `12.73s` wall clock
- The first-pass terrain read is:
  - backstop cases are mostly solved for `empty`, with `half` still exposing
    authority / path-clearance pressure
  - the mostly-solved low clip guard lane was removed from the maintained pack
  - the later clip retune was also removed from the maintained pack after it
    proved too path-blocking for a localized-avoidance test

### Former reactive terrain terminal-corpus slice

- Added the first terminal reactive terrain condition sets:
  - `terrain_backstop_wall`
  - `terrain_backstop_slanted`
- Backstop variants are shape variants, not height/severity bands: both use a
  `400m` rise, with `wall` testing a steep target-side face and `slanted`
  testing a longer ramp face.
- The condition sets mutate scenario terrain geometry only. They do not introduce
  controller-visible mode switches, and fixture metadata such as
  `hazard_driver=containment_backstop` remains report context.
- Added current-lane-only terrain packs:
  - `fixtures/packs/experimental_terrain_backstop_suite.json`
  - `fixtures/packs/experimental_terrain_backstop_full.json`
- Added terminal-matrix `arc_points` entry selectors so terrain packs can drop
  terrain-blind high-arc cells without cloning matrix definitions.
- The experimental terrain matrix now keeps backstop on `a70/a80`.
- `terrain_clip` is parked for redesign rather than kept as a scored pack entry.
- The first terrain packs intentionally cover `empty` and `half` payload tiers
  only. Full payload remains a separate authority-frontier concern.
- The first broader draft included both low and medium clip variants, but the
  low clip lane was removed after it proved to be mostly a guard lane rather
  than a meaningful terrain challenge.
- The initial pre-clearance controller signal was clean:
  - pruned backstop cases expose the containment gap without all-pass high-arc
    clutter
  - raised clip cases now expose descent-path terrain intersections
  - these gaps justified adding the generic candidate-path terrain-clearance
    constraint described above

### Active implementation focus

1. Keep terminal guidance focused on valid-approach landing behavior.
2. Treat terrain packs as non-blocking diagnostics until approach-corridor or
   waypoint-planning semantics exist.
3. If terrain work resumes, keep controller behavior geometry-derived and avoid
   condition-name branches.

## 2026-04-29

### Regression-policy checkpoint

- Added a structured batch comparison regression policy so compare results now
  carry an explicit `pass` / `warn` / `fail` status instead of relying only on
  manual report reading.
- When both compared reports contain a preferred `current` or `staged`
  controller lane, the policy is scoped to that lane so hidden reference lanes
  do not gate the main terminal workbench.
- The default gate fails on:
  - shared runs that move from success to failure
  - increased scored failure count when compare run sets match exactly
  - decreased scored success rate when compare run sets match exactly
- The default gate warns on:
  - material mean sim-time increase
  - increased invalidated-run count
  - compare coverage mismatch between current and baseline run sets
- Batch reports now show a Regression Policy panel and a policy chip in the
  overview diff row.
- `pd-eval run-pack --enforce-regression-policy` exits nonzero when a resolved
  compare baseline fails the required policy thresholds.

### Active implementation focus

1. Use the regression-policy gate for future controller and corpus changes.
2. Keep refining feasibility / annotation semantics only where the vehicle is
   authority limited, while keeping frontier failures scored.
3. Keep the next terminal-family expansion separate from transfer work:
   - the deferred terminal extension is a signed arrival family for climbing
     arrivals into the target
   - terrain and obstacle cases stay outside terminal pass/fail gates until a
     higher-level approach-corridor or waypoint layer exists
4. Shape the first transfer slice around a one-sided signed route arc:
   - descent, flat, and climb are route-angle cells in one family
   - radius starts fixed but remains a deferred axis because travel distance
     changes the trajectory shape
   - simple monotonic route terrain may be used for miss/crash containment, not
     as a terrain-avoidance objective.
   - first-class transfer matrix expansion, route metadata, monotonic route
     terrain, and the `transfer_bot_lab_suite` smoke pack are now in place
   - `transfer_pdg_v1` is staged and delegates final landing to
     `terminal_pdg_v1`
5. Keep controller tuning hypothesis-driven and smoke-suite gated rather than
   restarting broad parameter loops.

## 2026-04-28

### Current status

- The latest landing-time pass kept reporting diagnostics, not controller
  tuning: broad touchdown/settling shortcuts either did not move outcomes or
  traded small time savings for new scored crashes.
- Phase 2 remains the active phase, but the center of gravity has moved:
  - report/cache/review-tree infrastructure is no longer the bottleneck
  - the clean `empty` and `half` payload tiers have no scored current-lane
    failures
  - low-thrust/high-energy frontier cells are scored and annotated, not
    excluded from failure counts
  - the trajectory-error matrix is now the main stress corpus above the clean
    payload tiers
- The important distinction in reports is now:
  - `current` means the latest `terminal_pdg_v1` run
  - compare/baseline provenance may point at a cached prior current run or at
    the internal heuristic baseline controller
  - `baseline` as a report lane is not a physical scenario axis

### Current clean-matrix checkpoint

Report entrypoints:

- `outputs/eval/terminal_bot_lab_suite/summary.json`
- `outputs/eval/terminal_bot_lab_full/summary.json`

Latest recorded wall-clock signal with `8` workers:

- `terminal_bot_lab_suite`: `7.13s`
- `terminal_bot_lab_full`: `28.99s`

Smoke tier:

- `terminal_bot_lab_suite`
  - `current`: `168 / 180` scored successes, `12` scored failures,
    `9` impossible warnings, `12` frontier annotations
  - `baseline`: `33 / 180` scored successes, `147` scored failures,
    `9` impossible warnings, `12` frontier annotations

Full pack:

- `terminal_bot_lab_full`
  - `current`: `676 / 720` scored successes, `44` scored failures,
    `36` impossible warnings, `48` frontier annotations
  - `baseline`: `135 / 720` scored successes, `585` scored failures,
    `36` impossible warnings, `48` frontier annotations

`terminal_bot_lab_full` current-lane split by payload tier:

- `empty`: `252 / 252`
- `half`: `252 / 252`
- `full`: `172 / 216` scored, `44` scored failures,
  `36` impossible warnings, `48` frontier annotations

The clean matrix read is now:

- `empty` is solved on the maintained Earth corpus
- `half` is solved on the maintained clean Earth corpus
- `full` is the clean-matrix low-thrust/high-energy frontier tier; failed
  frontier cells remain scored failures

### Trajectory-error matrix checkpoint

Report entrypoints:

- `outputs/eval/terminal_traj_err_suite/summary.json`
- `outputs/eval/terminal_traj_err_full/summary.json`

The numeric checkpoint below is from fresh schema-14 local captures. Regenerate
the ignored `outputs/eval` entrypoints after schema/report changes before
treating the files in this checkout as authoritative.

Latest verified schema-14 wall-clock signal with `8` workers:

- `terminal_traj_err_suite`: `12.71s`
- `terminal_traj_err_full`: `52.03s`

Smoke tier:

- `terminal_traj_err_suite`
  - `current`: `689 / 720` scored successes, `31` scored failures,
    `36` impossible warnings, `48` frontier annotations

Full pack:

- `terminal_traj_err_full`
  - `current`: `2754 / 2880` scored successes, `126` scored failures,
    `144` impossible warnings, `192` frontier annotations

`terminal_traj_err_full` current-lane split by condition:

- `traj_undershoot_small`: `693 / 720` scored, `27` scored failures,
  `36` impossible warnings, `48` frontier annotations
- `traj_undershoot_large`: `707 / 720` scored, `13` scored failures,
  `36` impossible warnings, `48` frontier annotations
- `traj_overshoot_small`: `683 / 720` scored, `37` scored failures,
  `36` impossible warnings, `48` frontier annotations
- `traj_overshoot_large`: `671 / 720` scored, `49` scored failures,
  `36` impossible warnings, `48` frontier annotations

`terminal_traj_err_full` current-lane split by payload tier:

- `empty`: `1008 / 1008`
- `half`: `1006 / 1008`, `2` scored failures
- `full`: `740 / 864` scored, `124` scored failures,
  `144` impossible warnings, `192` frontier annotations

The trajectory-error read is now:

- `empty` is solved across the projected-miss corpus
- `half` is nearly solved, with only sparse high-energy overshoot-large outliers
  still standing out
- `full` is represented as a scored low-thrust/high-energy frontier stress tier
- the remaining scored failures are real stress cases, not report artifacts:
  - `traj_overshoot_large / half / a60 / high`: seeds `2` and `4`
  - low-thrust/high-energy `full / high` frontier failures across clean and
    trajectory-error conditions

### Latest tuning note

The latest tuning loops deliberately rejected several broad controller levers:

- Late-touchdown rescue sign/tilt changes regressed `terminal_traj_err_suite`
  from the checkpoint `685 / 720` scored successes to the `650-664 / 720`
  range, even when narrowed to off-pad projected touchdown.
- Extending lateral-hold descent slowing above the touchdown band did not solve
  `traj_overshoot_large / half / a45 / high / seed 6`; it moved the suite to
  `684 / 720` and introduced an additional frontier off-target result.
- Letting urgent latest-safe recovery pick tilt-infeasible short candidates was
  too broad: the suite dropped to `677 / 720` and created new empty/half
  overshoot regressions.
- Shortening terminal gate vertical-brake timing with a shared-control tilt
  assumption was too sensitive globally; both `0.5x` and `0.85x` tilt scales
  broke pinned `pd-control` landing fixtures before suite testing.
- Adding only a midpoint latest-safe candidate kept the suite at `685 / 720` but
  did not fix `traj_overshoot_large / half / a45 / high / seed 6`.
- Extending projected-off-pad lateral hold to `80 m` worsened that seed in a
  single-run probe: it still crashed off target, with higher normal speed and
  attitude error.
- Raising the latest-safe max horizon from `14 s` to `30 s` proved the seed is
  recoverable only when paired with a midpoint candidate, but the broad default
  regressed the smoke suite to `683 / 720`, created new half-payload scored
  crashes/timeouts, and raised mean sim time from about `33.6 s` to `46.8 s`.

Focused diagnosis that led to the accepted fix:

- `traj_overshoot_large / half / a45 / high / seed 6` is the lane's worst
  projected-overshoot smoke seed (`engine_off_impact_x = 105 m`). The passing
  `seed 0` and `seed 1` variants sit at `75 m` and `90 m`.
- At about `40 m` altitude the failed run is still roughly `45 m` off-pad,
  moving laterally at about `7 m/s`, and descending near `19 m/s`; by `24 m`
  it is still roughly `49 m` off-pad. Late touchdown rescue then correctly
  protects touchdown speed, but it has no remaining room to recenter.
- A simple ZEM/ZEV feasibility check from the initial state needs roughly a
  `20 s` or longer capture horizon before the combined acceleration fits the
  tilt limit. The current latest-safe horizon is capped at `14 s`, so the
  controller stays in short-horizon emergency braking until the problem is
  geometrically unrecoverable.

The useful finding is that the sparse half-payload overshoot failures are not
final-touchdown rescue problems. The accepted fix is a conditional
long-capture mode: when an urgent latest-safe candidate is over authority and
the current target offset and projected ballistic miss have opposite signs, the
controller adds `22 s` and `30 s` capture candidates without raising the global
`14 s` latest-safe cap.

The smoke suite improved from `685 / 720` to `686 / 720` scored successes. The
full trajectory-error pack improved from `2732 / 2880` to `2741 / 2880` scored
successes, with no new failures relative to the previous full cache. The patch
fixed six half-payload overshoot-large full-pack failures across `a30/a45`
plus three full-payload frontier failures. At that checkpoint, the remaining
half-payload outliers were:

- `traj_overshoot_large / half / a60 / high / seed 2`
- `traj_overshoot_large / half / a60 / high / seed 4`
- `traj_overshoot_large / half / a80 / high / seed 2`

After the later `90s` eval-window policy change, the current full-pack
checkpoint leaves only the two `a60 / high` seeds above.

Landing-time follow-up finding:

- Extending the eval window from `60s` to `90s` cut timeout noise meaningfully:
  `terminal_traj_err_full` moved from `58` timeouts to `12`, and scored
  failures dropped from `35` to `2`.
- The accepted timeout change is explicitly eval policy, not controller logic:
  terminal matrix packs use `terminal_matrix_max_time_s = 90.0`, while
  reachability/frontier analysis still records and uses the original scenario
  `60s` reachability window.
- A focused landing-time pass rejected three tempting controller shortcuts:
  - ballistic idle-cut near touchdown cut wall/sim time slightly but added
    `29` crashes, mostly from stable-contact penetration tolerance
  - shortening centered settled descent from `3.0s` to `2.75s` only improved
    mean sim time by about `0.02s` and added `4` scored crashes
  - increasing settled-descent recenter gain improved some offsets but worsened
    mean sim time and added a scored crash
- The durable diagnosis is that the worst low-clearance dwell is usually not
  centered final hover. It is low-altitude unsafe recovery: the vehicle is
  close to the ground while still outside the touchdown footprint or carrying
  too much lateral speed.
- The report schema now tracks that directly with successful-run
  `low_altitude_dwell_s` and `low_altitude_unsafe_recovery_s` summaries, and
  the HTML report surfaces low-altitude unsafe recovery in overview/review
  tracking cells.
- A follow-up controller pass rejected the first broad fixes:
  - a low-altitude vertical-cushion rescue branch was effectively neutral on
    the full pack: headline outcomes stayed at `2754 / 2880` scored
    successes, `126` scored failures, and `12` timeouts, with only a tiny
    `overshoot_large / half` low-unsafe change
  - using the settled-descent command's smaller rescue tilt for its braking
    speed cap cut smoke-suite mean sim time from `31.50s` to `29.32s`, but it
    added `9` scored failures and loosened a reference fixture's centering
  - shortening centered settled descent from `3.0s` to `2.9s` was also not
    worth keeping: smoke-suite mean sim time moved only from `31.496s` to
    `31.489s` while adding one scored failure
- Current interpretation: the remaining landing-time cost is not a simple
  final-hover constant. It mixes low-altitude unsafe recovery, high-altitude
  late-safe arrival at the pad, and authority-limited frontier cases. Keep the
  new metrics, but do not keep landing-time controller shortcuts until they
  improve suite outcomes or a clearly targeted metric without adding crashes.

Overall controller-tuning checkpoint:

- The current `terminal_pdg_v1` is good enough for the maintained clean and
  trajectory-error Phase 2 workbench:
  - clean `empty` and `half` are solved
  - trajectory-error `empty` is solved
  - trajectory-error `half` has only `2 / 1008` scored failures, both in the
    same high-energy overshoot-large cell
  - the remaining full-payload failures are mostly the scored
    low-thrust/high-energy frontier this corpus is meant to expose
- Another broad tuning loop is unlikely to be the best next use of time. The
  recent loops improved isolated seeds only when they introduced broader crash
  regressions, weakened touchdown margins, or produced negligible timing gains.
- More controller tuning is still reasonable only as a tightly framed
  hypothesis with:
  - one or two pinned failing runs
  - a general mechanism, not arc/seed/condition branching
  - no smoke-suite scored-regression tolerance
- The better next slice is to move on to Phase 2 closure work: thresholded
  regression policy, clearer frontier/feasibility semantics where needed, and
  the next physical condition space such as terrain or obstacle terminal cases.

### Active implementation focus

1. Treat the current terminal controller as the Phase 2 baseline unless a
   specific, general controller hypothesis is worth testing against pinned
   failures and a smoke-suite no-regression gate.
2. Keep refining feasibility / annotation semantics only where the vehicle is
   authority limited, while keeping frontier failures scored
3. Add the next terminal corpus now that the current clean and trajectory-error
   semantics are stable enough:
   - terrain / obstacle conditions
   - later transfer-style conditions
4. Use thresholded regression policy so future tuning does not depend on
   manually reading every frontier churn pattern.

## 2026-04-23

### Status at this checkpoint

- Phase 2 is now primarily a controller-and-corpus phase, not a scaffolding
  phase.
- The terminal bot-lab workflow is in place end to end:
  - executable Earth `half_arc_terminal_v1` matrix
  - payload tiers `empty / half / full`
  - `terminal_pdg_v1` as the serious current lane
  - split trajectory-error condition packs for undershoot/overshoot
  - cache reuse / promotion / current-lane history compare in `pd-eval`
  - impossible-run classification for analytically unrecoverable terminal
    cases
- The maintained Earth terminal baseline now uses:
  - `60s` timeout headroom
  - relaxed `nominal_ttg_by_arc_point`
  - `a80 = 8.00s`
- The internal heuristic baseline lane is still useful as a reference
  controller, but it is no longer the main progress metric.

### Controller checkpoint at this date

- `terminal_bot_lab_suite`
  - `current`: `161 / 180` scored successes, `19` scored failures,
    `9` impossible
  - `baseline`: `33 / 180` scored successes, `147` scored failures,
    `9` impossible
- `terminal_bot_lab_full`
  - `current`: `643 / 720` scored successes, `77` scored failures,
    `36` impossible
  - `baseline`: `135 / 720` scored successes, `585` scored failures,
    `36` impossible

Full-tier current-lane split by payload tier:

- `empty`: `252 / 252`
- `half`: `228 / 252`
- `full`: `163 / 216` scored, `36` impossible

This means the maintained Earth matrix is now doing the intended job:

- `empty` is effectively solved on the current clean corpus
- `half` is mostly solved, with the remaining gap concentrated in `a80`
- `full` still exposes real control-authority limits and frontier cells

### Active implementation focus at this date

1. Close the shallow-tail controller gap on the maintained Earth matrix:
   - `half a80 mid/high`
   - the remaining scored `full` high-band failures
2. Expand the terminal corpus above the clean payload tiers:
   - `traj_undershoot_small`
   - `traj_undershoot_large`
   - `traj_overshoot_small`
   - `traj_overshoot_large`
   - later terrain and obstacle conditions
3. Tighten evaluation semantics now that the workflow is real:
   - broader frontier / infeasible classification beyond the current vertical
     brake bound
   - thresholded regression policy
   - only then more specialized report affordances

### Trajectory-error matrix checkpoint

- Added current-lane-only trajectory-error packs:
  - `fixtures/packs/terminal_traj_err_suite.json`
  - `fixtures/packs/terminal_traj_err_full.json`
- The tree keeps the same selector depth as the clean matrix:
  - `condition_set -> arc_point -> velocity_band -> vehicle_variant -> lane -> seed`
- Condition sets are split by direction and severity instead of aggregating
  undershoot and overshoot:
  - `traj_undershoot_small`
  - `traj_undershoot_large`
  - `traj_overshoot_small`
  - `traj_overshoot_large`
- The condition perturbation is projected engine-off miss distance:
  - `small`: `30m`, `45m`, `60m`
  - `large`: `75m`, `90m`, `105m`
  - undershoot remains short on the approach side
  - overshoot crosses past the target to the far side

Smoke-tier result:

- `terminal_traj_err_suite`
  - `current`: `640 / 720` scored successes, `80` scored failures,
    `36` impossible

Full-tier result:

- `terminal_traj_err_full`
  - `current`: `2557 / 2880` scored successes, `323` scored failures,
    `144` impossible
- By condition:
  - `traj_undershoot_small`: `655 / 720` scored, `36` impossible
  - `traj_undershoot_large`: `684 / 720` scored, `36` impossible
  - `traj_overshoot_small`: `624 / 720` scored, `36` impossible
  - `traj_overshoot_large`: `594 / 720` scored, `36` impossible
- By vehicle tier:
  - `empty`: `1008 / 1008`
  - `half`: `877 / 1008`
  - `full`: `672 / 864` scored, `144` impossible

The immediate read is that empty-payload trajectory error is solved across the
new corpus, half-payload failures cluster in the shallow/high-energy frontier,
and full-payload trajectory error mostly amplifies the already-known
control-authority limits.

## 2026-04-22

### Current status

- The terminal suite is no longer only a maintained design target.
- `terminal_pdg_v1` now exists as the first serious native Rust terminal
  controller lane:
  - terminal-only PDG-shaped guidance
  - latest-safe and nominal gate evaluation
  - braking-envelope vertical schedule
  - touchdown-clearance-aware rescue logic
- `pd-eval` now expands a real terminal matrix entry type:
  - selector hierarchy:
    - `mission`
    - `arrival_family`
    - `condition_set`
    - `vehicle_variant`
  - matrix axes:
    - `arc_point`
    - `velocity_band`
  - controller lane separated from the physical case
- `half_arc_terminal_v1` is now executable as the Earth baseline family.
- The main controller workbench packs are now:
  - `fixtures/packs/terminal_bot_lab_suite.json`
    - smoke tier
  - `fixtures/packs/terminal_bot_lab_full.json`
    - full tier
- The maintained Earth terminal hardware baseline now matches the core
  `pylander` vehicle/engine envelope closely enough for direct reasoning:
  - `8m x 10m` hull
  - `7200kg` dry mass
  - `6300kg` max fuel
  - `240000N` max thrust
  - `25%` ignited minimum throttle
  - `90 deg/s` max rotation rate
- The main vehicle variants are now payload tiers, not fuel-margin tiers:
  - `empty`
    - empty payload
  - `half`
    - half payload
  - `full`
    - full payload
- `pd-lab` still intentionally simplifies one engine detail:
  - no `pylander` overdrive path yet
  - fuel burn scales linearly between minimum and maximum thrust
- The bot-lab `current` lane now points at `terminal_pdg_v1`, not the older
  staged heuristic.
- Batch reports now surface the terminal matrix directly in the review tree:
  - `mission -> arrival_family -> condition_set`
  - `arc_point -> velocity_band -> vehicle_variant -> lane -> seed`

### First matrix results

- The first real Earth matrix run already showed that the old `current` lane
  was not viable on the real matrix and directly forced the `terminal_pdg_v1`
  pass.
- After aligning the maintained Earth suite to the heavier `pylander`
  vehicle/engine baseline and payload tiers:
  - `terminal_bot_lab_suite`
    - `baseline`: `0 / 126`
    - `current`: `0 / 126`
  - `terminal_bot_lab_full`
    - `baseline`: `0 / 504`
    - `current`: `0 / 504`
- That regression is useful signal, not a framework failure:
  - the suite is now discriminating against controllers that were implicitly
    tuned for an easier vehicle model
  - controller work needs to restart from this more faithful baseline instead
    of extrapolating from the earlier lighter craft
- The next controller task is no longer only "fix the shallow tail":
  - first restore viability on the steeper core
  - then reopen the shallow tail once the heavier baseline is under control

### Active implementation focus

1. Restore controller viability on the pylander-aligned Earth terminal matrix:
   - nominal first
   - then heavy-cargo
   - only then re-focus on the shallow tail
2. Expand the now-real matrix corpus carefully:
   - undershoot / overshoot trajectory-error conditions
   - small / large projected-miss severities
   - later terrain and obstacle conditions
3. Only after controller signal is meaningful on the matrix:
   - add thresholded regression policy
   - add compare cache / promotion / invalidation semantics
   - consider matrix-specific batch-report affordances

## 2026-04-21

### Current status

- The repo is now past the first “does the scaffold work?” stage.
- A real bot-lab workflow exists end to end:
  - deterministic sim and replay
  - shared controller kit
  - multiple built-in controller lanes
  - seeded packs and native multithreaded eval
  - single-run and batch report generation
  - report-site navigation under `outputs/reports/`
- `terminal_bot_lab_suite` is now the main controller workbench.
- External pack-vs-pack compare still exists, but only as explicit fixtures:
  - `terminal_compare_baseline_fixture`
  - `terminal_compare_regression_fixture`
- Batch reports now explicitly state their provenance and comparison mode:
  - standalone
  - lane compare
  - external compare
- Reporting is now good enough to support real iteration; it is no longer the
  primary blocker.
- Terminal-suite design now has a dedicated maintained doc:
  - `docs/terminal_suite.md`
- The biggest remaining structural gap is no longer “make the terminal matrix
  real.”
- That work is now in place. The next gap is controller performance and corpus
  expansion on top of the real matrix.

### Active implementation focus

1. Improve controller behavior on the real terminal matrix:
   - clean nominal
   - heavy-cargo / stress
2. Expand the real terminal corpus with:
   - trajectory-error conditions
   - later terrain and obstacle conditions
3. Once the corpus and controller signal are stable, tighten eval semantics
   around:
   - representative runs
   - thresholds and regression policy
   - compare cache / promotion / invalidation
4. After terminal guidance is structurally solid, start the smallest true
   Phase 3 transfer slice.

### Current open gaps

- The terminal matrix is now real, and the current lane is finally
  competitive on it, but the shallow-tail cells still need more work.
- The current bot-lab corpus only covers:
  - `clean`
  - `nominal`
  - `heavy_cargo`
  and still needs trajectory-error and later terrain conditions.
- Compare provenance is explicit in the report, but compare cache / promote /
  invalidate semantics are not implemented yet.
- Reporting is now being driven by real matrix scenarios, but should continue
  to follow corpus/controller needs rather than generic layout tuning.
- Terminal/eval suite design should now be treated as a maintained design
  surface, not only as ad hoc planning notes attached to one implementation
  pass.

#### Checkpoint 23: `terminal_pdg_v1` replaces the staged current lane

- Added a new native Rust controller in `pd-control`:
  - `terminal_pdg_v1`
  - terminal-only PDG-shaped guidance
  - latest-safe and nominal gate evaluation
  - braking-envelope vertical schedule
  - touchdown-clearance-aware rescue and touchdown cut logic
- Added focused `pd-control` tests for the new controller:
  - flat-fixture success
  - guidance metrics and gate-marker emission
- Switched both bot-lab packs so `current` now means `terminal_pdg_v1`:
  - `terminal_bot_lab_suite`
  - `terminal_bot_lab_full`
- The new controller is not only “alive”; it materially outperforms the old
  baseline on the real Earth matrix:
  - smoke: `100 / 126` vs `12 / 126`
  - full: `403 / 504` vs `42 / 504`
- The shallow tail is now the real remaining problem:
  - `a70`
  - `a80`
  - and especially the `high` velocity band

#### Checkpoint 22: real terminal matrix execution and full bot-lab suite

- Added a first-class terminal-matrix entry type to `pd-eval`:
  - explicit selector hierarchy
  - explicit `arc_point x velocity_band` matrix expansion
  - deterministic seed policy with documented side resolution
  - lane-aware execution over the same resolved physical cases
- Implemented `half_arc_terminal_v1` directly in the evaluator as the
  maintained Earth baseline family:
  - `radius_nominal = 800m`
  - `7` arc points
  - `3` velocity bands
  - `smoke` and `full` seed tiers
- Replaced the old provisional bot-lab pack with a real matrix smoke suite:
  - `fixtures/packs/terminal_bot_lab_suite.json`
- Added the matching full-tier pack:
  - `fixtures/packs/terminal_bot_lab_full.json`
- Extended the batch report tree so terminal review now exposes:
  - `arc_point`
  - `velocity_band`
  between the vehicle bucket and lane/seed detail
- The first real Earth runs are already informative:
  - baseline only succeeds on a narrow slice of the matrix
  - current has zero successes on both smoke and full tiers
- That is a good outcome for the framework even though it is bad for the
  current controller. The suite is now doing real discriminating work instead
  of only approximating the intended corpus.

## 2026-04-17

### Current status

- Repo scaffolded as a Cargo workspace.
- Initial crates created:
  - `pd-core`
  - `pd-control`
  - `pd-cli`
  - `pd-eval`
- `fixtures/scenarios/` created for authored scenario inputs.
- First `pd-core` contract slice is now implemented and compiling.
- First single-run path is now implemented through `pd-cli`.
- Early-termination mission evaluation now has a first concrete timed-checkpoint
  goal shape.
- Replay bundles now include `scenario.json` and can be replayed without an
  external scenario path.
- `pd-eval` now has a first minimal pack runner and summary output path.
- The docs are now realigned around the next real milestone: turning the thin
  controller loop into a proper bot framework with inspection, seeded coverage,
  and eval-side native parallelism.
- The controller layer now emits structured controller frames with:
  - command
  - status
  - phase
  - metrics
  - report/debug markers
- `pd-cli` now writes controller config and controller-update traces into
  artifact bundles and can generate a first static single-run inspection report.
- CLI defaults now write repo-local artifacts under `outputs/` so generated
  bundles and reports are easier to find during iteration.
- The single-run HTML report has now been reworked around a denser Plotly-based
  layout with a large spatial plot, compact metric panels, hover inspection,
  and summarized events/markers instead of verbose raw tables.

### Active implementation focus

1. Extend `pd-eval` with scenario families, seeded coverage, and deterministic
   native multithreading.
2. Tighten the first report path beyond a single static run page:
   - richer event/marker inspection
   - better batch-level summaries
   - easier regeneration from batch bundles
3. Keep artifacts simple and authoritative:
   - run manifest
   - action log
   - event log
   - controller update trace
   - optional sampled/debug trace

### Notes

- Start contract-first. Avoid building physics details before command, terrain,
  mission, and artifact types are stable.
- Use a fixed physics step with lower-rate controller updates and held commands.
- Treat sampled traces as optional report/debug caches, not the replay source of
  truth.

### Checkpoints

#### Checkpoint 1: `pd-core` contracts and loop

- Added the first shared domain types:
  - sim config and cadence validation
  - terrain and landing pad specs
  - vehicle geometry/specs
  - scenario/run context
  - observation, command, event, action, sample, and manifest artifacts
- Implemented a minimal deterministic simulation loop with:
  - fixed-rate physics stepping
  - lower-rate controller updates via held commands
  - simple hull/touchdown contact classification
  - terminal outcomes for on-target landing, off-target landing, crash, and timeout
- Added `pd-core` smoke tests for:
  - cadence validation
  - terrain interpolation
  - authoritative action/event/sample capture

### Known limitations

- Contact/landing logic is intentionally simple and will need refinement before
  richer terrain or transfer-style evaluations.
- `pd-eval` currently produces only JSON bundle output and summary counts. It
  does not yet provide baseline-to-baseline diffs, thresholds, or richer
  aggregate metrics.
- Scenario authoring is still pinned-case only. Scenario families, seed sweeps,
  and randomized coverage have not been implemented yet.
- `pd-eval` is still single-threaded. Native parallel execution for multi-seed
  and multi-scenario runs is still design-only.
- Replay bundles now capture controller config and controller traces, but replay
  still treats those as carried-through inspection data rather than as
  authoritative replay inputs.
- The first report path is single-run only. `pd-eval` bundles can be rendered
  through `pd-cli report`, but batch output does not yet generate polished
  report pages directly.

#### Checkpoint 2: baseline runner path

- Added `pd-control` controller traits plus:
  - `IdleController` for debugging
  - `BaselineController` for the first closed-loop landing path
- Added `pd-cli run <scenario>` with:
  - scenario JSON loading
  - controller selection
  - optional artifact file output
  - manifest printing for quick inspection
- Added the first authored scenario fixture:
  - `fixtures/scenarios/flat_terminal_descent.json`
- Replaced leftover crate stubs so the workspace shape better matches the repo
  design docs.
- Validated the first authored scenario by running the baseline controller to an
  on-target landing through `pd-cli`.

### Implementation notes

- The touchdown footprint is now treated as independent from the hull bounds.
  Gear can extend below or wider than the body, which matches the design docs
  better than constraining touchdown points inside the hull box.
- The landing gate uses a small touchdown settle band instead of requiring both
  points to hit in the exact same fixed step. That keeps the first discrete-time
  loop from misclassifying an otherwise safe touchdown as a crash.

#### Checkpoint 3: replay contract hardening

- Added a replay path that consumes logged controller actions instead of a live
  controller callback.
- Added artifact bundle output as separate files:
  - `manifest.json`
  - `actions.json`
  - `events.json`
  - `samples.json`
- Verified replay by comparing the reproduced manifest and event stream against
  the original bundle.

#### Checkpoint 4: mission evaluation split

- Added a dedicated `pd-core` evaluation module so contact detection and mission
  outcome mapping are no longer the same function.
- The simulation loop now:
  - detects contact classification from physics state
  - routes that classification through mission evaluation
  - keeps timeout handling in the same outcome layer
- This does not add new mission types yet, but it makes the next step
  clearer: early-termination goals should plug into the evaluation side rather
  than into low-level contact code.

#### Checkpoint 5: first early-termination goal

- Added `timed_checkpoint` as the first concrete non-landing mission type.
- The goal evaluates an in-flight state envelope at a configured end time,
  relative to the designated target pad.
- Added an authored scenario fixture:
  - `fixtures/scenarios/timed_checkpoint_idle.json`

#### Checkpoint 6: self-contained replay bundles

- Added `seed`, `tags`, and `metadata` to `ScenarioSpec`.
- Replay bundles now write `scenario.json` alongside:
  - `manifest.json`
  - `actions.json`
  - `events.json`
  - `samples.json`
- `pd-cli replay` can now reconstruct from the bundle alone without requiring a
  separate scenario path.

#### Checkpoint 7: minimal batch evaluation

- Added the first `pd-eval` batch runner and `pd-eval run-pack`.
- Added a pack spec and the first named suite:
  - `fixtures/packs/core_suite.json`
- Batch eval now emits per-run bundles plus one JSON summary with:
  - total runs
  - success count
  - mean sim time
  - mission-outcome counts
  - end-reason counts

#### Checkpoint 8: bot-framework realignment

- Reviewed the new project against `pylander`'s older bot framework and captured
  the gap explicitly:
  - the new repo shape and replay/eval boundaries are cleaner
  - the controller-facing contract, telemetry, and inspection path are still too
    thin for a real bot lab
- Reordered the design priorities accordingly:
  - minimal report/inspection moves into the near-term workflow
  - seeded scenario coverage becomes a first-class evaluation concern
  - eval-side native multithreading is now part of the intended batch-execution
    model
- Clarified the migration stance:
  - reuse scenario ideas and proven scenario shapes from `pylander`
  - do not transliterate the old bot interface or scenario files directly

#### Checkpoint 9: first proper bot-framework slice

- Replaced the thin controller callback with a richer controller frame contract:
  - `command`
  - `status`
  - `phase`
  - `metrics`
  - `markers`
- Added controller config/spec support for built-in controllers plus JSON-loaded
  controller configs in `pd-cli` and `pd-eval`.
- Updated bundle outputs to include:
  - `controller.json`
  - `controller_updates.json`
- Added the first static single-run inspection report:
  - terrain and trajectory view
  - event and marker overlays
  - sampled state inspection
  - basic time-series charts for altitude, velocity, throttle, and attitude
- Added `pd-cli report --bundle-dir ...` so report generation can be rerun over
  captured bundles.

#### Checkpoint 10: repo-local output defaults

- `pd-cli run` now defaults to `outputs/runs/<scenario>__<controller>/` when no
  output path is specified.
- `pd-cli replay` now defaults to `outputs/replays/<bundle>/` when no output
  path is specified.
- `pd-eval run-pack` now defaults to `outputs/eval/<pack>/`.
- `/outputs` is ignored in git so generated artifacts stay local.

#### Checkpoint 11: report density pass

- Reworked the report layout after comparing it with the more mature
  `pylander` viewer shape.
- Borrowed the useful parts conceptually:
  - Plotly-based interactive plots
  - one primary spatial panel
  - compact metric panels instead of long stacked sections
  - hover-driven inspection
  - sparse event markers and summaries instead of long event tables
- Kept the new report simpler than the old `pylander` viewer:
  - no direct transliteration of the old HTML structure
  - still static-output-first and bundle-driven

#### Checkpoint 12: report layout cleanup

- Tightened the report layout after direct visual inspection of generated HTML
  pages in a browser.
- Removed redundant inner Plotly titles so the panel headings remain the only
  titles on the page.
- Reduced chart clutter by:
  - disabling the modebar on compact metric plots
  - simplifying chart margins
  - moving compact-plot legends out of the title/modebar collision zone
- Tightened spacing across the dashboard so the page reads more like a dense
  inspection surface than a stack of roomy cards.
- Moved compact metric-plot legends into the top chart margin so they no longer
  collide with the x-axis title.

## 2026-04-18

### Current status

- Reports can now be served from a repo-owned script instead of ad hoc shell
  commands.
- The intended output-path stance is now captured explicitly:
  - keep single-run report paths stable and semantic
  - reserve digests and commit/workspace keys for batch caches and compare
    workflows
- `pd-eval` now expands scenario families into resolved seeded runs, records the
  resolved perturbation parameters per run, and can execute those runs on a
  native thread pool while preserving stable output ordering.
- Batch outputs now include:
  - `pack.json`
  - `resolved_runs.json`
  - `summary.json`
  - per-run bundles under `outputs/eval/<pack>/runs/`
- Batch summaries are now rich enough to surface:
  - outcome counts by entry and family
  - failed seeds
  - slowest runs
  - stable digests for the pack spec and resolved run set
- `pd-eval` now emits a first static batch `report.html`, and batch comparison
  against a baseline directory is now a first-class workflow rather than only a
  future design note.
- The report server now binds on all interfaces by default and prints a
  detected LAN URL so remote devices on the local network can open generated
  reports directly.
- A separate `outputs/reports/` tree now exists as the primary navigation
  surface for HTML pages, so report browsing no longer starts in raw artifact
  directories.
- `outputs/index.html` now acts as a generated root landing page above the
  report tree, so the server root can point at one stable navigation page.

#### Checkpoint 13: report server script and artifact-key review

- Added `scripts/serve-reports` as the canonical local report-serving entrypoint.
- The script:
  - serves `outputs/` over `python3 -m http.server`
  - runs inside a named detached `tmux` session by default
  - supports `start`, `stop`, `restart`, `status`, `attach`, and `url`
- Documented the workflow in the repo README so remote-over-SSH usage has one
  obvious entrypoint.
- Reviewed `pylander`'s artifact identity approach and kept the useful split:
  - semantic stable keys for per-run inspection paths
  - short digests and commit/workspace identity only for cache-heavy benchmark
    and compare outputs
- Deliberately did not pull over the heavier tracepack/cache naming machinery
  yet, because `pd-lab` does not have the equivalent compare workflow in-tree
  yet.
- Added `latest` symlinks for report-producing `pd-cli` workflows so the most
  recently written single-run or replay report is reachable through a stable
  convenience path.

#### Checkpoint 14: seeded sweeps and multithreaded batch eval

- Extended `pd-eval` pack entries from only pinned scenarios to two concrete
  forms:
  - `scenario`
  - `family`
- Added scenario-family expansion with:
  - explicit seed lists
  - seed ranges
  - deterministic numeric perturbations with optional quantization
- Kept the ownership boundary intact:
  - `pd-core` still runs one resolved concrete scenario
  - `pd-eval` owns family expansion, seed policy, and multirun orchestration
- Added stable resolved-run identity:
  - semantic run IDs such as `terminal_baseline_sweep_seed_0003`
  - recorded `family_id`, resolved seed, and resolved perturbation values
- Added deterministic batch identity with separate digests for:
  - the pack spec
  - the resolved run set
- Added native multithreaded execution in `pd-eval` via a bounded worker pool.
- Preserved deterministic output order and verified that sequential and
  multithreaded execution produce the same resolved-run digest.
- Added richer batch summary output:
  - mission and physical outcome counts
  - end-reason counts
  - grouped summaries by entry and by family
  - failed-run pointers with seed and bundle location
  - slowest-run pointers for quick inspection
- Added a first seeded sweep fixture.
  It has since been demoted and renamed into the explicit compare smoke
  fixtures:
  - `fixtures/packs/terminal_compare_baseline_fixture.json`
  - `fixtures/packs/terminal_compare_regression_fixture.json`
- Added `latest` symlink maintenance for `pd-eval` output directories so the
  most recent batch result is reachable through:
  - `outputs/eval/latest/`
  - `outputs/eval/<pack>/runs/latest/`

#### Checkpoint 15: first batch report and baseline compare

- Added a first static batch-report page at:
  - `outputs/eval/<pack>/report.html`
- Added `pd-eval report <batch-dir>` so batch pages can be regenerated from an
  existing output directory.
- Added optional baseline comparison through `--baseline-dir` on:
  - `pd-eval run-pack`
  - `pd-eval report`
- Added `compare.json` as a first-class batch artifact when a baseline is
  supplied.
- The compare model currently uses shared `run_id` matching and records:
  - compare basis (`shared`, `candidate only`, `baseline only`)
  - global candidate-vs-baseline deltas
  - grouped deltas by entry and family
  - regression, recovery, and outcome-change run lists
- Borrowed the useful part of the `pylander` batch-report approach:
  - comparative reports matter more than standalone summaries
  - grouped scenario views and representative run links are more useful than a
    giant flat table
- Kept the first `pd-lab` batch report simpler than the late `pylander` bundle
  pages:
  - dense HTML tables and cards first
  - no heavy chart layer yet

#### Checkpoint 16: batch cache reuse and promotion

- `pd-eval run-pack` now has a real cache layer under:
  - `outputs/eval/cache/<workspace-or-commit-key>/<batch-stem>/`
- Candidate cache identity is now derived from:
  - clean commit key or dirty workspace key
  - pack id
  - pack spec digest
  - resolved run digest
- `run-pack` now reuses a complete cache automatically by default instead of
  always rerunning the batch.
- Added explicit cache metadata in `meta.json` and threaded cache provenance
  into `summary.json` and the batch `Context` section.
- Added `--compare-ref auto|<ref>|none` to `pd-eval run-pack`:
  - dirty workspaces default to comparing against the clean `HEAD` cache
  - clean workspaces default to comparing against the previous clean commit
    cache
- Added `pd-eval promote-cache <pack>` so a validated dirty cache can be copied
  onto the clean commit key after checkpointing, instead of rerunning the old
  code just to recover a baseline.
- The batch `Context` section now explicitly reports:
  - current cache status (`fresh`, `reused`, `promoted`)
  - baseline cache source / compare ref resolution
  - when a compare was requested but no baseline cache was available
  - drill-down links currently target bundle artifacts unless a per-run report
    already exists

### Active implementation focus

1. Keep growing the controller kit rather than letting built-ins drift apart:
   - richer helper queries over terrain and target frame
   - shared metric/marker conventions across controller styles
   - small reusable guidance utilities instead of copy-pasted heuristics
2. Use the new bot-lab suite to tighten controller iteration:
   - compare baseline vs staged behavior on the same curated families
   - rank which failures are controller issues vs corpus issues
   - decide which scenario families deserve first-class promotion later
3. Tie batch and run reports together more tightly:
   - stronger drill-down from compare rows into run pages
   - more stable labeling of current vs baseline review links
4. Keep improving report meaning before report polish:
   - better representative-run selection
   - clearer "open this next" guidance

### Known limitations

- `pd-eval` now has a real batch report page, but the current version is still
  table-first and intentionally minimal compared with the older `pylander`
  report stack.
- Scenario-family perturbations currently cover only a small set of numeric
  fields; terrain mutation and richer family grammars are still future work.
- Batch identity digests exist, but there is not yet a cache or compare layer
  that actively uses them.
- Replay remains authoritative on scenario plus action/event logs; controller
  telemetry and sampled traces are still carried-through inspection data.
- Batch compare currently matches runs by shared `run_id`. That is the right
  first compare basis, but it is not yet a richer “same resolved scenario under
  different candidate/controller configs” matcher.
- `pd-eval` still does not generate polished per-run drill-down pages itself, so
  batch links currently fall back to bundle artifacts when no `report.html`
  exists for that run.
- The controller kit now exists, but it is still centered on the current
  terminal-descent problem. It has not yet been pressure-tested against richer
  transfer guidance or terrain-reactive scenarios.

#### Checkpoint 16: report-site structure and indexes

- Split stable HTML navigation away from raw artifacts by introducing:
  - `outputs/reports/index.html`
  - `outputs/reports/runs/`
  - `outputs/reports/replays/`
  - `outputs/reports/eval/`
- `pd-cli report` now defaults to the report-site path for bundles rather than
  writing back into the bundle directory.
- `pd-cli` single-run and replay bundle writes now also emit report-site copies
  automatically when the bundle lives under `outputs/`.
- `pd-eval` batch report generation now also emits report-site copies under:
  - `outputs/reports/eval/<pack>/index.html`
- Added simple generated index pages for the report-site home and each top-level
  scope, sorted newest-first.
- The server workflow now points to the root landing page first, with the
  report-only subtree still available under `/reports/`.
- Added a generated root landing page at `outputs/index.html` with links to:
  - the report site
  - raw artifact directories
  - latest run and latest batch pages
- Follow-up fixes tightened the navigation contract:
  - newest-first ordering now keys off the generated report file timestamp
    instead of the directory timestamp, so regenerating a report actually moves
    it to the top
  - nested report collections such as `outputs/reports/eval/<pack>/runs/` now
    get their own `index.html` page instead of dropping back to a raw directory
    listing
  - batch report drill-down links now prefer report-site detail pages when they
    exist, keeping navigation inside `/reports/`

#### Checkpoint 17: run summaries and compare-first batch triage

- Added structured per-run summary metrics to `RunManifest.summary` and bumped
  the run artifact schema to `2`.
- The run summary now carries:
  - fuel used and fuel remaining
  - minimum touchdown and hull clearance
  - max speed, attitude magnitude, and angular-rate magnitude
  - mission-envelope margin ratios
  - landing-specific terminal metrics:
    - pad offset
    - normal and tangential touchdown speed
    - attitude and angular-rate touchdown margins
  - checkpoint-specific terminal metrics:
    - position, velocity, and attitude error
    - checkpoint-envelope margins
- Reworked batch summaries around review value instead of only flat inventories:
  - closest current failures
  - worst failures
  - weakest successes
  - lowest-fuel successes
  - mean remaining fuel for successful groups
- Reworked compare ordering to rank shared runs by envelope-margin regression
  first, then by fuel and timing deltas.
- Borrowed the useful lesson from the late `pylander` benchmarking flow:
  the first question is usually not "show me every run", it is:
  - what regressed
  - what is closest to the edge
  - which run should I open next
- Updated the batch HTML page to surface that directly:
  - `Current Edge` cards on the overview
  - compare-time `Priority Review`
  - standalone `Closest Current Failures`
  - `Representative Successes`
  - failure inventories and slowest runs pushed into secondary `details`
    sections
- Kept the new run summaries and compare surfaces aligned with the report-site
  tree, so report navigation and batch triage now reinforce each other.

#### Checkpoint 18: controller kit and curated bot-lab corpus

- Split `pd-control` into a clearer bot-framework shape:
  - shared controller view/helpers in `pd-control/src/kit.rs`
  - concrete built-in controllers in `pd-control/src/controllers.rs`
- Added a reusable `ControllerView` over `RunContext + Observation` with:
  - target-relative position helpers
  - pad-margin and fuel-fraction helpers
  - target-surface normal/tangent kinematics
  - hover-throttle and vertical-target helper calculations
  - direct terrain sampling/profile helpers for controller code
- Added a small shared diagnostics convention layer:
  - common phase labels
  - common metric keys
  - standard phase/gate marker helpers
  - a `ControllerFrameBuilder` so built-ins stop hand-assembling frames
- Refactored `baseline_v1` onto the shared controller kit.
- Added a second built-in controller, `staged_descent_v1`, with:
  - explicit `translate -> descent -> flare -> touchdown` staging
  - standardized phase and gate markers
  - distinct behavior from the original baseline instead of a renamed clone
- Added a curated pack for controller iteration:
  - `fixtures/packs/terminal_bot_lab_suite.json`
- The new pack intentionally exercises both built-in controllers over:
  - nominal terminal runs
  - crossrange / attitude-biased terminal runs
  - low-margin / low-fuel terminal runs
  - one checkpoint reference
- This is the first point where `pd-lab` feels closer to a real bot framework
  rather than just a deterministic sim with batch tooling:
  - controllers now have a reusable helper layer
  - multiple controller styles share the same lab contract
  - the corpus is shaped for controller work, not only for smoke testing

#### Checkpoint 19: terminal-guidance selector realignment

- Reviewed the current terminal bot-lab pack against the old `pylander`
  selector model and confirmed the deeper issue was not only report layout:
  the corpus was still too flat:
  - one coarse `family`
  - one overloaded `entry_id`
  - one leaf `seed`
- Locked the intended terminal-guidance selector model into the architecture:
  - hierarchy axes:
    - `mission`
    - `arrival_family`
    - `condition_set`
    - `vehicle_variant`
  - matrix axes:
    - `arc_point`
    - `velocity_band`
  - leaf variation:
    - `seed`
  - controller lane:
    - separate from the physical case
- Documented the current implementation stance clearly:
  - current packs still approximate that model with `family + entry + seed`
  - pack metadata should carry explicit selector fields even before the report
    layer grows first-class matrix support
- Simplified the active terminal bot-lab corpus to reduce noise while that
  selector work is still in progress:
  - removed the dedicated `crossrange` family
  - kept clean nominal and low-margin terminal cases
  - recorded expectation tiers so not every case is implicitly treated as
    "must be solved"
- The next structural step is no longer "add more families". It is:
  - teach `pd-eval` about explicit terminal selector coordinates
  - replace ad hoc seeded arrival perturbations with a denser
    `arc_point x velocity_band` matrix

#### Checkpoint 20: suite simplification around terminal bot lab

- Recentered the day-to-day evaluation workflow on:
  - `fixtures/packs/terminal_bot_lab_suite.json`
- Demoted the old sweep/regression packs from primary examples to explicit
  compare smoke fixtures:
  - `fixtures/packs/terminal_compare_baseline_fixture.json`
  - `fixtures/packs/terminal_compare_regression_fixture.json`
- Updated the README batch-eval examples so the main example points at the
  bot-lab suite, while external pack-vs-pack compare is shown only through the
  fixture pair.
- Cleaned up generated outputs so the report index no longer surfaces the old
  sweep suite names as if they were first-class review surfaces.

#### Checkpoint 21: batch report provenance and compare context

- Added an explicit `Context` section near the top of batch reports so the page
  says what kind of report it is before the overview/tree:
  - `standalone`
  - `lane compare`
  - `external compare`
- The context table now surfaces:
  - current source
  - baseline source
  - baseline resolution
  - compare basis
  - scope resolution
  - compare status
- External compare pages now distinguish:
  - exact compare
  - shared-intersection compare
  - unavailable compare
- Internal bot-lab pages now say clearly that the comparison is lane-based
  within one pack, rather than an external baseline compare.
- The page also states a current limitation explicitly:
  - cached result reuse / promotion / invalidation is not modeled in `pd-lab`
    yet, so the report does not pretend to show it.
