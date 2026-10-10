# Frozen terminal coordination: diagnostic 1k

[Documentation home](README.md) · [Selected panel results](ballistic_terminal_coordination_results.md)

Status: closed after all 1,005 invocations. The [results](ballistic_terminal_coordination_sweep_results.md)
record 639/1000 verified landings. The original premeasurement protocol is
receipt-bound in the capture; the contract below is not a new flight allowance.

## Authority and purpose

After reviewing the panel, the user explicitly requested “let's try 1k”. This
is a separate full-population diagnostic contract, not passing or retroactively
waiving the panel's strict preservation gate. The old conditional sweep remains
closed. The panel records 115/121 landings with two regressions (048/755).

Measure the unchanged V13 terminal-coordination candidate against the original
1,000 worlds. No tuning, compilation, trajectory-selection changes, default
promotion, accepted-site replacement, root-navigation refresh or server lifecycle
changes are authorized. Collection changes only establish this new contract.

## Frozen candidate and population

- Executable SHA-256: `89de0cffcb61adae04631c7ff25144201c949c7117c7e26e1ae72c17d1f1fc77`.
- Rust tree SHA-256: `2c42d3691ee8ca675a3c3332871c47e20969b2c3036c32cf0eed0469191fc899`.
- Shared batch renderer SHA-256: `fe0e9720226321d6596f9f4f93068fd68b4d5ffc6ccefed8b8a71b4ad213178d`.
- Candidate: `ballistic_feedback_v13_terminal_coordination`, explicit
  `terminal-coordination` opt-in, experimental correction cap 24.
- Exactly 1,000 original scenarios, seeds and geometry copied as authenticated
  bytes from the V10 full capture. No terrain, pad, clock, fuel or deadline edits.
- Five external repeats: 142, 150, 288, 757, 974. They are outside the denominator.
  Three direct controls already measured in the panel are not additional flights.
- Maximum 1,005 invocations; zero retries. Four workers, waves of twenty,
  60-second per-case and 7,200-second campaign wall limits.

The sealed panel receipt is
`12e232a09a929d8725b6feee81dd311f6f1401fd6744f6c7e57b614e43980293`;
its result hash is
`c55328ce66f624ad5e8228b47f816bc4f128b86236b1e33ca0533517d0895640`.
Flight binary, Rust, renderer and planning-cycle script identities must match
that measured panel. The collector/protocol are separately frozen before running.

## Evidence and stopping

Use create-only destination
`outputs/eval/planner_v2_random_terrain/capture-terminal-coordination-diagnostic-1k-20261009-v1`.
Retain scenarios, candidate source/binaries, previous receipts/results,
authenticated complete V12 flights, full commands/updates, physical/mission
outcomes, integrity, exact source command replay and decision reproduction.
Keep finite airborne stops in the denominator; a prediction-domain stop is not
an executed crash. No incomplete record is a verified landing.

Stop the campaign on runner/proof failure, an actual crash/off-target landing,
actual terrain-domain violation, source/protected-evidence drift or wall limit.
Retain the partial inventory and report; do not retry or repair flight behavior
inside this allowance. Finite guarded stops do not stop the campaign.

## Review and interpretation

Verify all 1,005 records and exact repeats; all 121 overlapping panel worlds must
reproduce their complete feedback bytes. Check every available V12 preterminal
command/planning prefix (including unchanged full nonterminal flights). The 16
V12 exceptions remain unverified historical outcomes, never invented flights.

Report the fixed 1,000-world denominator, clear/blocked and recipe breakdowns,
typed remaining stops, and paired gains/losses against V12 (592) and V10 (562).
Keep the policy-3 cap-24 reference (817) separate. The common batch renderer's
previous-candidate column compares V10, not V12; label that honestly and provide
the V12 comparison in results. Improvements do not imply automatic adoption or
held-out generalization. Preserve the panel's failed admission verdict.

Run:

```sh
rtk proxy python3 -B scripts/run-terminal-coordination.py diagnostic-sweep
rtk proxy python3 -B scripts/run-terminal-coordination.py verify-diagnostic
```
