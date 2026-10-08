# Intervention timing comparison: useful coverage, rejected replacement

Date: 2026-10-07. Completes the bounded [timing pass](intervention_timing_plan.md)
at its focused acceptance gate. The accepted benchmark and original
[100-case challenge](terrain_challenge_results.md) remain unchanged.

## Verdict and final source

The timing blind spot is real: the candidate produces four new verified landings.
But replacing the old entries with four conflict-relative samples also loses a
successful multi-waypoint control and fails the existing `v2_ridge_early` CLI
landing test. **Do not adopt this replacement.** Previous timing is restored in
the working source; compact query diagnostics and their collapsed rich-report
view remain. No failed planner algorithm or alternative executable selector is
left in the active source. Candidate source is retained inside its capture.

The maintained eleven-step development gate passes after restoration, including
the unchanged ridge CLI test. The conditional accepted 44-case capture, full
100-case recheck and two repeats were not launched. There is no fresh full-pack,
final-source flight-campaign or arbitrary-terrain reliability claim.

## Frozen comparison

The source-frozen diagnostics baseline and timing candidate each execute the
same 16 saved challenge scenarios: 32 campaign invocations, with no terrain
regeneration, changed inputs or outcome-based filtering. Normal unit/CLI tests
are separate from these measured populations. All 32 have passing integrity and
final-source replay; no campaign case physically crashes or times out.

Baseline logging adds no motion changes: complete flight records match the
original challenge excluding only the three wall timings and the optional
`row_diagnostics` fields. Historical absent metadata round-trips unchanged.
The candidate keeps nominal construction, terminal controller, 42 templates,
ranking, five-metre reserve, two-second continuation and six-correction cap.
It changes only intervention clocks and their derived entry/row identities.

| Measure | Diagnostics baseline | Timing candidate |
| --- | ---: | ---: |
| Cases | 16 | 16 |
| Initially blocked / clear | 15 / 1 | 15 / 1 |
| Verified landings | 3 | 6 |
| `NoClearing` | 13 | 6 |
| `NoNominal` | 0 | 4 |
| Integrity / source replay | 16 / 16 | 16 / 16 |

Four gains and one lost control yield a net gain of three in this selected
diagnostic subset. It is not a held-out success estimate or permission to replace
the original 57/100 challenge result with an extrapolation.

| Case | Before | Candidate | Candidate handoffs |
| --- | --- | --- | ---: |
| `000` | `NoClearing` | `NoNominal` | 1 |
| `013` | `NoClearing` | `NoClearing` | 0 |
| `023` | `NoClearing` | Landed | 1 |
| `034` | Landed | Landed | 0 |
| `035` | `NoClearing` | Landed | 6 |
| `043` | `NoClearing` | `NoClearing` | 5 |
| `049` | `NoClearing` | Landed | 1 |
| `050` | Landed | Landed | 2 |
| `054` | Landed | `NoNominal` | 6 |
| `056` | `NoClearing` | Landed | 5 |
| `060` | `NoClearing` | `NoNominal` | 1 |
| `070` | `NoClearing` | `NoClearing` | 0 |
| `085` | `NoClearing` | `NoClearing` | 3 |
| `093` | `NoClearing` | `NoClearing` | 0 |
| `094` | `NoClearing` | `NoNominal` | 6 |
| `099` | `NoClearing` | `NoClearing` | 0 |

The direct `034` control is motion-exact. `050` still lands, now with two rather
than four corrections. `054` fails the predeclared successful-control gate.
The existing tracked CLI ridge test independently rejects the candidate with a
zero-correction `NoClearing` stop; no fixture, test expectation or guard was
loosened. Restoration makes that test pass again.

## What this established

### Timing coverage genuinely matters

All five former zero-admission source cases now test actual maneuvers: `035`,
`043`, `056`, `085`, `093` admit respectively three, three, two, two and three
entries. `035` and `056` reach verified landing; two others clear initial terrain
but stop later. `093` still exhausts the family. Opening an entry is not a
landing or physical-feasibility proof.

