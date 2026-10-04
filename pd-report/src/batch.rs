//! Shared templates and presentation components for terminal, transfer, and
//! native V2 batch reports. Domain aggregation stays with each caller; HTML
//! fragments passed here are trusted, already-rendered presentation fragments.

use crate::batch_tree;

/// Browser-side tree initialization for one batch page.
pub struct BatchTreeOptions<'a> {
    pub max_depth: usize,
    pub depth_by_kind: &'a [(&'a str, usize)],
    pub leaf_selector: &'a str,
    pub default_expansion: usize,
}

/// Presentation-only inputs to the common full batch-page template.
pub struct BatchPage<'a> {
    /// Plain text; escaped by the shared renderer.
    pub title: &'a str,
    /// Plain text; escaped by the shared renderer.
    pub subtitle: &'a str,
    /// Trusted HTML fragments, normally `chip` elements.
    pub chips_html: &'a str,
    /// Trusted HTML fragments, normally report artifact links.
    pub actions_html: &'a str,
    /// Optional trusted HTML inserted before the common hero (for example nav).
    pub before_hero_html: &'a str,
    /// Optional trusted HTML inserted between the common hero and sections.
    pub after_hero_html: &'a str,
    pub overview_html: &'a str,
    pub planner_html: &'a str,
    pub coverage_html: &'a str,
    pub context_html: &'a str,
    pub diagnostics_html: &'a str,
    pub review_tree_html: &'a str,
    pub comparison_html: &'a str,
    /// Optional trusted HTML appended after comparison (for provenance panels).
    pub appendix_html: &'a str,
    /// Optional class added to `<body>`; legacy reports use the empty string.
    pub body_class: &'a str,
    pub tree: BatchTreeOptions<'a>,
    /// Enable the shared coverage-cell-to-review-tree interaction.
    pub coverage_tree_jump: bool,
}

/// The complete common batch page, including the established report shell,
/// full shared styles, the shared tree engine, and configured initialization.
pub fn render_batch_page(page: BatchPage<'_>) -> String {
    let BatchPage {
        title,
        subtitle,
        chips_html,
        actions_html,
        before_hero_html,
        after_hero_html,
        overview_html,
        planner_html,
        coverage_html,
        context_html,
        diagnostics_html,
        review_tree_html,
        comparison_html,
        appendix_html,
        body_class,
        tree,
        coverage_tree_jump,
    } = page;
    let script = render_page_script(&tree, coverage_tree_jump);
    let style = style();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>{style}</style>
</head>
<body class="{body_class}">
  <div id="report-page" class="page" data-batch-template="common-v1">
    {before_hero_html}
    <header class="hero">
      <div>
        <h1>{title}</h1>
        <p class="subtitle">{subtitle}</p>
        <div class="chip-row">{chips_html}</div>
      </div>
      <div class="hero-actions">{actions_html}</div>
    </header>
    {after_hero_html}
    {overview_html}
    {planner_html}
    {coverage_html}
    {context_html}
    {diagnostics_html}
    {review_tree_html}
    <div class="compare-sections">{comparison_html}</div>
    {appendix_html}
  </div>
  <script>{script}</script>
</body>
</html>"#,
        title = escape_html(title),
        subtitle = escape_html(subtitle),
        body_class = escape_html(body_class),
        style = style,
        before_hero_html = before_hero_html,
        chips_html = chips_html,
        actions_html = actions_html,
        after_hero_html = after_hero_html,
        overview_html = overview_html,
        planner_html = planner_html,
        coverage_html = coverage_html,
        context_html = context_html,
        diagnostics_html = diagnostics_html,
        review_tree_html = review_tree_html,
        comparison_html = comparison_html,
        appendix_html = appendix_html,
        script = script,
    )
}

/// Status-only detail page using the same report shell and palette. It has no
/// tree initialization or trajectory by construction.
pub fn render_status_page(title: &str, navigation_html: &str, body_html: &str) -> String {
    let title = escape_html(title);
    let style = style();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>{style}</style>
</head>
<body>
  <div id="report-page" class="page" data-batch-template="common-v1">
    {navigation_html}
    <header class="hero"><div><h1>{title}</h1></div></header>
    <section class="panel status-page">{body_html}</section>
  </div>
</body>
</html>"#,
        title = title,
        style = style,
        navigation_html = navigation_html,
        body_html = body_html,
    )
}

