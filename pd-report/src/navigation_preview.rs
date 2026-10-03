//! Read-only navigation preview for the retained waypoint planner V2 capture.
//!
//! These renderers produce HTML strings only. They do not scan the report tree,
//! write stable-site indexes, or change any retained flight report.

use std::{collections::HashSet, fmt::Write as _};

use anyhow::{Context, Result, ensure};

use crate::{
    escape_html,
    waypoint_v2::data::{CaseCard, ReportLink, SuiteReport},
};

const CASE_COUNT: usize = 32;
const GROUP_COUNTS: [(&str, &str, usize); 3] = [
    ("clear", "Clear controls", 8),
    ("ordinary", "Ordinary terrain missions", 16),
    ("diagnostic", "Diagnostics", 8),
];
const ANNOTATED_CASE_ID: &str = "v2_ridge_late";
const FLAT_CASE_ID: &str = "v2_clear_845";
const PLATEAU_CASE_ID: &str = "v2_plateau_reference_900";

/// Destinations used by the navigation-only preview.
///
/// Hrefs are supplied by the caller in their final relative or root-relative
/// form and are preserved verbatim apart from HTML attribute escaping.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewNavigation {
    pub suite: SuiteReport,
    pub annotated_case_id: String,
    pub home_href: String,
    pub suite_href: String,
    pub guidance_href: Option<String>,
    pub batch_library_href: Option<String>,
    pub setups_href: Option<String>,
    pub replays_href: Option<String>,
    pub history: Vec<ReportLink>,
}

/// Render a non-writing preview of the report home and its curated V2 entry.
pub fn render_preview_home(navigation: &PreviewNavigation) -> Result<String> {
    validate_navigation(navigation)?;

    let mut html = page_start("Reports · Navigation preview");
    html.push_str(
        "<main class=\"page\"><p class=\"eyebrow\">Powered Descent Lab · navigation preview</p>",
    );
    html.push_str("<header class=\"hero\"><div><h1>Reports</h1><p>Choose the report that answers your question. The preview keeps retained flight evidence and presentation review separate.</p></div></header>");
    html.push_str("<section class=\"primary-reading\" aria-labelledby=\"primary-heading\"><h2 id=\"primary-heading\">What should I open?</h2><div class=\"primary-grid\">");
    render_primary_entry(
        &mut html,
        Some(&navigation.suite_href),
        "Current waypoint planner V2",
        "Opt-in · current capture · navigation preview",
        "A compact index for the policy 3 capture. Late ridge opens the annotated rich preview; other supported cases retain their original full reports.",
        "Open current V2 preview",
    )?;
    render_primary_entry(
        &mut html,
        navigation.guidance_href.as_deref(),
        "Maintained guidance overview",
        "Maintained evidence",
        "Terminal, direct-transfer, waypoint-guidance and generated-route reports, kept separate from this opt-in capture.",
        "Open guidance overview",
    )?;
    html.push_str("</div></section>");

    html.push_str("<section class=\"secondary-section\" aria-labelledby=\"secondary-heading\"><h2 id=\"secondary-heading\">Other report collections</h2><div class=\"secondary-grid\">");
    render_secondary_entry(
        &mut html,
        navigation.batch_library_href.as_deref(),
        "Batch report library",
        "Browse maintained batch reports.",
    )?;
    render_secondary_entry(
        &mut html,
        navigation.setups_href.as_deref(),
        "Analytical setups",
        "Inspect deterministic setup-only planning evidence.",
    )?;
    render_secondary_entry(
        &mut html,
        navigation.replays_href.as_deref(),
        "Replays",
        "Open deterministic replay reports.",
    )?;
    html.push_str("<section class=\"secondary-card history-card\"><p class=\"eyebrow\">Secondary collection</p><h3>Research and history</h3>");
    if navigation.history.is_empty() {
        html.push_str("<p class=\"unavailable-copy\"><strong>Unavailable.</strong> No history destinations were supplied to this preview.</p>");
    } else {
        html.push_str("<p>Older captures and presentation editions supplied for this preview.</p><ul class=\"history-list\">");
        for link in &navigation.history {
            write!(
                html,
                "<li><a href=\"{}\">{}</a></li>",
                safe_href(&link.href)?,
                escape_html(&link.label)
            )
            .expect("String writes cannot fail");
        }
        html.push_str("</ul>");
    }
    html.push_str("</section></div></section></main></body></html>");
    Ok(html)
}