The formerly launch-only late searches now include coast/terminal states.
`023` and `049` land after one correction; `000` and `060` clear locally but
stop `NoNominal` near x=1094/1098 m with about 47 m/s forward speed. The other
three late cases still cannot satisfy the current family/certificate checks.
This supports the timing diagnosis without claiming all such terrain is flyable.

### A window containing good entry times does not sample those times

All 110 prior executed corrections began within the proposed 14-second window.
That observation did not establish adequacy of four quarter-window samples.
For the existing ridge control, conflict time is 15.508 s and its successful
entry is 10.600 s. The candidate samples 1.500, 5.000, 8.500 and 12.000 s,
omitting that entry. The existing CLI landing test fails. This is a bounded
sampling limitation, not evidence that the ridge became physically impossible.

Timing also changes the selected handoff states through unchanged greedy ranking.
`054` used to finish its sixth correction at x=979.80 m, vx=71.34 m/s,
vy=-10.67 m/s. The candidate finishes at x=897.45 m, vx=87.39 m/s,
vy=+1.46 m/s; the next bounded nominal acquisition cannot fit a safe landing
profile. Local continuation still passes. This is not a certificate/replay defect
and is not a reason to impose a full landing guarantee at every waypoint.

### Better diagnostics are retained

Each local query keeps a compact row summary: exact cutoff reason and state,
minimum reserve, required progress, boundary-category counts and first failed
handoff/continuation. It does not save every trajectory, command or all 360
boundary records. The rich page adds a collapsed query-diagnostics table while
preserving common plots, payloads, handoff visualization and navigation.

A query trace may stop after its selected handoff certificate has already
passed. Its later cutoff is not an actual-flight crash or an invalidated handoff.
Historical data without the new field remains readable with unchanged encoding.

## Captures and review

- Logging-only baseline:
  `outputs/eval/planner_v2_random_terrain/capture-intervention-20261007-diagnostics`.
- Rejected timing candidate:
  `outputs/eval/planner_v2_random_terrain/capture-intervention-20261007-focus`.

Both bind the original manifest/input receipt, exact scenario bytes, copied
source inventory, executable hash, ordered attempt ledger and final artifact
receipt. Python and native common-report verification pass. The final working
source restores prior clocks and adds presentation-only diagnostic rendering
after the logging baseline; the two flight captures retain their actual source
identities rather than being relabeled as final-source campaigns.

From report home choose **Waypoint planning → Intervention timing comparison**.
Open `035` for departure recovery, `023` or `049` for late-obstruction recovery,
and `054` for the lost control. The instrumentation baseline is in report history.
Accepted selector/report bodies and prior captures remain intact.

Final read-only publication checks cover 34 pages (two batch pages and 32 rich
details), 296 local links, all 32 embedded rich payloads and the diagnostic
panel/row counts in the 30 cases that actually queried a clearing maneuver.
The two direct controls correctly have no query table. All 48 protected accepted
selector/report files match the frozen hashes. Representative `035` and `054`
detail URLs respond over the existing LAN server. These are structural/data
checks, not a separate human visual-acceptance claim.

## Recommended small follow-up, not started

Keep the working entry search first. Only when it has **no acceptable local
clearing**, try the conflict-relative entries as a fallback. Reuse the same
42-template family, guards and actual-handoff replanning; do not backtrack a
flown segment or require a landing suffix.

This gives a simple preservation argument: an old successful flight never needs
the fallback, so its existing commands and handoffs remain selected. It trades
additional bounded search work on exhausted cases for coverage, rather than
replacing useful sample times or retuning every terrain. Worst-case entry work
would increase, and new recovered flights still need verification. It does not
solve existing `NoNominal` or correction-cap stops automatically.

That additive fallback requires a new approved pass. No further clock tuning,
grid expansion, braking/ranking change, commit, push, server change or accepted
benchmark promotion was performed here.
