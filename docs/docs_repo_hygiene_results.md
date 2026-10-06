# Documentation and repository hygiene review (2026-10-06)

[Documentation home](README.md) · [Development](development.md) · [Archive](history.md)

## Verdict and scope

The preparation details below retain their pre-review state. The review section
at the end closes that handoff without relabeling earlier validation evidence.

The pending documentation reorganization is review-ready above committed source
`7039b33`. Current workflow and ownership instructions now agree with the live
tools; historical records and their original outcomes remain preserved. This
does not establish new flight capability, terrain coverage or host integration.

This pass reviewed and refined the previous uncommitted docs work. It ran in the
primary agent without delegation. No Rust implementation, Cargo manifest/lock,
frozen JSON input, planner policy, captured flight or report template changed.
No new acceptance campaign, publication, server lifecycle change, commit or push
was performed. The separate earlier docs checkpoint remains in Progress.

## Findings resolved

- The root README is a task-first overview. Detailed evaluation/report operations,
  controller snapshots, project rationale and the complete research catalog live
  behind the documentation home, not another flattened research ledger.
- Current contract pages now link back to the docs home and relevant workflows.
  All repository Markdown pages are reachable from the root README through local
  Markdown links. Old root README anchors and the earlier navigation-entrypoint
  anchor remain available.
- Architecture no longer describes retired V1 chord/corridor/all-leg validation
  as live core APIs. The current exact point-envelope terrain query and the
  evaluator's realized-flight/contact audit are distinguished from historical DTOs.
- Evaluation documents native `--no-publish`, the different feature-gated CLI
  argument names, create-only bundles, preflight and saved-command replay.
  Native collection, strict CLI landing success and recorded-evidence checking
  are distinguished rather than sharing a misleading exit-code claim.
- Reports documents the actual default refresh scope, selected native capture
  regeneration, intentional saved-capture publication and optional browser checks.
  LAN serving exposes the entire output tree without authentication; environment
  overrides do not reconfigure a running server.
- [Maintained scripts](../scripts/README.md) and
  [fixture ownership](../fixtures/README.md) separate checks, flight capture,
  replay, publication, current inputs and retired metadata. The current planner
  still depends on two frozen files under `fixtures/research/`; directory and
  version names are not deletion criteria.
- The documentation checker ignores literal inline code and comments, handles
  deleted tracked files while still rejecting links to them, and keeps diagnostic
  line numbers. Seven focused tests cover those boundaries. It remains a small
  inline-link/ATX-anchor check, not a full Markdown parser or external crawler.
- Agent guidance and ignore rules define safe evidence boundaries. Scratch and
  Python caches are ignored, not deleted. Dated research plans remain dated;
  their commands are not restored as executable entrypoints.

## Validation

The maintained 11-step developer gate passes:

- 447 workspace Rust tests pass; four explicit local-evidence tests remain ignored.
- Feature-enabled/default-off CLI tests, help and dependency boundaries pass.
- Rust formatting, diff whitespace and strict all-target/all-feature Clippy pass.
- All 51 maintained JavaScript tests pass, including the seven documentation tests.
- Local documentation links and anchors pass. Generated output links are deliberately
  optional, so this is not a check of missing local captures or external URLs.

The final check covers 99 Markdown files and 542 local links, with 34 generated
evidence links skipped and no issues. All 99 pages are reachable from the root.

A separate temporary Git-initialized copy of the repository files, with no
`outputs/` or build products, passes all 51 Node tests and the docs check. It was
removed after validation. Final links are checked again after adding this record.

Additional read-only checks establish:

- All 34 original root README command lines remain present in the relocated
  workflow material. The controller checkpoint tail and project-direction text
  remain intact; refresh descriptions were intentionally corrected, not removed.
- All five restored archival source blobs exist at their pinned Git revisions.
  These are verified pre-retirement references, not relabeled measured provenance.
- Both SHA-256-pinned inputs in the current policy-3 pack remain exact.
- The selected site's saved-evidence workflow check passes: 44 cases, 42 preserved
  rich payloads, 36 actual handoffs and 46 receipt-bound pages. It performs no new
  physical replay and does not regenerate the site.
- All 31 protected evidence/input scopes and 3,867 entries remain unchanged.
- The tracked-file inventory contains no generated `outputs/`, `target/`,
  debug scratch or Python bytecode. A limited common private-key/GitHub-token/AWS-key
  pattern scan found no matches; this is not a comprehensive secrets audit.

No 44-case numerical parity run was added: flight implementation is unchanged,
the ordinary tracked tests pass, and this is a docs/tooling pass.

## Deliberately left alone

- The existing detached `powered-descent-lab-terminal-analysis` worktree has an
  uncommitted terminal-controller experiment. It was inspected read-only, not removed.
- Two missing temporary worktrees, `/tmp/pd_bde_probe` and
  `/tmp/tmp.X4QVh9d2oH`, have stale Git registrations. A prune dry-run confirms
  the exact targets; actual pruning was not performed.
- No license choice, external CI service, global RTK hook, dependency/toolchain
  upgrade or broad ignored-file cleanup is inferred from repository hygiene.
- Frozen research fixtures, retired pack metadata and dated result records are
  deliberately retained. Restoring retired executables or deleting history is not
  a prerequisite for current planner use.

Next is review and commit of this docs/tooling checkpoint, if requested. Further
housekeeping should follow a concrete maintenance finding; substantive planner
work remains a separate choice between bounded procedural-terrain evaluation and
a concrete host-consumer requirement.

## Review before commit (2026-10-06)

The user subsequently authorized review, split commits and push. Review found
one bounded checker defect: a literal comment opener inside inline code could
consume later real links, while an inline-code HTML ID could appear to be a real
anchor. A failing regression reproduced it; source-ordered code/comment masking
and separate visible-HTML anchor extraction fix it. All eight focused tests pass.
Staged whitespace checking also caught and removed trailing blank lines in the
two new directory guides, which the earlier unstaged check could not inspect.
No blocking review finding remains within the documented checker scope.

The final 11-step developer gate passes 447 Rust tests (four ignored) and all
52 Node tests, plus CLI/dependency boundaries, formatting and strict Clippy.
The first staged commit independently passes documentation checks and all 52
Node tests in a temporary output-free repository copy. The final docs commit is
checked the same way before commit; temporary validation copies are removed.

The changes are split by responsibility: `df6de82` adds the link gate/tests and
its prerequisite archive-link/anchor repairs; the second commit reorganizes
workflows and adds repository, fixture and tooling guidance. Archived banners
use the preserved root README anchor, so the first commit does not depend on
the later documentation home. Detailed material and historical measurements
remain intact.

The earlier preparation-only commit/push statements are not the current handoff.
Publication verification here means the requested Git push, not report
regeneration: capture/site evidence, frozen inputs, flight source and the
separate dirty analysis worktree remain unchanged. The final push and upstream
parity are verified after both commits; no server or report operation is implied.
