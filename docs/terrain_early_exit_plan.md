# Bounded early-nominal exit pass

Status: **completed bounded implementation and validation**; see the
[results](terrain_early_exit_results.md). Derived from the
[19-probe handoff result](terrain_handoff_probe_results.md). This is the smallest
candidate supported by the measured contrast, not a universal planner repair.

## Goal and exclusions

Avoid coasting past a usable nominal solely to reach the old nominal's conflict-x
gate. Test one generic early exit on an already selected clearing maneuver, while
preserving the existing waypoint-clearing fallback and all safety/replay bounds.
Do not retune the nominal constructor, change controllers, add per-seed/recipe
rules, globally rerank local rows, relax reserve, or require direct landability
after every ordinary waypoint. Default correction-cap promotion is separate.

## Candidate contract

1. Run the existing nominal audit/local search unchanged and select the same
   native clearing maneuver. Keep its complete accepted proposal and original H
   certificate as evidence/fallback; do not alter them to pretend an earlier
   point passed the old progress gate.
2. Consider **one** earlier point: the first post-burn, command-aligned, upright,
   fully settled, zero-throttle coast boundary of that selected proposal. It must
   be strictly before H and remain within the original absolute deadline.
   Reconstruct it forward from actual E, with unchanged consumed commands/fuel.
3. Require the unchanged body reserve and supported family throughout the next
   two seconds on the selected coasting maneuver. Query the unchanged nominal
   constructor once at that actual state, then its actual-terrain audit.
4. If and only if the nominal/audit passes all existing safety and integrity
   checks, consume the clearing prefix only to that point, prove the complete
   source prefix and commit the exact checked nominal as the next piece.
   Bind its incoming state, command clock, fuel and deadline to the actual exit.
   Do not discard it and run another selection that could pick something else.
5. If the query is finite-infeasible, unsupported or terrain-blocked, execute
   the original clearing maneuver to its original H with unchanged selection,
   commands and ordinary outcome. Never execute the blocked early nominal.
   Proof/integrity errors fail closed; they are not quiet finite fallbacks.

This early branch requires a clear direct nominal because it exits the current
clearing piece early. **The ordinary waypoint branch still needs only its local
clearing/continuation certificate**, not a mandatory landing suffix. A later
phase may define an earlier blocked-route waypoint handoff, but its semantics
and additional correction execution are not established by this experiment.

## Evidence/API design checkpoint before coding

Review how to represent **actual piece end versus original clearing witness H**.
Do not overwrite the native proposal's H, falsify its progress result, or weaken
its validator. An explicit optional early-exit record should bind original row/H,
query state, eligibility checks, exact selected nominal and actual segment end.
Existing saved policy-3 records must decode without it.

The owned session must preserve exclusive command ownership at the exit pair,
normal post-step physical state, source/action replay and failure retention.
The exit counts as one actual local correction, not a fictitious extra waypoint;
an unused query must not appear as an executed handoff. Common detail reports
must annotate the actual end and retain the original witness as query evidence,
without replacing rich plots or navigation. Do not publish a new site by default.

Confirm this representation with bundle/progress/report validators before starting
implementation. Preserve sole current policy ownership; do not reintroduce V1 or
a general research-mode interface. Keep experimental source/capture provenance
explicit and ordinary release/accepted-selector files unchanged during diagnosis.

## Bounded first execution gate

Freeze the same seven input byte identities before coding/running. On the five
subjects, preserve the selected maneuver and its exact command prefix to the
earliest coast point. Use the relaxed-cap diagnostic setup only under explicit
experimental provenance; do not silently replace default cap 6 or old captures.

- `967/955/024`: demonstrate the intended earlier actual exit and a complete
  target-landing tuple: planner landed, physical target touchdown, mission
  success, integrity passed and original-source replay passed. An audit alone
  is not acceptance.
- `983/516`: demonstrate an early terrain-blocked query that is **not executed**;
  fallback preserves the original selected piece, H and complete ordinary flight
  endpoint. Do not add ranking/braking fixes to make them green in this pass.
- `271`: require a complete verified landing and source replay; early exits at
  earlier hops may legitimately alter its route, so do not demand a false
  whole-flight identity. `000`: require exact unchanged direct commands/outcome,
  with no local correction/early-query branch.
- Repeat `967` and `983` once each. **Nine primary/control/repeat missions**;
  no retries or replacement inputs in the fixed matrix. Existing per-case and
  collection wall limits remain explicit, and partial failures are retained.

Add synthetic tests for the one-query bound, no usable earlier boundary,
unsupported/blocked finite fallback, integrity-error retention, exit-pair command
ownership, actual-state/queued-nominal binding, old record decoding and exact
actual-versus-witness report annotations. Run the maintained developer gate.

Separately run the normal 44-case native acceptance pack, without publishing or
changing the accepted selector. All 36 core landings and existing integrity/replay
contracts must pass; diagnostics/unsupported cases stay separate. Corrected
flights may legitimately change when an earlier exit is used, so compare outcomes,
contracts and real prefixes rather than demanding historical flight identity.
Require exact preservation for unchanged direct routes and declined early exits.

## Stop and wider-evaluation decision

Stop the candidate on replay/proof errors, lost control/core landings, changed
fallback commands, altered source inputs, violated safety bounds, more than one
early query per selected maneuver or need for case-specific conditions. Diagnose
without broadening the candidate into a second fix within the same matrix.

If the bounded execution/acceptance gates pass, review source and evidence before
authorizing a paired rerun of the same 1k population. Require all 748 prior
landings preserved, explicit counts of early exits/fallbacks, unchanged clear
routes and honest new nominal/clearing stops. Only then use a separately frozen
fresh 100-world test sample for generalization; never tune on it or count the
five diagnostic subjects as held-out evidence.

Do not predict a new pass rate from 3/5 deliberately selected clear audits.
The two blocked early states, other nominal failures and 154 local-clearing
exhaustions remain distinct next capability questions. If the optional early
exit gives little broad benefit, inspect those shared mechanisms before adding
another scalar ranking heuristic. The user authorized only the nine-mission
matrix and separate normal 44-case pack for this pass. The broader 1k/fresh-100
flights, production-cap promotion, publication, commit and push remain excluded.
