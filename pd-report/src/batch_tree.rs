//! Shared presentation and interaction primitives for nested batch reports.

/// Warm terminal and transfer report shell, table, preview, and tree styles.
///
/// The legacy batch report includes this string directly. V2 batch reports can
/// use the same styles without carrying a second copy of the presentation.
pub const STYLE: &str = r#"
:root {
  color-scheme: light;
  --bg: #f1ede5;
  --surface: #fffdf8;
  --surface-strong: #f8f2e8;
  --ink: #20211e;
  --muted: #6d665c;
  --line: #d9cdbc;
  --accent: #b95024;
  --accent-soft: #f4ded1;
  --good: #176b5c;
  --bad: #a43a2c;
  --warn: #966515;
  --display: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, Georgia, serif;
  --mono: "Iosevka Term", "SFMono-Regular", Consolas, monospace;
  --sans: "Avenir Next", "IBM Plex Sans", "Trebuchet MS", sans-serif;
  --shadow: 0 18px 44px rgba(45, 34, 23, 0.08);
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background:
    linear-gradient(rgba(69, 58, 44, 0.025) 1px, transparent 1px),
    linear-gradient(90deg, rgba(69, 58, 44, 0.025) 1px, transparent 1px),
    radial-gradient(circle at 12% 0%, rgba(185,80,36,0.12), transparent 31rem),
    linear-gradient(180deg, #faf7f0 0%, var(--bg) 100%);
  background-size: 32px 32px, 32px 32px, auto, auto;
  background-attachment: fixed;
  color: var(--ink);
  font-family: var(--sans);
  line-height: 1.45;
}
.page {
  max-width: 1500px;
  margin: 0 auto;
  padding: 20px 20px 52px;
}
.hero {
  position: relative;
  overflow: hidden;
  display: flex;
  justify-content: space-between;
  gap: 18px;
  align-items: flex-start;
  margin-bottom: 20px;
  padding: 20px 22px 22px;
  border: 1px solid var(--line);
  border-radius: 22px;
  background:
    radial-gradient(circle at 90% 20%, rgba(185,80,36,0.09), transparent 18rem),
    rgba(255,253,248,0.94);
  box-shadow: var(--shadow);
}
.hero::before {
  content: "";
  position: absolute;
  inset: 0 0 auto;
  height: 4px;
  background: linear-gradient(90deg, var(--accent) 0 42%, var(--good) 42% 71%, #3568a8 71%);
}
.hero::after {
  content: "";
  position: absolute;
  width: 250px;
  height: 250px;
  right: -92px;
  top: -154px;
  border: 1px solid rgba(185,80,36,0.2);
  border-radius: 50%;
  pointer-events: none;
}
.hero > * { position: relative; z-index: 1; }
.hero h1 {
  margin: 0 0 6px;
  max-width: 34ch;
  font-family: var(--display);
  font-size: clamp(1.9rem, 3.2vw, 2.7rem);
  font-weight: 500;
  letter-spacing: -0.035em;
  line-height: 0.98;
  overflow-wrap: anywhere;
}
.subtitle {
  margin: 0;
  color: var(--muted);
  max-width: 68ch;
  font-size: 0.94rem;
}
.chip-row { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 10px; }
.chip {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  border-radius: 999px;
  border: 1px solid var(--line);
  background: rgba(255,255,255,0.75);
  padding: 5px 9px;
  font-size: 0.8rem;
  color: var(--muted);
  max-width: 100%;
  overflow-wrap: anywhere;
}
.chip strong { color: var(--ink); font-weight: 700; flex: none; white-space: nowrap; }
.chip .mono { min-width: 0; overflow-wrap: anywhere; word-break: break-all; }
.hero-actions { display: flex; flex-wrap: wrap; gap: 8px; justify-content: flex-end; }
.hero-actions a {
  text-decoration: none;
  color: var(--ink);
  border: 1px solid var(--line);
  background: var(--surface);
  padding: 7px 12px;
  border-radius: 999px;
  font-size: 0.84rem;
  white-space: nowrap;
  box-shadow: 0 3px 10px rgba(45,34,23,0.04);
}
.hero-actions a:hover {
  border-color: var(--accent);
  color: var(--accent);
  text-decoration: none;
  transform: translateY(-1px);
}
.section-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 10px 14px;
  margin-bottom: 10px;
}
.section-head h2, .table-heading h3 {
  margin: 0;
  font-family: var(--display);
  font-size: 1.28rem;
  font-weight: 600;
  letter-spacing: -0.015em;
  color: var(--ink);
}
.overview-stack { display: grid; gap: 4px; }
.overview-main { font-weight: 700; color: var(--ink); font-variant-numeric: tabular-nums; }
.overview-sub { color: var(--muted); font-size: 0.84rem; font-variant-numeric: tabular-nums; }
.card {
  background: rgba(255,253,248,0.92);
  border: 1px solid var(--line);
  border-radius: 18px;
  padding: 14px 15px;
  box-shadow: 0 12px 32px rgba(39,28,18,0.055);
  min-width: 0;
}
.card h2 {
  margin: 0 0 10px;
  font-size: 0.9rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.metric {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: baseline;
  margin-top: 7px;
  font-size: 0.92rem;
}
.metric strong { font-size: 1.2rem; font-variant-numeric: tabular-nums; }
.metric .good { color: var(--good); }
.metric .bad { color: var(--bad); }
.metric .warn { color: var(--warn); }
.layout {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
  gap: 12px;
  margin-bottom: 12px;
}
.panel {
  background: rgba(255,253,248,0.94);
  border: 1px solid var(--line);
  border-radius: 18px;
  padding: 14px 15px;
  box-shadow: 0 12px 32px rgba(39,28,18,0.05);
  min-width: 0;
}
.panel h2 { margin: 0 0 8px; font-size: 1rem; }
.panel p { margin: 0 0 10px; color: var(--muted); font-size: 0.9rem; }
.table-wrap { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 0.88rem; font-variant-numeric: tabular-nums; }
th, td {
  text-align: left;
  padding: 8px 10px;
  border-bottom: 1px solid rgba(217,205,188,0.72);
  vertical-align: top;
}
th {
  color: var(--muted);
  font-weight: 700;
  font-size: 0.76rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  white-space: nowrap;
}
td code {
  font-family: var(--mono);
  font-size: 0.82rem;
  background: rgba(248,243,234,0.9);
  padding: 1px 5px;
  border-radius: 6px;
}
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
.muted { color: var(--muted); }
.mono { font-family: var(--mono); }
.good { color: var(--good); }
.bad { color: var(--bad); }
.warn { color: var(--warn); }
.tree-stack { display: grid; gap: 14px; }
.tree-table-section {
  border: 1px solid var(--line);
  border-radius: 16px;
  background: rgba(255,253,248,0.86);
  box-shadow: 0 10px 28px rgba(39,28,18,0.045);
  padding: 12px 12px 10px;
  min-width: 0;
  max-width: 100%;
}
.table-heading {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 8px 12px;
  align-items: baseline;
  margin-bottom: 10px;
}
.table-heading h3 { font-size: 0.98rem; }
.table-heading h3 code { font-size: 0.86rem; }
.table-heading .section-meta { color: var(--muted); font-size: 0.84rem; }
.tree-controls, .view-mode-controls { display: flex; flex-wrap: wrap; gap: 8px; }
.tree-controls button, .view-mode-controls button {
  border: 1px solid var(--line);
  background: rgba(255,253,248,0.92);
  color: var(--ink);
  border-radius: 999px;
  padding: 7px 11px;
  font: inherit;
  font-size: 0.84rem;
  cursor: pointer;
}
.tree-controls button:hover, .view-mode-controls button:hover {
  border-color: var(--accent);
  color: var(--accent);
  transform: translateY(-1px);
}
.view-mode-controls button.active {
  border-color: rgba(23, 107, 92, 0.34);
  background: rgba(23, 107, 92, 0.1);
  color: var(--good);
}
.scenario-table { width: 100%; min-width: 980px; }
.scenario-table thead th { background: rgba(248,243,234,0.96); position: sticky; top: 0; z-index: 1; }
.scenario-table th:first-child, .scenario-table td:first-child {
  position: sticky;
  left: 0;
  z-index: 2;
  background: #fffdf8;
  box-shadow: 1px 0 0 rgba(217,205,188,0.82);
}
.scenario-table thead th:first-child { z-index: 3; background: #f8f3ea; }
.scenario-table .current-row td:first-child { background: #f0f7f2; }
.scenario-table .baseline-row td:first-child { background: #faf3ed; }
.summary-row { cursor: pointer; }
.summary-row.current-row { box-shadow: inset 4px 0 0 rgba(14, 107, 96, 0.72); }
.summary-row.baseline-row { box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.72); }
.summary-row.lane-controller-current { box-shadow: inset 4px 0 0 rgba(47, 125, 74, 0.78); }
.summary-row.lane-controller-baseline { box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.78); }
.scenario-row, .scenario-row > td { background: rgba(23, 107, 92, 0.045); }
.scenario-row:hover, .scenario-row:hover > td { background: rgba(23, 107, 92, 0.09); }
.baseline-scenario-row, .baseline-scenario-row > td { background: rgba(185, 80, 36, 0.045); }
.baseline-scenario-row:hover, .baseline-scenario-row:hover > td { background: rgba(185, 80, 36, 0.09); }
.summary-row.lane-controller-current, .summary-row.lane-controller-current > td { background: rgba(23, 107, 92, 0.055); }
.summary-row.lane-controller-current:hover, .summary-row.lane-controller-current:hover > td { background: rgba(23, 107, 92, 0.1); }
.summary-row.lane-controller-baseline, .summary-row.lane-controller-baseline > td { background: rgba(185, 80, 36, 0.055); }
.summary-row.lane-controller-baseline:hover, .summary-row.lane-controller-baseline:hover > td { background: rgba(185, 80, 36, 0.1); }
.summary-row td:first-child { font-weight: 700; }
.seed-row.current-row, .mission-row.current-row { box-shadow: inset 4px 0 0 rgba(14, 107, 96, 0.44); }
.seed-row.baseline-row, .mission-row.baseline-row { box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.44); }
.seed-row.lane-controller-current { box-shadow: inset 4px 0 0 rgba(47, 125, 74, 0.46); }
.seed-row.lane-controller-baseline { box-shadow: inset 4px 0 0 rgba(163, 101, 54, 0.46); }
.seed-row, .seed-row > td, .mission-row, .mission-row > td { background: rgba(255, 253, 248, 0.86); }
.seed-row:hover, .seed-row:hover > td, .mission-row:hover, .mission-row:hover > td { background: rgba(248, 242, 232, 0.92); }
.baseline-seed-row, .baseline-seed-row > td, .baseline-mission-row, .baseline-mission-row > td { background: rgba(250, 246, 239, 0.86); }
.baseline-seed-row:hover, .baseline-seed-row:hover > td, .baseline-mission-row:hover, .baseline-mission-row:hover > td { background: rgba(246, 237, 226, 0.94); }
.seed-row.lane-controller-current, .seed-row.lane-controller-current > td { background: rgba(248, 252, 249, 0.88); }
.seed-row.lane-controller-current:hover, .seed-row.lane-controller-current:hover > td { background: rgba(238, 247, 243, 0.95); }
.seed-row.lane-controller-baseline, .seed-row.lane-controller-baseline > td { background: rgba(252, 247, 241, 0.9); }
.seed-row.lane-controller-baseline:hover, .seed-row.lane-controller-baseline:hover > td { background: rgba(247, 238, 228, 0.96); }
.summary-row.current-row.changed { box-shadow: inset 7px 0 0 rgba(181,93,45,0.72); }
.summary-row.baseline-row.changed { box-shadow: inset 7px 0 0 rgba(181,93,45,0.5); }
.tree-label { padding-left: calc(10px + var(--depth, 0) * 22px); white-space: nowrap; }
.row-tag {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 4.9rem;
  margin-right: 8px;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 0.68rem;
  font-weight: 700;
  line-height: 1.15;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  border: 1px solid transparent;
}
.row-tag.current { background: rgba(14, 107, 96, 0.16); color: var(--accent); border-color: rgba(14, 107, 96, 0.22); }
.row-tag.baseline { background: rgba(181, 126, 80, 0.14); color: #8a5126; border-color: rgba(181, 126, 80, 0.24); }
.row-tag.diff { background: rgba(199, 160, 84, 0.18); color: #7a5611; border-color: rgba(199, 160, 84, 0.24); }
.row-tag.lane-current { background: rgba(47, 125, 74, 0.16); color: var(--good); border-color: rgba(47, 125, 74, 0.24); }
.row-tag.lane-baseline { background: rgba(181, 93, 45, 0.16); color: #8a5126; border-color: rgba(181, 93, 45, 0.24); }
.outcome-bad { color: var(--bad); font-weight: 700; }
.expander { display: inline-block; width: 1.15rem; color: var(--accent); font-weight: 700; text-align: center; transform-origin: center; }
.expander.muted { color: var(--muted); }
.summary-row[aria-expanded="true"] .expander { transform: rotate(45deg); }
.selector-code { font-family: var(--mono); font-size: 0.82rem; background: rgba(248,243,234,0.9); padding: 1px 5px; border-radius: 6px; }
.selector-inline { font-family: var(--mono); font-size: 0.8rem; color: var(--muted); }
.row-note { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 0.8rem; line-height: 1.3; flex-wrap: wrap; }
.row-note .emph { color: var(--ink); font-weight: 600; }
.row-note .bad { color: var(--bad); font-weight: 700; }
.row-note .good { color: var(--good); font-weight: 700; }
.preview-cell { display: grid; gap: 6px; }
.run-preview {
  display: inline-flex;
  width: 148px;
  max-width: 100%;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: #fbf7ee;
  overflow: hidden;
  box-shadow: 0 3px 10px rgba(39,28,18,0.06);
}
.run-preview img, .run-preview svg { display: block; width: 100%; height: auto; background: #fbf7ee; }
.lane-preview { width: 148px; }
.scenario-table.baseline-hidden .baseline-row { display: none; }
.review-tree-root.diff-only .scenario-table tr.unchanged { display: none; }
.baseline-row { color: var(--muted); }
[hidden] { display: none !important; }
@media (max-width: 1100px) {
  .layout { grid-template-columns: 1fr; }
  .hero { flex-direction: column; }
  .hero-actions { justify-content: flex-start; }
  .scenario-table { min-width: 860px; }
}
@media (max-width: 700px) { .page { padding-inline: 14px; } }
"#;

/// Browser-side tree engine. Each batch renderer initializes it explicitly
/// with its hierarchy depths and leaf selector.
pub const SCRIPT: &str = r#"
window.PdBatchTree = (() => {
  const asDepth = (value, fallback) => {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? Math.max(0, Math.trunc(parsed)) : fallback;
  };

  function init(root, options = {}) {
    if (!root) return null;
    if (root.__pdBatchTreeApi) return root.__pdBatchTreeApi;

    const summarySelector = options.summarySelector || "tr.summary-row";
    const leafSelector = options.leafSelector || "tr.seed-row";
    const tableSelector = options.tableSelector || "table[data-tree-table]";
    const depthByKind = options.depthByKind || {};
    const tables = Array.from(root.querySelectorAll(tableSelector));
    const tableRows = (table) => {
      const selector = `${summarySelector}, ${leafSelector}`;
      return Array.from(table.querySelectorAll(selector));
    };
    const summaryRows = (table) =>
      Array.from((table || root).querySelectorAll(summarySelector));
    const leafRows = (table) =>
      Array.from((table || root).querySelectorAll(leafSelector));
    const rowDepth = (row) => {
      const explicit = row.dataset.depth;
      if (explicit !== undefined && explicit.trim() !== "") {
        const parsed = Number(explicit);
        if (Number.isFinite(parsed)) return Math.max(0, Math.trunc(parsed));
      }
      return asDepth(depthByKind[row.dataset.kind || ""], 0);
    };
    const inferredMaxDepth = summaryRows().reduce(
      (maxDepth, row) => Math.max(maxDepth, rowDepth(row)), 0
    );
    const maxDepth = asDepth(options.maxDepth, inferredMaxDepth);
    const defaultExpansion = Math.min(
      maxDepth,
      asDepth(options.defaultExpansion, maxDepth)
    );
    const controlScope = options.controlScope || root.closest("section") || root.parentElement || root;
    const page = options.page || root.ownerDocument.getElementById("report-page") || root.ownerDocument.body;

    const childRows = (table, group) =>
      tableRows(table).filter((row) => row.dataset.parent === group);
    const hasLeafChildren = (table, group) =>
      leafRows(table).some((row) => row.dataset.parent === group);
    const collapseDescendants = (table, group, visited = new Set()) => {
      if (!group || visited.has(group)) return;
      visited.add(group);
      childRows(table, group).forEach((child) => {
        child.hidden = true;
        if (child.matches(summarySelector) && child.dataset.group) {
          child.setAttribute("aria-expanded", "false");
          collapseDescendants(table, child.dataset.group, visited);
        }
      });
    };
    const showImmediateChildren = (table, row, includeLeaves) => {
      const group = row.dataset.group;
      if (!group) return;
      childRows(table, group).forEach((child) => {
        child.hidden = child.matches(leafSelector) && !includeLeaves;
      });
    };
    const toggleRow = (row) => {
      if (!row.matches(summarySelector) || !row.dataset.group) return;
      const table = row.closest("table");
      if (!table) return;
      const expanded = row.getAttribute("aria-expanded") === "true";
      if (expanded) {
        row.setAttribute("aria-expanded", "false");
        collapseDescendants(table, row.dataset.group);
      } else {
        row.setAttribute("aria-expanded", "true");
        showImmediateChildren(table, row, true);
      }
    };
    const applyExpansionState = (table, targetDepth, showLeaves) => {
      const clampedDepth = Math.min(maxDepth, asDepth(targetDepth, maxDepth));
      tableRows(table).forEach((row) => {
        row.hidden = Boolean(row.dataset.parent);
      });
      summaryRows(table).forEach((row) => {
        if (row.dataset.group) row.setAttribute("aria-expanded", "false");
      });
      table.dataset.targetDepth = String(clampedDepth);
      table.dataset.showLeaves = showLeaves ? "true" : "false";

      let changed = true;
      while (changed) {
        changed = false;
        summaryRows(table).forEach((row) => {
          if (row.hidden || !row.dataset.group) return;
          const hasSummaryChildren = childRows(table, row.dataset.group)
            .some((child) => child.matches(summarySelector));
          const hasVisibleLeafChildren = showLeaves && hasLeafChildren(table, row.dataset.group);
          if ((rowDepth(row) < clampedDepth && hasSummaryChildren) || hasVisibleLeafChildren) {
            if (row.getAttribute("aria-expanded") !== "true") {
              row.setAttribute("aria-expanded", "true");
              showImmediateChildren(table, row, showLeaves);
              changed = true;
            }
          }
        });
      }
    };
    const expandGroups = (depth = maxDepth) => {
      tables.forEach((table) => applyExpansionState(table, depth, false));
    };
    const collapseGroups = (depth = 0) => {
      tables.forEach((table) => applyExpansionState(table, depth, false));
    };
    const expandLeaves = () => {
      tables.forEach((table) => applyExpansionState(table, maxDepth, true));
    };
    const expandPath = (group) => {
      tables.forEach((table) => {
        let row = summaryRows(table).find((candidate) => candidate.dataset.group === group);
        const path = [];
        while (row) {
          path.unshift(row);
          const parent = row.dataset.parent;
          row = parent
            ? summaryRows(table).find((candidate) => candidate.dataset.group === parent)
            : null;
        }
        path.forEach((ancestor) => {
          if (ancestor.getAttribute("aria-expanded") === "true") return;
          ancestor.setAttribute("aria-expanded", "true");
          showImmediateChildren(table, ancestor, true);
        });
      });
    };
    const collapseLeaves = () => {
      tables.forEach((table) => {
        const depth = asDepth(table.dataset.targetDepth, defaultExpansion);
        applyExpansionState(table, depth, false);
      });
    };
    const stepDepth = (table, delta) => {
      const currentDepth = asDepth(table.dataset.targetDepth, defaultExpansion);
      const keepLeaves = table.dataset.showLeaves === "true";
      const nextDepth = Math.max(0, Math.min(maxDepth, currentDepth + delta));
      applyExpansionState(table, nextDepth, keepLeaves && nextDepth === maxDepth);
    };
    const collapseAll = () => collapseGroups(0);

    tables.forEach((table) => {
      applyExpansionState(table, defaultExpansion, false);
      summaryRows(table)
        .filter((row) => row.dataset.group)
        .forEach((row) => {
          row.addEventListener("click", (event) => {
            if (event.target.closest("a, button")) return;
            toggleRow(row);
          });
          row.addEventListener("keydown", (event) => {
            if (event.target.closest("a, button")) return;
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            toggleRow(row);
          });
        });
    });

    controlScope.querySelectorAll("[data-tree-action]").forEach((button) => {
      button.addEventListener("click", () => {
        const action = button.getAttribute("data-tree-action");
        if (action === "expand-depth") {
          tables.forEach((table) => stepDepth(table, 1));
        } else if (action === "collapse-depth") {
          tables.forEach((table) => stepDepth(table, -1));
        } else if (action === "expand-seeds") {
          expandLeaves();
        } else if (action === "collapse-seeds") {
          collapseLeaves();
        } else if (action === "collapse-all") {
          collapseAll();
        } else if (action === "toggle-baseline") {
          tables.forEach((table) => table.classList.toggle("baseline-hidden"));
          const anyHidden = tables.some((table) => table.classList.contains("baseline-hidden"));
          button.textContent = anyHidden ? "Show Baseline" : "Hide Baseline";
        } else if (action === "toggle-diff") {
          root.classList.toggle("diff-only");
          button.textContent = root.classList.contains("diff-only") ? "Show All Groups" : "Show Changed Only";
        }
      });
    });

    const modeButtons = Array.from(root.ownerDocument.querySelectorAll("[data-view-mode]"));
    const setViewMode = (mode) => {
      page.classList.toggle("current-standalone", mode === "current-only");
      modeButtons.forEach((button) => {
        button.classList.toggle("active", button.getAttribute("data-view-mode") === mode);
      });
    };
    modeButtons.forEach((button) => {
      button.addEventListener("click", () => {
        const mode = button.getAttribute("data-view-mode");
        if (mode) setViewMode(mode);
      });
    });
    if (modeButtons.length) setViewMode("compare");

    const api = {
      tables,
      summaryRows,
      maxDepth,
      expandGroups,
      collapseGroups,
      expandPath,
      expandLeaves,
      collapseLeaves,
    };
    root.__pdBatchTreeApi = api;
    return api;
  }

  return { init };
})();
"#;

/// Render the shared hierarchy controls, retaining the legacy action names.
pub fn render_controls(leaf_label: &str, has_compare: bool) -> String {
    let leaf_label = escape_html(leaf_label);
    let mut buttons = vec![
        r#"<button type="button" data-tree-action="expand-depth">Expand</button>"#.to_owned(),
        r#"<button type="button" data-tree-action="collapse-depth">Collapse</button>"#.to_owned(),
        format!(
            r#"<button type="button" data-tree-action="expand-seeds">Expand {leaf_label}</button>"#
        ),
        format!(
            r#"<button type="button" data-tree-action="collapse-seeds">Collapse {leaf_label}</button>"#
        ),
    ];
    if has_compare {
        buttons.push(
            r#"<button type="button" class="compare-only-control" data-tree-action="toggle-baseline">Hide Baseline</button>"#
                .to_owned(),
        );
        buttons.push(
            r#"<button type="button" class="compare-only-control" data-tree-action="toggle-diff">Show Changed Only</button>"#
                .to_owned(),
        );
    }
    format!(r#"<div class="tree-controls">{}</div>"#, buttons.join(""))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_keep_legacy_actions_and_escape_leaf_label() {
        let controls = render_controls("Missions <&>", true);

        assert!(controls.contains("data-tree-action=\"expand-depth\""));
        assert!(controls.contains("data-tree-action=\"collapse-depth\""));
        assert!(
            controls.contains("data-tree-action=\"expand-seeds\">Expand Missions &lt;&amp;&gt;")
        );
        assert!(
            controls
                .contains("data-tree-action=\"collapse-seeds\">Collapse Missions &lt;&amp;&gt;")
        );
        assert!(controls.contains("data-tree-action=\"toggle-baseline\""));
        assert!(controls.contains("data-tree-action=\"toggle-diff\""));
    }

    #[test]
    fn controls_omit_compare_actions_for_standalone_reports() {
        let controls = render_controls("Seeds", false);

        assert!(controls.contains("Expand Seeds"));
        assert!(!controls.contains("toggle-baseline"));
        assert!(!controls.contains("toggle-diff"));
    }
}