/// Render the compact, searchable suite navigation preview.
pub fn render_preview_suite(navigation: &PreviewNavigation) -> Result<String> {
    validate_navigation(navigation)?;
    let suite = &navigation.suite;
    let title = escape_html(&suite.title);

    let mut html = page_start(&format!("{} · Navigation preview", suite.title));
    html.push_str("<main class=\"page\"><nav class=\"breadcrumbs\" aria-label=\"Breadcrumb\">");
    write!(
        html,
        "<a href=\"{}\">Reports home</a><span aria-hidden=\"true\">›</span><span aria-current=\"page\">Current V2 preview</span></nav>",
        safe_href(&navigation.home_href)?
    )
    .expect("String writes cannot fail");
    html.push_str(
        "<p class=\"eyebrow\">Waypoint planner V2 · current opt-in capture · preview</p>",
    );
    write!(html, "<header class=\"hero\"><div><h1>{title}</h1><p class=\"intro\">Navigation and one annotated rich report preview. This page does not replace the detailed mission reports.</p></div><span class=\"preview-badge\">Presentation preview</span></header>")
        .expect("String writes cannot fail");

    html.push_str("<section class=\"capture-panel\" aria-labelledby=\"capture-heading\"><div><p class=\"eyebrow\">Retained flight capture</p><h2 id=\"capture-heading\">Capture identity</h2>");
    write!(
        html,
        "<p class=\"capture-label\">{}</p>",
        escape_html(&suite.capture_label)
    )
    .expect("String writes cannot fail");
    html.push_str("<p class=\"capture-note\">Flight outcomes and source reports belong to this retained capture; the annotation is a separate presentation preview.</p></div><dl class=\"capture-facts\"><div><dt>Planner</dt><dd>Policy 3 · opt-in</dd></div><div><dt>Vehicle</dt><dd>Current tested vehicle</dd></div><div><dt>Environment</dt><dd>Earth gravity</dd></div><div><dt>Rates</dt><dd>120 Hz physics · 60 Hz controller</dd></div></dl></section>");

    html.push_str("<section class=\"recommendations\" aria-labelledby=\"recommendations-heading\"><div class=\"section-heading\"><div><h2 id=\"recommendations-heading\">Three useful starting points</h2><p>Open one direct control, one annotated correction and one repeated-correction reference.</p></div></div><div class=\"recommendation-grid\">");
    render_recommendation(
        &mut html,
        find_case(suite, ANNOTATED_CASE_ID).expect("validated annotated case"),
        "Late ridge",
        "One local correction, shown in the annotated rich preview.",
        true,
    )?;
    render_recommendation(
        &mut html,
        find_case(suite, FLAT_CASE_ID).expect("validated flat control"),
        "Flat 845 direct control",
        "The uncut direct-flight control in its original full report.",
        false,
    )?;
    render_recommendation(
        &mut html,
        find_case(suite, PLATEAU_CASE_ID).expect("validated plateau reference"),
        "Reference plateau",
        "Three recorded corrections in the original full report.",
        false,
    )?;
    html.push_str("</div></section>");

    render_group_totals(&mut html, suite);
    render_case_list(&mut html, navigation)?;
    render_suite_sources(&mut html, suite)?;
    html.push_str("</main><script>");
    html.push_str(FILTER_JS);
    html.push_str("</script></body></html>");
    Ok(html)
}

fn page_start(title: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>{PREVIEW_CSS}</style></head><body>",
        escape_html(title)
    )
}

fn render_primary_entry(
    html: &mut String,
    href: Option<&str>,
    title: &str,
    eyebrow: &str,
    description: &str,
    action: &str,
) -> Result<()> {
    let Some(href) = href else {
        write!(html, "<article class=\"primary-card unavailable-card\"><p class=\"eyebrow\">Unavailable</p><h3>{}</h3><p>{}</p><span class=\"unavailable-copy\">No destination was supplied.</span></article>", escape_html(title), escape_html(description))
            .expect("String writes cannot fail");
        return Ok(());
    };
    write!(html, "<a class=\"primary-card\" href=\"{}\"><p class=\"eyebrow\">{}</p><h3>{}</h3><p>{}</p><span class=\"card-action\">{}</span></a>", safe_href(href)?, escape_html(eyebrow), escape_html(title), escape_html(description), escape_html(action))
        .expect("String writes cannot fail");
    Ok(())
}

fn render_secondary_entry(
    html: &mut String,
    href: Option<&str>,
    title: &str,
    description: &str,
) -> Result<()> {
    if let Some(href) = href {
        write!(html, "<a class=\"secondary-card\" href=\"{}\"><span class=\"eyebrow\">Secondary collection</span><strong>{}</strong><span>{}</span></a>", safe_href(href)?, escape_html(title), escape_html(description))
            .expect("String writes cannot fail");
    } else {
        write!(html, "<article class=\"secondary-card unavailable-card\"><span class=\"eyebrow\">Unavailable</span><strong>{}</strong><span>{}</span><span class=\"unavailable-copy\">No destination was supplied.</span></article>", escape_html(title), escape_html(description))
            .expect("String writes cannot fail");
    }
    Ok(())
}

fn render_group_totals(html: &mut String, suite: &SuiteReport) {
    html.push_str("<section class=\"group-totals\" aria-labelledby=\"totals-heading\"><h2 id=\"totals-heading\">Case entries by group</h2><p>These are separate capture entry totals, not success rates or acceptance results.</p><dl>");
    for (group, label, _) in GROUP_COUNTS {
        let count = suite
            .cases
            .iter()
            .filter(|case| case.group == group)
            .count();
        write!(
            html,
            "<div><dt>{}</dt><dd><strong>{count}</strong> recorded {}</dd></div>",
            escape_html(label),
            if count == 1 { "case" } else { "cases" }
        )
        .expect("String writes cannot fail");
    }
    html.push_str("</dl></section>");
}