/// Shared report table wrapper. Headers, attributes, and rows are trusted HTML.
pub fn render_table(
    class: &str,
    attributes_html: &str,
    headers_html: &str,
    rows_html: &str,
) -> String {
    format!(
        r#"<table class="{}" {}><thead>{}</thead><tbody>{}</tbody></table>"#,
        escape_html(class),
        attributes_html,
        headers_html,
        rows_html
    )
}

/// Shared report row wrapper. `attributes_html` and `cells_html` are trusted.
pub fn render_row(class: Option<&str>, attributes_html: &str, cells_html: &str) -> String {
    let class_attr = class.map_or_else(String::new, |class| {
        format!(r#" class="{}""#, escape_html(class))
    });
    let attributes = if attributes_html.is_empty() {
        String::new()
    } else {
        format!(" {attributes_html}")
    };
    format!("<tr{class_attr}{attributes}>{cells_html}</tr>")
}

/// Common seven-column legacy overview row, extensible with caller-provided
/// columns (including a native V2 handoff column).
pub fn render_overview_row(class: &str, cells_html: &[&str]) -> String {
    let cells = cells_html
        .iter()
        .map(|content| format!("<td>{content}</td>"))
        .collect::<String>();
    render_row(Some(class), "", &cells)
}

/// Common legacy context row. Every entry is rendered as one trusted `<td>`.
pub fn render_context_row(cells_html: &[&str]) -> String {
    let cells = cells_html
        .iter()
        .map(|content| format!("<td>{content}</td>"))
        .collect::<String>();
    render_row(None, "", &cells)
}

pub fn render_overview_table(headers_html: &str, rows_html: &str) -> String {
    render_table("summary-table", "", headers_html, rows_html)
}

pub fn render_context_table(headers_html: &str, rows_html: &str) -> String {
    render_table("context-table", "", headers_html, rows_html)
}

pub fn render_overview_section(view_controls_html: &str, table_html: &str) -> String {
    format!(
        r#"<section class="header-overview"><div class="section-head"><h2>Overview</h2>{view_controls_html}</div><div class="table-wrap">{table_html}</div></section>"#,
        view_controls_html = view_controls_html,
        table_html = table_html,
    )
}

pub fn render_context_section(
    attention: bool,
    open: bool,
    status_class: &str,
    status_label: &str,
    table_html: &str,
) -> String {
    let attention_class = if attention { " attention" } else { "" };
    let open_attr = if open { " open" } else { "" };
    format!(
        r#"<details class="header-context{attention_class}"{open_attr}><summary><h2>Context</h2><span class="status-chip {status_class}">{status_label}</span></summary><div class="table-wrap">{table_html}</div></details>"#,
        attention_class = attention_class,
        open_attr = open_attr,
        status_class = escape_html(status_class),
        status_label = escape_html(status_label),
        table_html = table_html,
    )
}

pub fn render_coverage_section(axis_note: &str, filters_html: &str, panes_html: &str) -> String {
    format!(
        r#"<section class="coverage-section"><div class="section-head"><div><h2>Coverage</h2><span class="section-note">{}; click a cell to inspect its review-tree branch.</span></div>{filters_html}</div>{panes_html}</section>"#,
        escape_html(axis_note),
        filters_html = filters_html,
        panes_html = panes_html,
    )
}

pub fn render_coverage_pane(
    mission: &str,
    condition: &str,
    vehicle: &str,
    profile: &str,
    table_html: &str,
) -> String {
    format!(
        r#"<div class="coverage-pane table-wrap" data-coverage-pane data-mission="{}" data-condition="{}" data-vehicle="{}" data-profile="{}">{table_html}</div>"#,
        escape_html(mission),
        escape_html(condition),
        escape_html(vehicle),
        escape_html(profile),
        table_html = table_html,
    )
}

pub fn render_coverage_table(corner: &str, headers_html: &str, rows_html: &str) -> String {
    format!(
        r#"<table class="coverage-table"><thead><tr><th>{}</th>{headers_html}</tr></thead><tbody>{rows_html}</tbody></table>"#,
        escape_html(corner),
        headers_html = headers_html,
        rows_html = rows_html,
    )
}

pub fn render_coverage_row(label: &str, cells_html: &str) -> String {
    format!("<tr><th>{}</th>{cells_html}</tr>", escape_html(label))
}

pub fn render_coverage_cell(class: &str, tree_tokens: Option<&str>, contents_html: &str) -> String {
    render_coverage_target_cell(class, "data-tree-tokens", tree_tokens, contents_html)
}

pub fn render_coverage_group_cell(class: &str, group_id: &str, contents_html: &str) -> String {
    render_coverage_target_cell(class, "data-tree-group", Some(group_id), contents_html)
}

fn render_coverage_target_cell(
    class: &str,
    target_attribute: &str,
    target: Option<&str>,
    contents_html: &str,
) -> String {
    let class = if class.trim().is_empty() {
        "coverage-cell".to_owned()
    } else {
        format!("coverage-cell {}", escape_html(class.trim()))
    };
    let target_attribute = target.map_or_else(String::new, |value| {
        format!(
            r#" {target_attribute}="{}" tabindex="0""#,
            escape_html(value),
            target_attribute = target_attribute,
        )
    });
    format!(
        r#"<td><div class="{class}"{target_attribute}>{contents_html}</div></td>"#,
        class = class,
        target_attribute = target_attribute,
        contents_html = contents_html,
    )
}

pub fn render_review_tree_section(controls_html: &str, note_html: &str, tree_html: &str) -> String {
    format!(
        r#"<section class="review-tree-section"><div class="section-head"><h2>Review Tree</h2>{controls_html}</div>{note_html}<div id="review-tree-root" class="review-tree-root">{tree_html}</div></section>"#,
        controls_html = controls_html,
        note_html = note_html,
        tree_html = tree_html,
    )
}

pub fn render_tree_table_section(
    heading_html: &str,
    metadata_html: &str,
    table_html: &str,
) -> String {
    format!(
        r#"<section class="tree-table-section"><div class="table-heading"><h3>{heading_html}</h3><div class="section-meta">{metadata_html}</div></div><div class="table-wrap">{table_html}</div></section>"#,
        heading_html = heading_html,
        metadata_html = metadata_html,
        table_html = table_html,
    )
}

pub fn render_review_tree_table(table_id: &str, headers_html: &str, rows_html: &str) -> String {
    render_table(
        "scenario-table",
        &format!(r#"data-tree-table="{}""#, escape_html(table_id)),
        headers_html,
        rows_html,
    )
}

fn render_page_script(tree: &BatchTreeOptions<'_>, coverage_tree_jump: bool) -> String {
    let depths = tree
        .depth_by_kind
        .iter()
        .map(|(kind, depth)| ((*kind).to_owned(), *depth))
        .collect::<std::collections::BTreeMap<_, _>>();
    let config = serde_json::json!({
        "maxDepth": tree.max_depth,
        "depthByKind": depths,
        "leafSelector": tree.leaf_selector,
        "defaultExpansion": tree.default_expansion,
    });
    let config = serde_json::to_string(&config)
        .expect("tree configuration is serializable")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    let coverage = if coverage_tree_jump {
        COVERAGE_TREE_SCRIPT
    } else {
        ""
    };
    format!(
        r#"{tree_script}
(() => {{
  const page = document.getElementById("report-page");
  const root = document.getElementById("review-tree-root");
  if (!root || !page || !window.PdBatchTree) return;
  const tree = window.PdBatchTree.init(root, {config});
  if (!tree) return;
  {coverage}
}})();"#,
        tree_script = batch_tree::SCRIPT,
        config = config,
        coverage = coverage,
    )
}

const COVERAGE_TREE_SCRIPT: &str = r#"
  const selectorToken = (value) => {
    let output = "";
    let lastDash = false;
    for (const raw of value.toLowerCase()) {
      if (/[a-z0-9]/.test(raw)) { output += raw; lastDash = false; }
      else if (raw === "+") { output += "plus"; lastDash = false; }
      else if (raw === "-") { output += "minus"; lastDash = false; }
      else if (!lastDash) { output += "-"; lastDash = true; }
    }
    return output.replace(/^-+|-+$/g, "") || "x";
  };
  const coverageFilters = Array.from(document.querySelectorAll("[data-coverage-filter]"));
  const updateCoverage = () => {
    document.querySelectorAll("[data-coverage-pane]").forEach((pane) => {
      pane.hidden = coverageFilters.some((filter) =>
        pane.dataset[filter.dataset.coverageFilter] !== filter.value
      );
    });
  };
  coverageFilters.forEach((filter) => filter.addEventListener("change", updateCoverage));
  updateCoverage();
  document.querySelectorAll("[data-tree-tokens]").forEach((cell) => {
    const inspectCoverageCell = () => {
      const tokens = cell.dataset.treeTokens.split("|").filter(Boolean).map(selectorToken);
      tree.expandGroups();
      let target = null;
      tree.tables.forEach((table) => {
        tree.summaryRows(table).forEach((row) => {
          const parts = (row.dataset.group || "").split("--");
          if (tokens.every((token) => parts.includes(token)) &&
              (!target || (row.dataset.group || "").length > (target.dataset.group || "").length)) {
            target = row;
          }
        });
      });
      if (target) {
        target.scrollIntoView({behavior: "smooth", block: "center"});
      target.classList.add("coverage-target");
        window.setTimeout(() => target.classList.remove("coverage-target"), 1400);
      }
    };
    cell.addEventListener("click", inspectCoverageCell);
    cell.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      inspectCoverageCell();
    });
  });
  document.querySelectorAll("[data-tree-group]").forEach((cell) => {
    const inspectGroup = () => {
      const group = cell.dataset.treeGroup;
      const target = tree.summaryRows().find((row) => row.dataset.group === group);
      if (!target) return;
      tree.expandPath(group);
      target.scrollIntoView({behavior: "smooth", block: "center"});
        target.classList.add("coverage-target");
      window.setTimeout(() => target.classList.remove("coverage-target"), 1400);
    };
    cell.addEventListener("click", inspectGroup);
    cell.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      inspectGroup();
    });
  });
