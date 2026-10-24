# Timing reference execution

This pass follows the [frozen qualification plan](timing_reference_plan.md).
Its create-only root is `outputs/timing-reference/loop-20261024T185303Z`.
The baseline binds initial head `2a87566`, 934 Git-visible file hashes and all
foreign changes. The outer deadline is Unix 1792878783; limits are one billion
charged steps, 1,200 captures, 12 GiB new artifacts, one native worker and a
40 GiB free-space floor. Fitting and RL updates remain zero.

## Initial declaration

The 63/132-pair comparison, 1% reference-selection rule, 300 independent primary
draws, 60 boundary cases, retained handoff regressions and isolated measurement
protocol precede all new campaign outcomes. No qualification inputs are generated
until the reference is frozen. Controller behavior, clocks, tolerances and old
outcomes remain fixed.

## Pilot and campaign release

Implementation commit `eac8470` adds the named 132-pair opt-in profile and
measurement sidecars. Five targeted Rust tests, strict targeted Clippy and nine
native Python tests pass, including full proof and measured/unmeasured identity.
The compiled source ledger points into the retained `source-epoch-a` copy.

The fixed pilot completed 12 Summary executions and four expanded captures with
eight reconstructions: 3,630,876 charged steps, 16,217,782 logical bytes and
2.875 s measured campaign time. All four captures verify target landing.
Maximum pilot forecast work is 14,172 / 112,016 / 200,584 steps for legacy,
63-pair and expanded selection respectively. This pilot establishes cost only.

`campaign-release.json` freezes phase caps before further outcomes: development
90 million, isolated benchmark 70 million, primary 500 million, boundary
100 million and retained regressions 100 million steps. Including the pilot's
25-million cap, reservations total 885 million within the one-billion outer
limit. Capture reservations total at most 1,165. The original three-hour,
12 GiB and free-space limits remain. Primary allocation is 1.667 million steps
per original, covering Summary, Captured, both reconstructions and neutral proof
for all three selectors. No primary inputs have been generated at release.

Development also reproduces both old profiles on all 180 old programs, rather
than checking only the 60 direct originals. This preservation check does not
expand the reference-selection or qualification denominator.

### Development reservation stop

The 90-million development phase stopped at 88,044,297 charged steps because the
next first frame requires a two-million-step reservation. It retained 407 complete
Summary rows and 58 captures with 116 reconstructions. The aborted attempt was
original 058's 63-pair Summary: its selection spent 90,894 forecast steps and
its first command advanced two native ticks before the next frame's reservation
failed. These partial costs remain in the fault receipt; this attempt makes no
landing claim. The pilot contained direct inputs and underestimated
the cost of reproducing the 120 old arrival programs twice.

`development/fault.json` remains unchanged. A separate create-only completion
phase reserves 20 million of the unused outer reserve for the exact 13 remaining
Summary slots and two expanded captures. Total reserved work becomes 905 million,
still within one billion. No completed slot is rerun, no input or selector is
changed, and no qualification inputs are opened before reference freeze.

The exact remaining schedule completed in its separate root. Both old selectors
reproduce all 180 retained programs exactly (360 complete executions).
All 60 expanded development captures verify target landing. Mean direct fuel
falls from 589.9276 to 570.1138 kg, a 3.3587% saving; the predeclared 1% rule
therefore selects the 132-pair reference. `reference-freeze.json` binds this
decision, source/library hashes and the input generator before any qualification
inputs are generated. The legacy default remains unchanged.

## Isolated measurements

The maintained developer gate passes all 19 checks. Isolated collection then
completed all 360 sequential benchmark invocations, charging 35,057,610 steps.
Every repeat preserves its profile's decision fingerprint. Each profile has
84 measured selections after its first invocation and two warmups per original.

Median selection wall time is 4.459 ms for legacy, 82.083 ms for 63 pairs and
109.920 ms for 132 pairs. Ordinary controller-frame medians are approximately
0.6 microseconds, excluding actuator allocation, native plant stepping and
Python/persistence overhead. The search is a planning cost and exceeds a 60 Hz
frame budget; these measurements do not establish real-time scheduling.
Raw durations, planning-frame durations, whole-episode process CPU, work counts,
affinity and host/load context are retained separately from deterministic info.

After reference freeze, input-only generation produced 300 independent primary
draws and 60 endpoint cases without native outcome admission or replacement.
All 360 physical initial states are unique and separate from exposed inputs.
The primary cells have 18–37 draws rather than fixed quotas; the engineering
panel has five per cell. All qualification profiles and retained regression
slots are bound to immutable allocations before execution.