fn render_case_list(html: &mut String, navigation: &PreviewNavigation) -> Result<()> {
    let suite = &navigation.suite;
    html.push_str("<section class=\"case-index\" aria-labelledby=\"case-index-heading\"><div class=\"section-heading\"><div><h2 id=\"case-index-heading\">All 32 supplied cases</h2><p>One compact list preserves the capture order and keeps unsupported cases visible without inventing flight reports.</p></div></div>");
    html.push_str("<div class=\"case-controls\"><div class=\"group-filters\" role=\"group\" aria-label=\"Filter cases by group\"><button type=\"button\" data-group-filter=\"all\" aria-pressed=\"true\">All cases <span>32</span></button>");
    for (group, label, count) in GROUP_COUNTS {
        write!(html, "<button type=\"button\" data-group-filter=\"{group}\" aria-pressed=\"false\">{} <span>{count}</span></button>", escape_html(label))
            .expect("String writes cannot fail");
    }
    html.push_str("</div><label class=\"search-label\" for=\"case-search\">Search by name, result or case ID<input id=\"case-search\" type=\"search\" autocomplete=\"off\" aria-controls=\"case-list\" placeholder=\"For example, ridge or plateau\"></label></div>");
    write!(
        html,
        "<p class=\"filter-status\" id=\"case-filter-status\" aria-live=\"polite\">Showing all {} recorded cases.</p>",
        suite.cases.len()
    )
    .expect("String writes cannot fail");
    html.push_str("<ul class=\"case-list\" id=\"case-list\" aria-label=\"Waypoint V2 cases grouped by capture category\">");
    for (group, label, count) in GROUP_COUNTS {
        write!(html, "<li class=\"group-heading\" data-group-heading=\"{group}\"><h3>{}</h3><span>{count} recorded cases</span></li>", escape_html(label))
            .expect("String writes cannot fail");
        for case in suite.cases.iter().filter(|case| case.group == group) {
            render_case_row(html, case, &navigation.annotated_case_id)?;
        }
    }
    html.push_str("</ul><noscript><p class=\"noscript-note\">All cases remain listed above. Group filters and search need JavaScript.</p></noscript></section>");
    Ok(())
}

fn render_case_row(html: &mut String, case: &CaseCard, annotated_case_id: &str) -> Result<()> {
    let is_annotated = case.case_id == annotated_case_id;
    write!(html, "<li class=\"case-row\" id=\"case-{}\" data-case-row data-group=\"{}\"><div class=\"case-main\"><h3>{}</h3><code>{}</code><span class=\"group-tag\">{}</span></div><div class=\"case-result\"><strong>Recorded result</strong><span>{}</span></div><div class=\"case-corrections\"><strong>Corrections</strong><span>{}</span></div><div class=\"case-action\">",
        escape_html(&case.case_id), escape_html(&case.group), escape_html(&case.title), escape_html(&case.case_id), group_label(&case.group).expect("validated group"), escape_html(&case.outcome),
        if case.href.is_some() { case.correction_count.to_string() } else { "Unavailable · not simulated".to_owned() })
        .expect("String writes cannot fail");
    if let Some(href) = &case.href {
        let (kind, action) = if is_annotated {
            ("Annotated rich preview", "Open annotated rich preview")
        } else {
            (
                "Original full report · not enhanced",
                "Open original full report",
            )
        };
        write!(
            html,
            "<span class=\"page-kind{}\">{}</span><a class=\"open-report\" href=\"{}\">{}</a>",
            if is_annotated { " annotated" } else { "" },
            escape_html(kind),
            safe_href(href)?,
            escape_html(action)
        )
        .expect("String writes cannot fail");
    } else {
        html.push_str("<span class=\"not-simulated\">Not simulated</span><span class=\"no-report\">No flight report</span>");
    }
    html.push_str("</div>");
    if let Some(reason) = &case.reason {
        write!(
            html,
            "<p class=\"case-reason\"><strong>{}</strong> {}</p>",
            if case.href.is_some() {
                "Recorded note:"
            } else {
                "Reason:"
            },
            escape_html(reason)
        )
        .expect("String writes cannot fail");
    }
    html.push_str("</li>");
    Ok(())
}

fn render_recommendation(
    html: &mut String,
    case: &CaseCard,
    heading: &str,
    description: &str,
    annotated: bool,
) -> Result<()> {
    let href = case
        .href
        .as_deref()
        .expect("validated recommendation report");
    let badge = if annotated {
        "Annotated rich preview"
    } else {
        "Original full report · not enhanced"
    };
    write!(html, "<a class=\"recommendation-card{}\" href=\"{}\"><span class=\"page-kind{}\">{}</span><strong>{}</strong><span>{}</span><span class=\"recommendation-action\">{}</span></a>", if annotated { " highlighted" } else { "" }, safe_href(href)?, if annotated { " annotated" } else { "" }, escape_html(badge), escape_html(heading), escape_html(description), if annotated { "Open annotated rich preview" } else { "Open original full report" })
        .expect("String writes cannot fail");
    Ok(())
}

fn render_suite_sources(html: &mut String, suite: &SuiteReport) -> Result<()> {
    if suite.source_links.is_empty() && suite.diagnostics.is_empty() {
        return Ok(());
    }
    html.push_str(
        "<details class=\"sources\"><summary>Capture details and source links</summary><dl>",
    );
    for (label, value) in &suite.diagnostics {
        write!(
            html,
            "<div><dt>{}</dt><dd>{}</dd></div>",
            escape_html(label),
            escape_html(value)
        )
        .expect("String writes cannot fail");
    }
    html.push_str("</dl>");
    if !suite.source_links.is_empty() {
        html.push_str("<ul class=\"source-links\">");
        for link in &suite.source_links {
            write!(
                html,
                "<li><a href=\"{}\">{}</a></li>",
                safe_href(&link.href)?,
                escape_html(&link.label)
            )
            .expect("String writes cannot fail");
        }
        html.push_str("</ul>");
    }
    html.push_str("</details>");
    Ok(())
}