"#;

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Shared styles from the terminal/transfer batch template.
pub fn style() -> String {
    format!("{}{}", batch_tree::STYLE, PAGE_STYLE)
}

const PAGE_STYLE: &str = include_str!("batch.css");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_batch_page_owns_legacy_shell_and_safe_tree_initialization() {
        let html = render_batch_page(BatchPage {
            title: "A <report>",
            subtitle: "A & B",
            chips_html: "<span class=\"chip\">runs</span>",
            actions_html: "<a href=\"summary.json\">summary</a>",
            before_hero_html: "<nav>Home</nav>",
            after_hero_html: "<p>Preview</p>",
            overview_html: "<section class=\"header-overview\">overview</section>",
            planner_html: "",
            coverage_html: "",
            context_html: "",
            diagnostics_html: "",
            review_tree_html: "<section class=\"review-tree-section\"><div id=\"review-tree-root\"></div></section>",
            comparison_html: "",
            appendix_html: "",
            body_class: "",
            tree: BatchTreeOptions {
                max_depth: 1,
                depth_by_kind: &[("group", 0), ("family", 1)],
                leaf_selector: "tr.mission-row",
                default_expansion: 1,
            },
            coverage_tree_jump: true,
        });
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("<title>A &lt;report&gt;</title>"));
        assert!(html.contains("<p class=\"subtitle\">A &amp; B</p>"));
        assert!(html.contains("--accent: #b95024"));
        assert!(html.contains("window.PdBatchTree ="));
        assert!(html.contains("if (!root || !page || !window.PdBatchTree) return;"));
        assert!(html.contains("leafSelector\":\"tr.mission-row\""));
        assert!(html.contains("tree.expandPath(group)"));
        assert!(html.contains("[data-tree-group]"));
        assert!(
            html.contains(".page code {\n  overflow-wrap: anywhere;\n  word-break: break-all;\n}")
        );
        assert!(html.contains(".scenario-table:has(th[data-optional=\"handoffs\"])"));
        assert!(html.find("<nav>").unwrap() < html.find("<header class=\"hero\">").unwrap());
        assert!(html.find("</header>").unwrap() < html.find("<p>Preview</p>").unwrap());
    }

    #[test]
    fn shared_rows_tables_coverage_targets_and_status_pages_keep_common_classes() {
        let overview = render_overview_table("<tr><th>one</th><th>two</th></tr>", "<tr></tr>");
        assert!(overview.contains("class=\"summary-table\""));
        assert!(
            render_overview_row("current-summary-row", &["one", "two"])
                .contains("<tr class=\"current-summary-row\"><td>one</td><td>two</td></tr>")
        );
        assert!(render_context_row(&["context"]).contains("<tr><td>context</td></tr>"));
        let tree = render_review_tree_table("v2", "<tr><th>Mission</th></tr>", "<tr></tr>");
        assert!(tree.contains("class=\"scenario-table\" data-tree-table=\"v2\""));
        let group_cell =
            render_coverage_group_cell("has-failure", "v2-ordinary-ridge", "<strong>2</strong>");
        assert!(group_cell.contains("data-tree-group=\"v2-ordinary-ridge\""));
        assert!(group_cell.contains("tabindex=\"0\""));
        let html = render_status_page(
            "Unsupported case",
            "<nav>Reports</nav>",
            "<p>Not simulated</p>",
        );
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("class=\"panel status-page\""));
        assert!(!html.contains("id=\"review-tree-root\""));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("polyline"));
    }
}
