# Progress

## 2026-04-17

### Current status

- Repo scaffolded as a Cargo workspace.
- Initial crates created:
  - `pd-core`
  - `pd-control`
  - `pd-cli`
  - `pd-eval`
- `fixtures/scenarios/` created for authored scenario inputs.

### Active implementation focus

1. Freeze the first shared contracts in `pd-core`.
2. Wire a minimal `pd-cli run` path for one flat terminal-descent scenario.
3. Keep artifacts simple and authoritative:
   - run manifest
   - action log
   - event log
   - optional sampled/debug trace

### Notes

- Start contract-first. Avoid building physics details before command, terrain,
  mission, and artifact types are stable.
- Use a fixed physics step with lower-rate controller updates and held commands.
- Treat sampled traces as optional report/debug caches, not the replay source of
  truth.