fn validate_navigation(navigation: &PreviewNavigation) -> Result<()> {
    validate_href(&navigation.home_href).context("invalid home destination")?;
    validate_href(&navigation.suite_href).context("invalid V2 suite destination")?;
    validate_optional_href(navigation.guidance_href.as_deref(), "guidance")?;
    validate_optional_href(navigation.batch_library_href.as_deref(), "batch library")?;
    validate_optional_href(navigation.setups_href.as_deref(), "setups")?;
    validate_optional_href(navigation.replays_href.as_deref(), "replays")?;
    for link in &navigation.history {
        validate_report_link(link).context("invalid history destination")?;
    }

    let suite = &navigation.suite;
    validate_text(&suite.title, "suite title")?;
    validate_text(&suite.capture_label, "capture label")?;
    ensure!(
        suite.policy_version == 3,
        "preview requires waypoint policy version 3"
    );
    ensure!(
        suite.cases.len() == CASE_COUNT,
        "preview requires exactly {CASE_COUNT} supplied cases; found {}",
        suite.cases.len()
    );
    ensure!(
        navigation.annotated_case_id == ANNOTATED_CASE_ID,
        "first preview must annotate the late-ridge case {ANNOTATED_CASE_ID}"
    );

    let mut ids = HashSet::with_capacity(suite.cases.len());
    let mut group_counts = [0usize; 3];
    let mut linked_count = 0usize;
    for case in &suite.cases {
        validate_case(case)?;
        ensure!(
            ids.insert(case.case_id.as_str()),
            "duplicate suite case ID: {}",
            case.case_id
        );
        let index = group_index(&case.group).expect("validated case group");
        group_counts[index] += 1;
        if let Some(href) = &case.href {
            validate_href(href)
                .with_context(|| format!("invalid report link for case {}", case.case_id))?;
            linked_count += 1;
        } else {
            ensure!(
                case.reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty()),
                "unsupported case {} has no recorded reason",
                case.case_id
            );
        }
    }
    for (index, (_, label, expected)) in GROUP_COUNTS.iter().enumerate() {
        ensure!(
            group_counts[index] == *expected,
            "preview requires {expected} {label} cases; found {}",
            group_counts[index]
        );
    }
    ensure!(
        linked_count == 30,
        "preview requires 30 linked flight reports and 2 unsupported cases; found {linked_count} linked reports"
    );
    ensure!(
        find_case(suite, &navigation.annotated_case_id).is_some_and(|case| case.href.is_some()),
        "annotated late-ridge case is missing or has no rich preview destination"
    );
    for (id, label) in [
        (FLAT_CASE_ID, "flat 845 direct control"),
        (PLATEAU_CASE_ID, "reference plateau"),
    ] {
        ensure!(
            find_case(suite, id).is_some_and(|case| case.href.is_some()),
            "required {label} recommendation is missing or unsupported"
        );
    }
    for (label, value) in &suite.diagnostics {
        validate_text(label, "capture diagnostic label")?;
        validate_text(value, "capture diagnostic value")?;
    }
    for link in &suite.source_links {
        validate_report_link(link).context("invalid capture source link")?;
    }
    Ok(())
}

fn validate_case(case: &CaseCard) -> Result<()> {
    ensure!(
        valid_case_id(&case.case_id),
        "invalid or empty suite case ID: {:?}",
        case.case_id
    );
    validate_text(&case.title, "case title")?;
    validate_text(&case.outcome, "case result")?;
    ensure!(
        group_index(&case.group).is_some(),
        "unsupported case group {:?}; expected clear, ordinary, or diagnostic",
        case.group
    );
    if let Some(reason) = &case.reason {
        validate_text(reason, "case reason")?;
    }
    Ok(())
}

fn validate_text(value: &str, label: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} is empty");
    ensure!(
        !value.chars().any(char::is_control),
        "{label} contains a control character"
    );
    Ok(())
}

fn validate_report_link(link: &ReportLink) -> Result<()> {
    validate_text(&link.label, "link label")?;
    validate_href(&link.href).with_context(|| format!("invalid destination for {}", link.label))
}

fn validate_optional_href(href: Option<&str>, label: &str) -> Result<()> {
    if let Some(href) = href {
        validate_href(href).with_context(|| format!("invalid {label} destination"))?;
    }
    Ok(())
}

fn validate_href(href: &str) -> Result<()> {
    ensure!(!href.trim().is_empty(), "link target is empty");
    ensure!(
        href == href.trim(),
        "link target has leading or trailing whitespace"
    );
    ensure!(
        !href
            .chars()
            .any(|character| character.is_control() || character == '\\'),
        "link target contains a control character or backslash"
    );
    ensure!(
        !href.starts_with("//"),
        "protocol-relative link targets are not supported"
    );
    let first_delimiter = href
        .char_indices()
        .find_map(|(index, character)| matches!(character, '/' | '?' | '#').then_some(index))
        .unwrap_or(usize::MAX);
    if let Some(colon) = href.find(':')
        && colon < first_delimiter
    {
        let scheme = href[..colon].to_ascii_lowercase();
        ensure!(
            scheme == "http" || scheme == "https",
            "unsupported link scheme: {scheme}"
        );
    }
    Ok(())
}

fn safe_href(href: &str) -> Result<String> {
    validate_href(href)?;
    Ok(escape_html(href))
}

fn valid_case_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn group_index(group: &str) -> Option<usize> {
    GROUP_COUNTS
        .iter()
        .position(|(candidate, _, _)| *candidate == group)
}

fn group_label(group: &str) -> Option<&'static str> {
    GROUP_COUNTS
        .iter()
        .find(|(candidate, _, _)| *candidate == group)
        .map(|(_, label, _)| *label)
}

fn find_case<'a>(suite: &'a SuiteReport, id: &str) -> Option<&'a CaseCard> {
    suite.cases.iter().find(|case| case.case_id == id)
}

const FILTER_JS: &str = r#"
(() => {
  const rows = Array.from(document.querySelectorAll("[data-case-row]"));
  const headings = Array.from(document.querySelectorAll("[data-group-heading]"));
  const buttons = Array.from(document.querySelectorAll("[data-group-filter]"));
  const search = document.getElementById("case-search");
  const status = document.getElementById("case-filter-status");
  if (!rows.length || !buttons.length || !search || !status) return;
  const normalize = (value) => value.toLocaleLowerCase().trim();
  const apply = () => {
    const active = buttons.find((button) => button.getAttribute("aria-pressed") === "true");
    const group = active ? active.dataset.groupFilter : "all";
    const query = normalize(search.value);
    let visible = 0;
    rows.forEach((row) => {
      const matches = (group === "all" || row.dataset.group === group)
        && (!query || normalize(row.textContent).includes(query));
      row.hidden = !matches;
      if (matches) visible += 1;
    });
    headings.forEach((heading) => {
      const key = heading.dataset.groupHeading;
      heading.hidden = !rows.some((row) => row.dataset.group === key && !row.hidden);
    });
    status.textContent = "Showing " + visible + " of " + rows.length + " recorded cases.";
  };
  buttons.forEach((button) => button.addEventListener("click", () => {
    buttons.forEach((candidate) => candidate.setAttribute("aria-pressed", String(candidate === button)));
    apply();
  }));
  search.addEventListener("input", apply);
})();
"#;

const PREVIEW_CSS: &str = r#"
:root{color-scheme:light;--canvas:#f1ede5;--paper:#fffdf8;--ink:#20211e;--muted:#6d665c;--line:#d9cdbc;--rust:#b95024;--green:#176b5c;--blue:#315f86;--display:"Iowan Old Style","Palatino Linotype",Georgia,serif;--sans:"Avenir Next","IBM Plex Sans","Trebuchet MS",sans-serif;--mono:"Iosevka Term","SFMono-Regular",Consolas,monospace;--shadow:0 18px 44px rgba(54,39,25,.08)}*{box-sizing:border-box}html{background:var(--canvas);color:var(--ink);font-family:var(--sans)}body{margin:0;min-height:100vh;background:radial-gradient(circle at 7% -8%,rgba(185,80,36,.14),transparent 31rem),linear-gradient(180deg,#fbf8f2,var(--canvas))}.page{width:min(1180px,100%);margin:auto;padding:28px 22px 64px}.eyebrow{color:var(--rust);font-size:.68rem;font-weight:800;letter-spacing:.13em;text-transform:uppercase;margin:0 0 8px}.hero{display:flex;justify-content:space-between;align-items:flex-start;gap:20px;border:1px solid var(--line);border-radius:20px;background:rgba(255,253,248,.92);box-shadow:var(--shadow);padding:22px 24px;margin:12px 0 24px}.hero h1{font:500 clamp(2rem,4vw,3.2rem)/1 var(--display);letter-spacing:-.025em;margin:0 0 10px;overflow-wrap:anywhere}.hero p{color:var(--muted);line-height:1.5;max-width:72ch;margin:0}.hero .intro{font-size:1.05rem}.preview-badge,.page-kind,.group-tag,.not-simulated{display:inline-flex;align-items:center;border-radius:999px;padding:5px 9px;font-size:.72rem;line-height:1.3;font-weight:800}.preview-badge{flex:none;background:#f7eadf;color:#823d1d;border:1px solid #e9c7b2}.primary-reading,.secondary-section,.recommendations,.group-totals,.case-index{margin:26px 0}.primary-reading h2,.secondary-section h2,.recommendations h2,.group-totals h2,.case-index h2{font:500 1.55rem/1.2 var(--display);margin:0 0 12px}.primary-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px}.primary-card,.secondary-card,.recommendation-card{min-width:0;color:inherit;text-decoration:none;transition:transform 150ms ease,border-color 150ms ease,box-shadow 150ms ease}.primary-card{position:relative;display:flex;flex-direction:column;gap:9px;min-height:210px;padding:20px;border:1px solid var(--line);border-radius:18px;background:var(--paper);box-shadow:0 12px 32px rgba(54,39,25,.06)}.primary-card:first-child{border-left:5px solid var(--green)}.primary-card h3{font:500 1.45rem/1.15 var(--display);margin:0}.primary-card>p:not(.eyebrow){color:var(--muted);line-height:1.5;margin:0}.card-action{margin-top:auto;padding-top:9px;color:var(--green);font-weight:800}.secondary-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px}.secondary-card{display:flex;flex-direction:column;gap:7px;padding:14px;border:1px solid var(--line);border-radius:14px;background:rgba(255,253,248,.8);min-height:142px}.secondary-card strong,.history-card h3{font:500 1.1rem/1.2 var(--display);margin:0}.secondary-card>span:not(.eyebrow){color:var(--muted);font-size:.88rem;line-height:1.4}.history-card{grid-column:span 2}.history-card>p:not(.eyebrow){font-size:.88rem;color:var(--muted);line-height:1.4;margin:0}.history-list{padding-left:18px;margin:0;line-height:1.5}.history-list a,.source-links a{color:var(--green);overflow-wrap:anywhere}.unavailable-card{color:#666;background:#f5f1e9}.unavailable-card .eyebrow,.unavailable-copy{color:#87572f}.capture-panel{display:grid;grid-template-columns:minmax(0,1fr) minmax(280px,.8fr);gap:18px;padding:19px 21px;border:1px solid var(--line);border-radius:18px;background:var(--paper);box-shadow:0 12px 32px rgba(54,39,25,.05)}.capture-panel h2{font:500 1.35rem/1.2 var(--display);margin:0 0 7px}.capture-label{overflow-wrap:anywhere;font-weight:700}.capture-note{color:var(--muted);line-height:1.45;margin:8px 0 0}.capture-facts{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px 16px;margin:0}.capture-facts dt,.sources dt{color:var(--muted);font-size:.74rem;text-transform:uppercase;letter-spacing:.06em}.capture-facts dd{margin:3px 0 0;font-weight:700}.section-heading{display:flex;justify-content:space-between;gap:16px;align-items:flex-end;margin-bottom:11px}.section-heading h2{margin:0 0 5px}.section-heading p,.group-totals>p{margin:0;color:var(--muted);line-height:1.45}.recommendation-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:11px}.recommendation-card{display:flex;flex-direction:column;gap:8px;border:1px solid var(--line);border-radius:14px;background:var(--paper);padding:14px;min-height:150px}.recommendation-card strong{font:500 1.15rem/1.15 var(--display)}.recommendation-card>span:not(.page-kind){color:var(--muted);font-size:.9rem;line-height:1.4}.recommendation-card .recommendation-action{margin-top:auto;color:var(--green);font-weight:800}.recommendation-card.highlighted{border-color:#dca482;background:#fffaf5}.page-kind{width:fit-content;background:#e7eee9;color:#315849;border:1px solid #cadbd0}.page-kind.annotated{background:#f7eadf;border-color:#e9c7b2;color:#823d1d}.group-totals{padding:16px 18px;border:1px solid var(--line);border-radius:16px;background:rgba(255,253,248,.72)}.group-totals>p{font-size:.88rem}.group-totals dl{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px;margin:13px 0 0}.group-totals dl>div{padding:10px 12px;background:var(--paper);border:1px solid var(--line);border-radius:11px}.group-totals dt{font-weight:800}.group-totals dd{margin:4px 0 0;color:var(--muted)}.group-totals dd strong{color:var(--ink);font-size:1.1rem}.case-controls{display:grid;grid-template-columns:minmax(0,1fr) minmax(230px,.55fr);gap:14px;align-items:end;padding:13px;border:1px solid var(--line);border-radius:14px;background:rgba(255,253,248,.82)}.group-filters{display:flex;flex-wrap:wrap;gap:7px}.group-filters button{font:700 .82rem var(--sans);color:var(--ink);border:1px solid var(--line);border-radius:999px;background:#fffdf8;padding:8px 11px;cursor:pointer}.group-filters button[aria-pressed=true]{background:#e6eee8;border-color:#9db9aa;color:#214b3c}.group-filters button span{color:var(--muted);margin-left:3px}.search-label{display:grid;gap:5px;font-weight:700;font-size:.83rem}.search-label input{min-width:0;width:100%;font:inherit;border:1px solid #bdb3a5;border-radius:9px;padding:9px 10px;background:white}.filter-status{color:var(--muted);font-size:.84rem;margin:11px 2px}.case-list{list-style:none;margin:0;padding:0;border:1px solid var(--line);border-radius:15px;background:var(--paper);overflow:hidden}.group-heading{display:flex;justify-content:space-between;gap:12px;align-items:center;padding:11px 15px;background:#eee7db;border-bottom:1px solid var(--line)}.group-heading h3{font:800 .82rem var(--sans);letter-spacing:.06em;text-transform:uppercase;margin:0}.group-heading span{font-size:.8rem;color:var(--muted)}.case-row{display:grid;grid-template-columns:minmax(170px,1.2fr) minmax(130px,.8fr) minmax(90px,.5fr) minmax(190px,1fr);gap:10px 14px;align-items:center;padding:12px 15px;border-bottom:1px solid #eee9e1;min-width:0}.case-row[hidden],.group-heading[hidden]{display:none}.case-main{display:grid;grid-template-columns:auto minmax(0,1fr);gap:3px 8px;align-items:baseline;min-width:0}.case-main h3{font-size:.95rem;line-height:1.2;margin:0;overflow-wrap:anywhere}.case-main code{grid-column:1/-1;font:.72rem var(--mono);color:var(--muted);overflow-wrap:anywhere}.group-tag{grid-column:2;grid-row:1;background:#f0eee8;color:#655b4e;font-size:.64rem;padding:3px 7px}.case-result,.case-corrections{display:grid;gap:3px;min-width:0}.case-result strong,.case-corrections strong{color:var(--muted);font-size:.67rem;letter-spacing:.05em;text-transform:uppercase}.case-result span,.case-corrections span{font-size:.84rem;line-height:1.35;overflow-wrap:anywhere}.case-action{display:flex;flex-wrap:wrap;gap:6px;align-items:center;min-width:0}.case-action .page-kind{font-size:.63rem;padding:4px 7px}.open-report{color:var(--green);font-size:.82rem;font-weight:800;overflow-wrap:anywhere}.not-simulated{background:#fff0dc;color:#744817}.no-report{color:var(--muted);font-size:.8rem}.case-reason{grid-column:1/-1;margin:0;color:#604419;font-size:.82rem;line-height:1.4;overflow-wrap:anywhere}.sources{margin-top:24px;padding:14px 17px;border:1px solid var(--line);border-radius:14px;background:var(--paper)}.sources summary{font-weight:800;cursor:pointer}.sources dl{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:10px;margin:14px 0}.sources dl div{min-width:0}.sources dd{margin:3px 0;overflow-wrap:anywhere}.source-links{padding-left:20px;line-height:1.55}.breadcrumbs{display:flex;flex-wrap:wrap;gap:9px;align-items:center;margin-bottom:16px;font-size:.87rem}.breadcrumbs a{color:var(--green);font-weight:800}.breadcrumbs span[aria-current=page]{color:var(--muted)}.noscript-note{padding:10px;color:var(--muted)}a:focus-visible,button:focus-visible,input:focus-visible,summary:focus-visible{outline:3px solid rgba(49,95,134,.38);outline-offset:3px}a:hover{border-color:var(--rust);box-shadow:0 14px 33px rgba(54,39,25,.1);transform:translateY(-1px)}button:hover{border-color:var(--rust)}@media(max-width:880px){.page{padding:18px 13px 45px}.secondary-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.capture-panel{grid-template-columns:1fr}.case-controls{grid-template-columns:1fr}.case-row{grid-template-columns:minmax(150px,1.2fr) minmax(120px,.8fr) minmax(85px,.5fr)}.case-action{grid-column:1/-1}.case-reason{grid-column:1/-1}}@media(max-width:580px){.hero{display:grid;padding:18px}.preview-badge{justify-self:start}.primary-grid,.recommendation-grid,.group-totals dl{grid-template-columns:1fr}.primary-card{min-height:180px}.secondary-grid{grid-template-columns:1fr}.history-card{grid-column:auto}.capture-facts{grid-template-columns:1fr 1fr}.case-row{grid-template-columns:1fr 1fr;gap:8px;padding:11px}.case-main{grid-column:1/-1}.case-result{grid-column:1}.case-corrections{grid-column:2}.case-action,.case-reason{grid-column:1/-1}.group-heading{padding:10px 11px}.recommendation-card{min-height:130px}}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, group: &str, index: usize) -> CaseCard {
        let unsupported =
            id == "v2_diag_unsupported_gravity" || id == "v2_diag_unsupported_vehicle";
        CaseCard {
            case_id: id.to_owned(),
            title: match id {
                ANNOTATED_CASE_ID => "Late ridge".to_owned(),
                FLAT_CASE_ID => "Flat 845".to_owned(),
                PLATEAU_CASE_ID => "Reference plateau".to_owned(),
                _ => format!("Case {index}"),
            },
            group: group.to_owned(),
            outcome: if unsupported {
                "Preflight stopped"
            } else {
                "Landed"
            }
            .to_owned(),
            correction_count: if id == PLATEAU_CASE_ID {
                3
            } else if id == ANNOTATED_CASE_ID {
                1
            } else {
                0
            },
            href: (!unsupported).then(|| format!("cases/{id}/index.html")),
            inspect: None,
            reason: unsupported.then(|| "Gravity is unsupported by this capture.".to_owned()),
        }
    }

    fn navigation() -> PreviewNavigation {
        let mut cases = vec![
            card(FLAT_CASE_ID, "clear", 0),
            card(ANNOTATED_CASE_ID, "ordinary", 8),
            card(PLATEAU_CASE_ID, "ordinary", 9),
            card("v2_diag_unsupported_gravity", "diagnostic", 24),
            card("v2_diag_unsupported_vehicle", "diagnostic", 25),
        ];
        for index in 1..8 {
            cases.push(card(&format!("v2_clear_{index}"), "clear", index));
        }
        for index in 0..14 {
            cases.push(card(&format!("v2_ordinary_{index}"), "ordinary", index));
        }
        for index in 0..6 {
            cases.push(card(&format!("v2_diagnostic_{index}"), "diagnostic", index));
        }
        PreviewNavigation {
            suite: SuiteReport {
                title: "Waypoint planner V2 — flight stories".to_owned(),
                capture_label: "Retained policy 3 capture · 2026-10-02".to_owned(),
                policy_version: 3,
                cases,
                diagnostics: vec![("Capture date".to_owned(), "2026-10-02".to_owned())],
                source_links: vec![ReportLink {
                    label: "Original suite summary".to_owned(),
                    href: "../capture/suite-summary.json".to_owned(),
                }],
            },
            annotated_case_id: ANNOTATED_CASE_ID.to_owned(),
            home_href: "../../index.html".to_owned(),
            suite_href: "waypoint-v2/current/index.html".to_owned(),
            guidance_href: Some("guidance/index.html".to_owned()),
            batch_library_href: None,
            setups_href: None,
            replays_href: Some("replays/index.html".to_owned()),
            history: vec![ReportLink {
                label: "Older V2 capture".to_owned(),
                href: "history/older/index.html".to_owned(),
            }],
        }
    }

    #[test]
    fn suite_escapes_text_and_attribute_values() {
        let mut input = navigation();
        input.suite.title = "<script>alert('x')</script>".to_owned();
        input.suite.cases[0].title = "Flat <b>control</b> & direct".to_owned();
        input.suite.cases[0].href = Some("cases/flat.html?x=\"quoted\"&y=1".to_owned());

        let html = render_preview_suite(&input).expect("safe text should render");
        assert!(html.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"));
        assert!(html.contains("Flat &lt;b&gt;control&lt;/b&gt; &amp; direct"));
        assert!(html.contains("href=\"cases/flat.html?x=&quot;quoted&quot;&amp;y=1\""));
        assert!(!html.contains("<script>alert('x')</script>"));
    }

    #[test]
    fn unsafe_schemes_are_rejected_in_all_destinations() {
        let mut input = navigation();
        input.guidance_href = Some("javascript:alert(1)".to_owned());
        assert!(render_preview_home(&input).is_err());

        let mut input = navigation();
        input.guidance_href = Some("//example.org/index.html".to_owned());
        assert!(render_preview_home(&input).is_err());

        let mut input = navigation();
        input.suite.cases[1].href = Some("data:text/html,hi".to_owned());
        assert!(render_preview_suite(&input).is_err());

        let mut input = navigation();
        input.history[0].href = "file:///etc/passwd".to_owned();
        assert!(render_preview_home(&input).is_err());
    }

    #[test]
    fn colons_after_query_or_fragment_delimiters_are_relative_link_content() {
        assert!(validate_href("search?tag=alpha:beta").is_ok());
        assert!(validate_href("#case:detail").is_ok());
        assert!(validate_href("reports/index.html#case:detail").is_ok());
    }

    #[test]
    fn suite_has_separate_group_counts_and_honest_unsupported_rows() {
        let html = render_preview_suite(&navigation()).expect("valid suite should render");
        assert!(html.contains("<strong>8</strong> recorded cases"));
        assert!(html.contains("<strong>16</strong> recorded cases"));
        assert!(html.contains(
            "These are separate capture entry totals, not success rates or acceptance results."
        ));
        assert!(html.contains("id=\"case-v2_diag_unsupported_gravity\""));
        assert!(html.contains("Not simulated"));
        assert!(html.contains("Reason:</strong> Gravity is unsupported by this capture."));
        assert!(!html.contains("href=\"cases/v2_diag_unsupported_gravity/index.html\""));
        assert!(!html.contains("href=\"cases/v2_diag_unsupported_vehicle/index.html\""));
    }

    #[test]
    fn suite_labels_one_annotated_preview_and_original_full_reports() {
        let html = render_preview_suite(&navigation()).expect("valid suite should render");
        assert!(html.contains("Annotated rich preview"));
        assert!(html.contains("Open annotated rich preview"));
        assert!(html.contains("Original full report · not enhanced"));
        assert!(html.contains("Open original full report"));
        assert!(html.contains("v2_clear_845"));
        assert!(html.contains("v2_plateau_reference_900"));
    }

    #[test]
    fn suite_exposes_search_group_filters_and_stable_case_anchors() {
        let html = render_preview_suite(&navigation()).expect("valid suite should render");
        assert!(html.contains("type=\"search\""));
        assert!(html.contains("aria-label=\"Filter cases by group\""));
        assert!(html.contains("data-group-filter=\"ordinary\""));
        assert!(html.contains("data-case-row data-group=\"ordinary\""));
        assert!(html.contains("id=\"case-v2_ridge_late\""));
        assert!(html.contains("search.addEventListener(\"input\", apply)"));
        assert!(html.contains("button.addEventListener(\"click\""));
    }

    #[test]
    fn home_has_one_current_entry_and_marks_empty_destinations_unavailable() {
        let mut input = navigation();
        input.guidance_href = None;
        input.history.clear();
        let html = render_preview_home(&input).expect("valid navigation should render");
        assert_eq!(
            html.matches("href=\"waypoint-v2/current/index.html\"")
                .count(),
            1
        );
        assert_eq!(html.matches("href=\"guidance/index.html\"").count(), 0);
        assert!(html.contains("Maintained guidance overview"));
        assert!(html.contains("No destination was supplied."));
        assert!(html.contains("No history destinations were supplied to this preview."));
        assert_eq!(html.matches("Current waypoint planner V2</h3>").count(), 1);
    }

    #[test]
    fn incomplete_counts_duplicate_ids_and_missing_annotated_case_fail_validation() {
        let mut input = navigation();
        input.suite.cases.pop();
        assert!(render_preview_suite(&input).is_err());

        let mut input = navigation();
        input.suite.cases[1].case_id = input.suite.cases[0].case_id.clone();
        assert!(render_preview_suite(&input).is_err());

        let mut input = navigation();
        input.annotated_case_id = "missing_case".to_owned();
        assert!(render_preview_suite(&input).is_err());
    }
}
