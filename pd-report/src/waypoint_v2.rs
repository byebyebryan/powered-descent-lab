//! Small, self-contained HTML/SVG reports for retained waypoint V2 evidence.
//!
//! This module only presents the checked display payload. It does not load
//! capture files, simulate flight, or infer missing path geometry.

use std::{collections::BTreeSet, fmt::Write as _};

use anyhow::{Context, Result, ensure};
use pd_core::Vec2;

pub mod data;

pub const RENDERER_VERSION: &str = "waypoint-v2-html-svg-1";

const SVG_WIDTH: f64 = 1080.0;
const SVG_HEIGHT: f64 = 700.0;
const PLOT_LEFT: f64 = 100.0;
const PLOT_TOP: f64 = 34.0;
const PLOT_WIDTH: f64 = 920.0;
const PLOT_HEIGHT: f64 = 560.0;

/// Render one flight from actual samples bracketed by exact stored endpoints.
pub fn render_flight(report: &data::FlightReport) -> Result<String> {
    validate_flight(report)?;
    let default_selection = report
        .segments
        .iter()
        .position(|segment| segment.kind == data::SegmentKind::LocalCorrection)
        .or_else(|| (!report.segments.is_empty()).then_some(0));
    let svg = flight_svg(report, default_selection)?;
    let mut html = String::new();
    let title = super::escape_html(&report.title);
    let result = flight_result(report);

    html.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    write!(html, "<title>{title} · Waypoint V2</title>").expect("String writes cannot fail");
    html.push_str("<style>");
    html.push_str(FLIGHT_CSS);
    html.push_str(EXACT_DETAIL_CSS);
    html.push_str("</style></head><body><main class=\"page\">");
    if let Some(href) = &report.index_href {
        write!(
            html,
            "<nav class=\"breadcrumbs\" aria-label=\"Breadcrumb\"><a href=\"{}\">Waypoint V2 suite</a><span aria-hidden=\"true\">›</span><span aria-current=\"page\">{title}</span></nav>",
            safe_href(href)?
        )
        .expect("String writes cannot fail");
    } else {
        html.push_str("<p class=\"eyebrow\">Waypoint V2 flight report</p>");
    }
    write!(
        html,
        "<header class=\"hero\"><h1>{title}</h1><p class=\"result\">{}</p><div class=\"summary\"><span><strong>{}</strong> {}</span><span><strong>{}</strong> elapsed</span></div></header>",
        super::escape_html(&result),
        report.correction_count,
        correction_word(report.correction_count),
        super::escape_html(&format!("{} s", friendly_time(report.elapsed_s)))
    )
    .expect("String writes cannot fail");
    html.push_str("<section class=\"panel chart-panel\"><div class=\"section-heading\"><div><h2>Where it flew</h2><p>Actual executed samples with exact stored segment endpoints. Horizontal and vertical scales match.</p></div></div><figure class=\"trajectory\">");
    html.push_str(&svg);
    if report.segments.is_empty() && !report.landed {
        html.push_str("<figcaption>No flight segment was executed; the diagram shows the recorded start state only.</figcaption>");
    } else {
        html.push_str("<figcaption>H marks where a local correction ended and planning restarted from the actual state. It is not a stop or a landing guarantee.</figcaption>");
    }
    html.push_str("</figure><ul class=\"legend\" aria-label=\"Flight path legend\"><li><span class=\"swatch initial\"></span>Initial nominal flight actually flown</li><li><span class=\"swatch correction\"></span>Local correction actually flown</li><li><span class=\"swatch replanned\"></span>Replanned nominal flight actually flown</li><li><span class=\"swatch terrain\"></span>Terrain</li></ul></section>");

    html.push_str("<section class=\"panel pieces-panel\"><div class=\"section-heading\"><div><h2>Flight pieces</h2><p>Choose a flown piece to highlight it and inspect its exact retained states.</p></div></div>");
    if report.segments.is_empty() {
        html.push_str("<p class=\"empty-state\">No executed flight pieces. The initial proposal did not become a flown path.</p>");
    } else {
        write!(
            html,
            "<ol class=\"piece-list\" data-default-selection=\"{}\">",
            default_selection.expect("nonempty segments have a default selection")
        )
        .expect("String writes cannot fail");
        for (index, segment) in report.segments.iter().enumerate() {
            let start = &segment.points[0];
            let end = segment.points.last().expect("validated nonempty segment");
            let label = segment_label(segment);
            let selected = default_selection == Some(index);
            write!(html, "<li><button type=\"button\" class=\"piece-button\" data-select-segment=\"{index}\" aria-controls=\"segment-detail-{index}\" aria-pressed=\"{selected}\"><span class=\"piece-title\">{}</span><span class=\"piece-range\">Step {}–{} · {}–{} s</span><span class=\"piece-hint\">Select to highlight this flown piece</span></button></li>",
                super::escape_html(&label), start.physics_step, end.physics_step,
                super::escape_html(&friendly_time(start.time_s)), super::escape_html(&friendly_time(end.time_s)))
                .expect("String writes cannot fail");
        }
        html.push_str("</ol>");
        html.push_str("<div class=\"detail-stack\" aria-live=\"polite\">");
        for (index, segment) in report.segments.iter().enumerate() {
            render_segment_detail(&mut html, index, segment, default_selection == Some(index));
        }
        html.push_str("</div><noscript><p class=\"empty-state\">The default selected piece and trajectory remain visible. Selecting another piece requires JavaScript.</p></noscript>");
    }
    html.push_str("</section>");

    render_flight_provenance(&mut html, report)?;
    html.push_str("</main><script>");
    html.push_str(SELECT_SEGMENT_JS);
    html.push_str("</script></body></html>");
    Ok(html)
}

/// Render the curated suite index in the order supplied by the adapter.
pub fn render_suite(report: &data::SuiteReport) -> Result<String> {
    validate_suite(report)?;
    let title = super::escape_html(&report.title);
    let mut html = String::new();
    html.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    write!(html, "<title>{title} · Waypoint V2</title>").expect("String writes cannot fail");
    html.push_str("<style>");
    html.push_str(SUITE_CSS);
    html.push_str("</style></head><body><main class=\"page\"><p class=\"eyebrow\">Waypoint V2 · curated suite</p>");
    write!(html, "<header class=\"hero\"><h1>{title}</h1><p>Policy v{} · source capture: <span class=\"wrap\">{}</span></p><p class=\"intro\">Recorded flight evidence only. Unsupported preflight cases stay visible without an invented flight.</p></header>",
        report.policy_version, super::escape_html(&report.capture_label))
        .expect("String writes cannot fail");

    let featured: Vec<_> = report
        .cases
        .iter()
        .filter(|case| case.inspect.is_some())
        .collect();
    if !featured.is_empty() {
        let heading = if featured.len() == 5 {
            "Start with these five flights"
        } else {
            "Featured flights"
        };
        write!(
            html,
            "<section class=\"feature-section\"><h2>{heading}</h2><div class=\"feature-grid\">"
        )
        .expect("String writes cannot fail");
        for case in &featured {
            render_case_card(&mut html, case, true)?;
        }
        html.push_str("</div>");
        let clear_cases: Vec<_> = report
            .cases
            .iter()
            .filter(|case| case.group == "clear" && case.href.is_some())
            .collect();
        if !clear_cases.is_empty() {
            html.push_str("<p class=\"related-controls\"><strong>Clear direct controls:</strong> ");
            for (index, case) in clear_cases.iter().enumerate() {
                if index > 0 {
                    html.push_str(" · ");
                }
                write!(
                    html,
                    "<a href=\"{}\">{}</a>",
                    safe_href(case.href.as_deref().expect("filtered href"))?,
                    super::escape_html(&case.title)
                )
                .expect("String writes cannot fail");
            }
            html.push_str("</p>");
        }
        html.push_str("</section>");
    }
    html.push_str("<section class=\"all-cases\"><h2>All recorded cases</h2><p>Counts are derived from this capture.</p>");
    for (group, label) in [
        ("clear", "Clear controls"),
        ("ordinary", "Ordinary terrain missions"),
        ("diagnostic", "Diagnostics"),
    ] {
        render_case_group(&mut html, report, group, label)?;
    }
    let extra_groups: BTreeSet<_> = report
        .cases
        .iter()
        .map(|case| case.group.as_str())
        .filter(|group| !matches!(*group, "clear" | "ordinary" | "diagnostic"))
        .collect();
    for group in extra_groups {
        render_case_group(&mut html, report, group, group)?;
    }
    html.push_str("</section>");
    render_suite_provenance(&mut html, report)?;
    html.push_str("</main></body></html>");
    Ok(html)
}

fn validate_flight(report: &data::FlightReport) -> Result<()> {
    ensure!(!report.title.trim().is_empty(), "flight title is empty");
    ensure!(!report.case_id.trim().is_empty(), "flight case ID is empty");
    ensure!(!report.outcome.trim().is_empty(), "flight outcome is empty");
    ensure!(
        report.elapsed_s.is_finite() && report.elapsed_s >= 0.0,
        "flight elapsed time is invalid"
    );
    ensure!(
        report.terrain.len() >= 2,
        "flight terrain needs at least two points"
    );
    for (index, point) in report.terrain.iter().enumerate() {
        ensure!(finite_vec(*point), "terrain point {index} is non-finite");
    }
    ensure!(
        report.terrain.windows(2).all(|pair| pair[1].x >= pair[0].x),
        "terrain points are not ordered by x"
    );
    ensure!(
        finite_point(&report.start),
        "flight start state is non-finite"
    );
    ensure!(
        finite_point(&report.finish),
        "flight finish state is non-finite"
    );
    ensure!(finite_vec(report.source_pad), "source pad is non-finite");
    ensure!(finite_vec(report.target_pad), "target pad is non-finite");
    if let Some(href) = &report.index_href {
        validate_href(href).context("invalid suite index link")?;
    }

    let mut correction_numbers = Vec::new();
    for (segment_index, segment) in report.segments.iter().enumerate() {
        ensure!(
            segment.points.len() >= 2,
            "segment {segment_index} has fewer than two retained points"
        );
        for (point_index, point) in segment.points.iter().enumerate() {
            ensure!(
                finite_point(point),
                "segment {segment_index} point {point_index} is non-finite"
            );
        }
        ensure!(
            segment.points.windows(2).all(|pair| {
                pair[1].physics_step >= pair[0].physics_step && pair[1].time_s >= pair[0].time_s
            }),
            "segment {segment_index} points are not chronological"
        );
        match (segment.kind, segment.correction.as_ref()) {
            (data::SegmentKind::LocalCorrection, Some(correction)) => {
                ensure!(
                    correction.number > 0,
                    "segment {segment_index} has correction number zero"
                );
                ensure!(
                    !correction.reason.trim().is_empty(),
                    "correction {} has no recorded reason",
                    correction.number
                );
                ensure!(
                    !correction.after_handoff.trim().is_empty(),
                    "correction {} has no post-handoff result",
                    correction.number
                );
                if let Some(conflict) = &correction.not_flown_conflict {
                    ensure!(
                        finite_point(conflict),
                        "correction {} conflict point is non-finite",
                        correction.number
                    );
                }
                correction_numbers.push(correction.number);
            }
            (data::SegmentKind::LocalCorrection, None) => {
                anyhow::bail!("local correction segment {segment_index} has no correction details")
            }
            (_, Some(_)) => {
                anyhow::bail!("non-correction segment {segment_index} carries correction details")
            }
            (_, None) => {}
        }
    }
    ensure!(
        correction_numbers.len() == report.correction_count as usize,
        "recorded correction count does not match correction segments"
    );
    ensure!(
        correction_numbers
            .iter()
            .copied()
            .eq(1..=correction_numbers.len()),
        "correction numbers are not in chronological order"
    );
    if let Some(first) = report.segments.first() {
        ensure!(
            same_point_state(&report.start, &first.points[0]),
            "first segment does not begin at the exact start state"
        );
        for (index, pair) in report.segments.windows(2).enumerate() {
            let left = pair[0].points.last().expect("validated nonempty segment");
            let right = &pair[1].points[0];
            ensure!(
                same_point_state(left, right),
                "segments {} and {} do not share an exact boundary state",
                index,
                index + 1
            );
        }
        let last = report.segments.last().expect("first segment exists");
        ensure!(
            same_point_state(
                last.points.last().expect("validated nonempty segment"),
                &report.finish
            ),
            "last segment does not end at the exact finish state"
        );
    }
    for link in &report.source_links {
        validate_link(link)?;
    }
    Ok(())
}

fn validate_suite(report: &data::SuiteReport) -> Result<()> {
    ensure!(!report.title.trim().is_empty(), "suite title is empty");
    ensure!(
        !report.capture_label.trim().is_empty(),
        "suite source capture label is empty"
    );
    ensure!(!report.cases.is_empty(), "suite has no cases");
    let featured = report
        .cases
        .iter()
        .filter(|case| case.inspect.is_some())
        .count();
    ensure!(
        featured <= 5,
        "suite has more than five curated featured cases: {featured}"
    );
    for case in &report.cases {
        ensure!(
            !case.case_id.trim().is_empty(),
            "suite contains an empty case ID"
        );
        ensure!(
            !case.title.trim().is_empty(),
            "suite case {} has an empty title",
            case.case_id
        );
        ensure!(
            !case.group.trim().is_empty(),
            "suite case {} has an empty group",
            case.case_id
        );
        ensure!(
            !case.outcome.trim().is_empty(),
            "suite case {} has an empty result",
            case.case_id
        );
        if let Some(href) = &case.href {
            validate_href(href)
                .with_context(|| format!("invalid report link for case {}", case.case_id))?;
        }
    }
    for link in &report.source_links {
        validate_link(link)?;
    }
    Ok(())
}

fn validate_link(link: &data::ReportLink) -> Result<()> {
    ensure!(!link.label.trim().is_empty(), "source link label is empty");
    validate_href(&link.href).with_context(|| format!("invalid URL for source link {}", link.label))
}

fn validate_href(href: &str) -> Result<()> {
    ensure!(!href.trim().is_empty(), "link target is empty");
    ensure!(
        !href
            .chars()
            .any(|character| character.is_control() || character == '\\'),
        "link target contains a control character or backslash"
    );
    if let Some(colon) = href.find(':') {
        let first_separator = href.find('/').unwrap_or(usize::MAX);
        if colon < first_separator {
            let scheme = href[..colon].to_ascii_lowercase();
            ensure!(
                scheme == "http" || scheme == "https",
                "unsupported link scheme: {scheme}"
            );
        }
    }
    Ok(())
}

fn safe_href(href: &str) -> Result<String> {
    validate_href(href)?;
    Ok(super::escape_html(href))
}

fn finite_vec(value: Vec2) -> bool {
    value.x.is_finite() && value.y.is_finite()
}

fn finite_point(point: &data::FlightPoint) -> bool {
    point.time_s.is_finite()
        && point.time_s >= 0.0
        && finite_vec(point.position_m)
        && finite_vec(point.velocity_mps)
}

fn same_point_state(left: &data::FlightPoint, right: &data::FlightPoint) -> bool {
    left.physics_step == right.physics_step
        && left.time_s == right.time_s
        && left.position_m == right.position_m
        && left.velocity_mps == right.velocity_mps
}

fn flight_result(report: &data::FlightReport) -> String {
    if report.landed {
        if report.correction_count == 0 {
            "Landed directly with no corrections.".to_owned()
        } else if report.correction_count == 1 {
            "Landed after one correction.".to_owned()
        } else {
            format!("Landed after {} corrections.", report.correction_count)
        }
    } else {
        report.outcome.clone()
    }
}

fn correction_word(count: u32) -> &'static str {
    if count == 1 {
        "correction"
    } else {
        "corrections"
    }
}

fn segment_label(segment: &data::FlownSegment) -> String {
    match (&segment.kind, &segment.correction) {
        (data::SegmentKind::InitialNominal, _) => "Initial nominal flight".to_owned(),
        (data::SegmentKind::LocalCorrection, Some(correction)) => {
            format!(
                "Executed local correction {} → H{}",
                correction.number, correction.number
            )
        }
        (data::SegmentKind::ReplannedNominal, _) => "Replanned nominal flight".to_owned(),
        (data::SegmentKind::LocalCorrection, None) => "Local correction".to_owned(),
    }
}

fn render_segment_detail(
    html: &mut String,
    index: usize,
    segment: &data::FlownSegment,
    selected: bool,
) {
    let entry = &segment.points[0];
    let end = segment.points.last().expect("validated nonempty segment");
    let label = segment_label(segment);
    write!(html, "<section class=\"segment-detail\" id=\"segment-detail-{index}\" data-segment-detail=\"{index}\"{}><h3>{}</h3>",
        if selected { "" } else { " hidden" }, super::escape_html(&label))
        .expect("String writes cannot fail");
    match (&segment.kind, &segment.correction) {
        (data::SegmentKind::LocalCorrection, Some(correction)) => {
            write!(html, "<p class=\"reason\"><strong>Why correction {} began:</strong> {}</p><p class=\"handoff-copy\"><strong>What followed H{}:</strong> {}</p><p class=\"handoff-summary\"><strong>Waypoint handoff H{}:</strong> {} s · replan from actual state.</p>",
                correction.number, super::escape_html(&correction.reason), correction.number, super::escape_html(&correction.after_handoff), correction.number, super::escape_html(&friendly_time(end.time_s)))
                .expect("String writes cannot fail");
            html.push_str(
                "<details class=\"exact-detail\"><summary>Exact states and audit detail</summary>",
            );
            write!(html, "<div class=\"state-pair\"><div><h4>Entry E{} · exact stored state</h4>{}</div><div><h4>Handoff H{} · exact stored state</h4>{}</div></div>",
                correction.number, render_state(entry), correction.number, render_state(end))
                .expect("String writes cannot fail");
            if let Some(conflict) = &correction.not_flown_conflict {
                html.push_str("<div class=\"conflict-detail\"><h4>Recorded hypothetical conflict · not flown</h4><p>This diagnostic point is not part of the executed trajectory.</p>");
                html.push_str(&render_state(conflict));
                html.push_str("</div>");
            }
            html.push_str("</details>");
        }
        _ => {
            html.push_str(
                "<details class=\"exact-detail\"><summary>Exact executed states</summary>",
            );
            write!(html, "<div class=\"state-pair\"><div><h4>Piece entry · exact stored state</h4>{}</div><div><h4>Piece end · exact stored state</h4>{}</div></div>", render_state(entry), render_state(end))
                .expect("String writes cannot fail");
            html.push_str("</details>");
        }
    }
    html.push_str("</section>");
}

fn render_state(point: &data::FlightPoint) -> String {
    format!(
        "<dl class=\"state-grid\"><div><dt>Physics step</dt><dd>{}</dd></div><div><dt>Time</dt><dd>{} s</dd></div><div><dt>Position</dt><dd>({}, {}) m</dd></div><div><dt>Velocity</dt><dd>({}, {}) m/s</dd></div></dl>",
        point.physics_step,
        super::escape_html(&exact(point.time_s)),
        super::escape_html(&exact(point.position_m.x)),
        super::escape_html(&exact(point.position_m.y)),
        super::escape_html(&exact(point.velocity_mps.x)),
        super::escape_html(&exact(point.velocity_mps.y)),
    )
}

fn render_flight_provenance(html: &mut String, report: &data::FlightReport) -> Result<()> {
    html.push_str("<details class=\"provenance\"><summary>Capture details and source links</summary><dl class=\"provenance-grid\">");
    provenance_row(html, "Case ID", &report.case_id);
    provenance_row(html, "Source capture", &report.capture_label);
    provenance_row(html, "Policy version", &report.policy_version.to_string());
    provenance_row(html, "Recorded outcome", &report.outcome);
    provenance_row(
        html,
        "Start physics step",
        &report.start.physics_step.to_string(),
    );
    provenance_row(
        html,
        "Finish physics step",
        &report.finish.physics_step.to_string(),
    );
    for (label, value) in &report.diagnostics {
        provenance_row(html, label, value);
    }
    html.push_str("</dl>");
    render_source_links(html, &report.source_links)?;
    html.push_str("</details>");
    Ok(())
}

fn render_suite_provenance(html: &mut String, report: &data::SuiteReport) -> Result<()> {
    html.push_str("<details class=\"provenance\"><summary>Capture details and source links</summary><dl class=\"provenance-grid\">");
    provenance_row(html, "Source capture", &report.capture_label);
    provenance_row(html, "Policy version", &report.policy_version.to_string());
    for (label, value) in &report.diagnostics {
        provenance_row(html, label, value);
    }
    html.push_str("</dl>");
    render_source_links(html, &report.source_links)?;
    html.push_str("</details>");
    Ok(())
}

fn provenance_row(html: &mut String, label: &str, value: &str) {
    write!(
        html,
        "<div><dt>{}</dt><dd>{}</dd></div>",
        super::escape_html(label),
        super::escape_html(value)
    )
    .expect("String writes cannot fail");
}

fn render_source_links(html: &mut String, links: &[data::ReportLink]) -> Result<()> {
    if !links.is_empty() {
        html.push_str("<h3>Source files</h3><ul class=\"source-links\">");
        for link in links {
            write!(
                html,
                "<li><a href=\"{}\">{}</a></li>",
                safe_href(&link.href)?,
                super::escape_html(&link.label)
            )
            .expect("String writes cannot fail");
        }
        html.push_str("</ul>");
    }
    Ok(())
}

fn render_case_group(
    html: &mut String,
    report: &data::SuiteReport,
    group: &str,
    label: &str,
) -> Result<()> {
    let cases: Vec<_> = report
        .cases
        .iter()
        .filter(|case| case.group == group)
        .collect();
    let summary = format!("{} · {} cases", label, cases.len());
    write!(
        html,
        "<details class=\"case-group\"><summary>{}</summary><ul class=\"case-list\">",
        super::escape_html(&summary)
    )
    .expect("String writes cannot fail");
    for case in cases {
        render_case_row(html, case)?;
    }
    html.push_str("</ul></details>");
    Ok(())
}

fn render_case_card(html: &mut String, case: &data::CaseCard, featured: bool) -> Result<()> {
    let css = if featured { "feature-card" } else { "case-row" };
    write!(
        html,
        "<article class=\"{css}\" data-case-id=\"{}\">",
        super::escape_html(&case.case_id)
    )
    .expect("String writes cannot fail");
    if let Some(href) = &case.href {
        write!(
            html,
            "<h3><a href=\"{}\">{}</a></h3>",
            safe_href(href)?,
            super::escape_html(&case.title)
        )
        .expect("String writes cannot fail");
    } else {
        write!(
            html,
            "<h3>{}</h3><span class=\"not-simulated\">Not simulated</span>",
            super::escape_html(&case.title)
        )
        .expect("String writes cannot fail");
    }
    if let Some(inspect) = &case.inspect {
        write!(
            html,
            "<p class=\"inspect\">{}</p>",
            super::escape_html(inspect)
        )
        .expect("String writes cannot fail");
    }
    write!(
        html,
        "<p class=\"case-result\">{}</p><p class=\"case-meta\">{} · {}</p>",
        super::escape_html(&case.outcome),
        case.correction_count,
        if case.correction_count == 1 {
            "correction"
        } else {
            "corrections"
        }
    )
    .expect("String writes cannot fail");
    if let Some(reason) = &case.reason {
        write!(
            html,
            "<p class=\"case-reason\"><strong>Recorded reason:</strong> {}</p>",
            super::escape_html(reason)
        )
        .expect("String writes cannot fail");
    }
    html.push_str("</article>");
    Ok(())
}

fn render_case_row(html: &mut String, case: &data::CaseCard) -> Result<()> {
    write!(
        html,
        "<li class=\"case-row\" data-case-id=\"{}\"><div class=\"case-row-main\">",
        super::escape_html(&case.case_id)
    )
    .expect("String writes cannot fail");
    if let Some(href) = &case.href {
        write!(
            html,
            "<a href=\"{}\">{}</a>",
            safe_href(href)?,
            super::escape_html(&case.title)
        )
        .expect("String writes cannot fail");
    } else {
        write!(html, "<span class=\"case-title\">{}</span><span class=\"not-simulated\">Not simulated</span>", super::escape_html(&case.title))
            .expect("String writes cannot fail");
    }
    write!(
        html,
        "</div><span class=\"case-result\">{}</span>",
        super::escape_html(&case.outcome)
    )
    .expect("String writes cannot fail");
    if let Some(reason) = &case.reason {
        write!(
            html,
            "<span class=\"case-reason\">{}</span>",
            super::escape_html(reason)
        )
        .expect("String writes cannot fail");
    }
    html.push_str("</li>");
    Ok(())
}

fn flight_svg(report: &data::FlightReport, default_selection: Option<usize>) -> Result<String> {
    let plot = Plot::from_report(report)?;
    let mut svg = String::new();
    write!(svg, "<svg class=\"trajectory-svg\" viewBox=\"0 0 {} {}\" role=\"img\" aria-labelledby=\"trajectory-title trajectory-desc\"><title id=\"trajectory-title\">Flown trajectory for {}</title><desc id=\"trajectory-desc\">Actual flight samples, terrain, source and target pads, and exact local-correction handoffs. Both axes use the same metre scale.</desc>", SVG_WIDTH as u16, SVG_HEIGHT as u16, super::escape_html(&report.title))
        .expect("String writes cannot fail");
    write!(
        svg,
        "<rect class=\"plot-area\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\"/>",
        PLOT_LEFT, PLOT_TOP, PLOT_WIDTH, PLOT_HEIGHT
    )
    .expect("String writes cannot fail");
    let visible = plot.visible_bounds();
    ensure!(
        [visible.0, visible.1, visible.2, visible.3]
            .iter()
            .all(|bound| bound.is_finite()),
        "flight geometry produces non-finite chart bounds"
    );
    for tick in 0..=5 {
        let fraction = tick as f64 / 5.0;
        let x = PLOT_LEFT + fraction * PLOT_WIDTH;
        let y = PLOT_TOP + fraction * PLOT_HEIGHT;
        let x_value = visible.0 + fraction * (visible.1 - visible.0);
        let y_value = visible.3 - fraction * (visible.3 - visible.2);
        write!(svg, "<line class=\"grid-line\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/><text class=\"tick-label\" x=\"{}\" y=\"{}\">{}</text><line class=\"grid-line\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/><text class=\"tick-label y-tick\" x=\"{}\" y=\"{}\">{}</text>",
            x, PLOT_TOP, x, PLOT_TOP + PLOT_HEIGHT, x, PLOT_TOP + PLOT_HEIGHT + 23.0, super::escape_html(&axis_number(x_value)),
            PLOT_LEFT, y, PLOT_LEFT + PLOT_WIDTH, y, PLOT_LEFT - 10.0, y + 5.0, super::escape_html(&axis_number(y_value)))
            .expect("String writes cannot fail");
    }
    let terrain_points = points_attribute(&report.terrain, &plot);
    let first_terrain = plot.map(report.terrain[0]);
    let last_terrain = plot.map(*report.terrain.last().expect("validated terrain"));
    write!(svg, "<path class=\"terrain-fill\" d=\"M {} L {} {} L {} {} Z\"/><polyline class=\"terrain-line\" points=\"{}\"/>",
        terrain_points.replace(' ', " L "),
        last_terrain.0, PLOT_TOP + PLOT_HEIGHT,
        first_terrain.0, PLOT_TOP + PLOT_HEIGHT, terrain_points)
        .expect("String writes cannot fail");
    for (index, segment) in report.segments.iter().enumerate() {
        let class = match segment.kind {
            data::SegmentKind::InitialNominal => "initial-flight",
            data::SegmentKind::LocalCorrection => "correction-flight",
            data::SegmentKind::ReplannedNominal => "replanned-flight",
        };
        let selected_class = if default_selection == Some(index) {
            " is-selected"
        } else {
            ""
        };
        write!(svg, "<polyline class=\"flight-path {class}{selected_class}\" data-flight-segment=\"{index}\" points=\"{}\" aria-label=\"{}\"/>", points_attribute(&segment.points.iter().map(|point| point.position_m).collect::<Vec<_>>(), &plot), super::escape_html(&segment_label(segment)))
            .expect("String writes cannot fail");
    }
    render_pad(&mut svg, "source-pad", "Source", report.source_pad, &plot);
    render_pad(&mut svg, "target-pad", "Target", report.target_pad, &plot);
    let start_xy = plot.map(report.start.position_m);
    write!(svg, "<g class=\"start-marker\" transform=\"translate({} {})\" data-step=\"{}\" data-time=\"{}\"><title>Start at step {}, {} s</title><circle r=\"9\"/><text x=\"14\" y=\"-14\">Start</text></g>",
        svg_num(start_xy.0), svg_num(start_xy.1), report.start.physics_step, exact(report.start.time_s), report.start.physics_step, exact(report.start.time_s))
        .expect("String writes cannot fail");
    if report.landed {
        let finish_xy = plot.map(report.finish.position_m);
        write!(svg, "<g class=\"landing-marker\" transform=\"translate({} {})\" data-step=\"{}\" data-time=\"{}\"><title>Landed at step {}, {} s</title><circle r=\"13\"/><path d=\"M -6 0 l4 5 9 -11\"/><text x=\"18\" y=\"-14\">Landed</text></g>",
            svg_num(finish_xy.0), svg_num(finish_xy.1), report.finish.physics_step, exact(report.finish.time_s), report.finish.physics_step, exact(report.finish.time_s))
            .expect("String writes cannot fail");
    } else if !report.segments.is_empty() {
        let finish_xy = plot.map(report.finish.position_m);
        write!(svg, "<g class=\"stop-marker\" transform=\"translate({} {})\" data-step=\"{}\" data-time=\"{}\"><title>Actual stop at step {}, {} s</title><rect x=\"-8\" y=\"-8\" width=\"16\" height=\"16\"/><text x=\"14\" y=\"-14\">Stop</text></g>",
            svg_num(finish_xy.0), svg_num(finish_xy.1), report.finish.physics_step, exact(report.finish.time_s), report.finish.physics_step, exact(report.finish.time_s))
            .expect("String writes cannot fail");
    }
    for (index, segment) in report.segments.iter().enumerate() {
        let Some(correction) = &segment.correction else {
            continue;
        };
        let entry = plot.map(segment.points[0].position_m);
        let handoff = plot.map(segment.points.last().expect("validated segment").position_m);
        let selected_class = if default_selection == Some(index) {
            " is-selected"
        } else {
            ""
        };
        let label_y = handoff_label_y(correction.number);
        write!(svg, "<g class=\"handoff-marker{selected_class}\" data-handoff-segment=\"{index}\" data-step=\"{}\" data-time=\"{}\" data-x-m=\"{}\" data-y-m=\"{}\" transform=\"translate({} {})\"><title>H{} · exact handoff at step {}, {} s</title><circle r=\"11\"/><text x=\"15\" y=\"{}\">H{}</text></g>",
            segment.points.last().expect("validated segment").physics_step,
            exact(segment.points.last().expect("validated segment").time_s),
            exact(segment.points.last().expect("validated segment").position_m.x),
            exact(segment.points.last().expect("validated segment").position_m.y),
            svg_num(handoff.0), svg_num(handoff.1), correction.number,
            segment.points.last().expect("validated segment").physics_step,
            exact(segment.points.last().expect("validated segment").time_s), label_y, correction.number)
            .expect("String writes cannot fail");
        let hidden = if default_selection == Some(index) {
            ""
        } else {
            " hidden"
        };
        write!(svg, "<g class=\"entry-marker\" data-selected-segment=\"{index}\"{hidden} transform=\"translate({} {})\"><title>Correction entry E{} at step {}, {} s</title><rect x=\"-9\" y=\"-9\" width=\"18\" height=\"18\"/><text x=\"14\" y=\"26\">E{}</text></g>",
            svg_num(entry.0), svg_num(entry.1), correction.number, segment.points[0].physics_step,
            exact(segment.points[0].time_s), correction.number)
            .expect("String writes cannot fail");
    }
    write!(svg, "<text class=\"axis-label\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">Horizontal position x (m)</text><text class=\"axis-label\" transform=\"translate(22 {}) rotate(-90)\" text-anchor=\"middle\">Height y (m)</text></svg>",
        PLOT_LEFT + PLOT_WIDTH / 2.0, PLOT_TOP + PLOT_HEIGHT + 65.0, PLOT_TOP + PLOT_HEIGHT / 2.0)
        .expect("String writes cannot fail");
    Ok(svg)
}

fn render_pad(svg: &mut String, class: &str, label: &str, position: Vec2, plot: &Plot) {
    let (x, y) = plot.map(position);
    write!(svg, "<g class=\"{class}\" transform=\"translate({} {})\"><title>{label} pad</title><path d=\"M 0 -11 L 11 0 L 0 11 L -11 0 Z\"/><text x=\"14\" y=\"27\">{label}</text></g>", svg_num(x), svg_num(y))
        .expect("String writes cannot fail");
}

fn points_attribute(points: &[Vec2], plot: &Plot) -> String {
    points
        .iter()
        .map(|point| {
            let (x, y) = plot.map(*point);
            format!("{},{}", svg_num(x), svg_num(y))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct Plot {
    center_x: f64,
    center_y: f64,
    scale: f64,
}

impl Plot {
    fn from_report(report: &data::FlightReport) -> Result<Self> {
        let mut points = Vec::new();
        points.extend(report.terrain.iter().copied());
        points.extend([
            report.source_pad,
            report.target_pad,
            report.start.position_m,
            report.finish.position_m,
        ]);
        for segment in &report.segments {
            points.extend(segment.points.iter().map(|point| point.position_m));
        }
        let min_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        let max_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max);
        let span_x = (max_x - min_x).max(1.0);
        let span_y = (max_y - min_y).max(1.0);
        ensure!(
            span_x.is_finite() && span_y.is_finite(),
            "flight geometry span overflows chart coordinates"
        );
        let scale = (PLOT_WIDTH / (span_x * 1.08)).min(PLOT_HEIGHT / (span_y * 1.08));
        let center_x = min_x / 2.0 + max_x / 2.0;
        let center_y = min_y / 2.0 + max_y / 2.0;
        ensure!(
            scale.is_finite() && scale > 0.0,
            "flight geometry cannot be scaled for the chart"
        );
        ensure!(
            center_x.is_finite() && center_y.is_finite(),
            "flight geometry center is invalid"
        );
        Ok(Self {
            center_x,
            center_y,
            scale,
        })
    }

    fn map(&self, point: Vec2) -> (f64, f64) {
        (
            PLOT_LEFT + PLOT_WIDTH / 2.0 + (point.x - self.center_x) * self.scale,
            PLOT_TOP + PLOT_HEIGHT / 2.0 - (point.y - self.center_y) * self.scale,
        )
    }

    fn visible_bounds(&self) -> (f64, f64, f64, f64) {
        let half_width = PLOT_WIDTH / (2.0 * self.scale);
        let half_height = PLOT_HEIGHT / (2.0 * self.scale);
        (
            self.center_x - half_width,
            self.center_x + half_width,
            self.center_y - half_height,
            self.center_y + half_height,
        )
    }
}

fn exact(value: f64) -> String {
    value.to_string()
}

fn friendly_time(value: f64) -> String {
    let mut number = format!("{value:.6}");
    while number.contains('.') && number.ends_with('0') {
        number.pop();
    }
    if number.ends_with('.') {
        number.pop();
    }
    if number == "-0" {
        "0".to_owned()
    } else {
        number
    }
}

fn handoff_label_y(correction_number: usize) -> i32 {
    if correction_number.is_multiple_of(2) {
        28
    } else {
        -15
    }
}

fn svg_num(value: f64) -> String {
    let mut number = format!("{value:.3}");
    while number.contains('.') && number.ends_with('0') {
        number.pop();
    }
    if number.ends_with('.') {
        number.pop();
    }
    if number == "-0" {
        "0".to_owned()
    } else {
        number
    }
}

fn axis_number(value: f64) -> String {
    format!("{value:.0}")
}

const SELECT_SEGMENT_JS: &str = r#"
(() => {
  const buttons = Array.from(document.querySelectorAll("[data-select-segment]"));
  const paths = Array.from(document.querySelectorAll("[data-flight-segment]"));
  const handoffs = Array.from(document.querySelectorAll("[data-handoff-segment]"));
  const details = Array.from(document.querySelectorAll("[data-segment-detail]"));
  const selectedMarkers = Array.from(document.querySelectorAll("[data-selected-segment]"));
  const select = (selected) => {
    buttons.forEach((button) => {
      const active = button.dataset.selectSegment === selected;
      button.setAttribute("aria-pressed", String(active));
    });
    paths.forEach((path) => path.classList.toggle("is-selected", path.dataset.flightSegment === selected));
    handoffs.forEach((marker) => marker.classList.toggle("is-selected", marker.dataset.handoffSegment === selected));
    details.forEach((panel) => { panel.hidden = panel.dataset.segmentDetail !== selected; });
    selectedMarkers.forEach((marker) => marker.toggleAttribute("hidden", marker.dataset.selectedSegment !== selected));
  };
  buttons.forEach((button) => button.addEventListener("click", () => select(button.dataset.selectSegment)));
  const list = document.querySelector("[data-default-selection]");
  if (list) select(list.dataset.defaultSelection);
})();
"#;

const FLIGHT_CSS: &str = r#"
:root{color-scheme:light;--ink:#172331;--muted:#566575;--line:#d8e0e7;--panel:#fff;--page:#f2f5f7;--blue:#2376c9;--orange:#d97706;--teal:#138c8c;--terrain:#58606a;--focus:#1a63b8}
*{box-sizing:border-box}html{background:var(--page);color:var(--ink);font-family:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}body{margin:0;overflow-x:hidden}.page{max-width:1180px;margin:0 auto;padding:24px clamp(14px,3vw,34px) 48px}.breadcrumbs{display:flex;gap:9px;align-items:center;flex-wrap:wrap;margin:0 0 18px;color:var(--muted);font-size:.92rem}.breadcrumbs a{color:#155da1}.hero{padding:10px 0 22px}.eyebrow{margin:0 0 8px;color:#405466;font-weight:700;letter-spacing:.05em;text-transform:uppercase;font-size:.8rem}h1{font-size:clamp(1.85rem,4vw,3rem);line-height:1.08;margin:0 0 12px;overflow-wrap:anywhere}.result{font-size:clamp(1.15rem,2.4vw,1.55rem);font-weight:680;margin:0 0 17px}.summary{display:flex;flex-wrap:wrap;gap:10px}.summary span{padding:8px 12px;border:1px solid var(--line);border-radius:999px;background:#fff;color:var(--muted)}.summary strong{color:var(--ink)}.panel{background:var(--panel);border:1px solid var(--line);border-radius:16px;padding:clamp(15px,2.5vw,26px);margin:0 0 20px;box-shadow:0 6px 24px rgba(22,37,52,.045)}.section-heading h2,.pieces-panel h2{margin:0 0 5px;font-size:1.35rem}.section-heading p{margin:0;color:var(--muted)}.trajectory{margin:18px 0 0}.trajectory-svg{display:block;width:100%;height:auto;max-width:100%;overflow:visible}.trajectory figcaption{margin:8px 2px 0;color:var(--muted);font-size:.92rem;line-height:1.45}.plot-area{fill:#fbfdff;stroke:#cad4de;stroke-width:1.5}.grid-line{stroke:#dfe6ec;stroke-width:1;vector-effect:non-scaling-stroke}.tick-label{fill:#526273;font:22px system-ui,sans-serif;text-anchor:middle}.y-tick{text-anchor:end}.axis-label{fill:#263647;font:24px system-ui,sans-serif;font-weight:650}.terrain-fill{fill:#65717c;fill-opacity:.13;stroke:none}.terrain-line{fill:none;stroke:var(--terrain);stroke-width:3;vector-effect:non-scaling-stroke}.flight-path{fill:none;stroke-width:5;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke;transition:stroke-width .12s,filter .12s}.initial-flight{stroke:var(--blue)}.correction-flight{stroke:var(--orange)}.replanned-flight{stroke:var(--teal)}.flight-path.is-selected{stroke-width:10;filter:drop-shadow(0 0 3px rgba(0,0,0,.35))}.source-pad,.target-pad{fill:#fff;stroke:#384b5d;stroke-width:3;vector-effect:non-scaling-stroke}.source-pad text,.target-pad text,.start-marker text,.landing-marker text,.stop-marker text,.handoff-marker text,.entry-marker text,.conflict-marker text{font:29px system-ui,sans-serif;font-weight:700;fill:var(--ink);paint-order:stroke;stroke:#fff;stroke-width:5px;stroke-linejoin:round;vector-effect:non-scaling-stroke}.start-marker circle{fill:#fff;stroke:#243d57;stroke-width:4;vector-effect:non-scaling-stroke}.landing-marker circle{fill:#e5f7e9;stroke:#16803c;stroke-width:4;vector-effect:non-scaling-stroke}.landing-marker path{fill:none;stroke:#16803c;stroke-width:4;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke}.landing-marker text{fill:#126b34}.stop-marker rect{fill:#fff1e8;stroke:#b64e28;stroke-width:4;vector-effect:non-scaling-stroke}.stop-marker text{fill:#943a1d}.handoff-marker circle{fill:#fff4df;stroke:var(--orange);stroke-width:4;vector-effect:non-scaling-stroke}.handoff-marker.is-selected circle{stroke-width:8;filter:drop-shadow(0 0 3px rgba(217,119,6,.65))}.handoff-marker.is-selected text{fill:#9b4e00}.entry-marker rect{fill:#e8f2ff;stroke:#155da1;stroke-width:4;vector-effect:non-scaling-stroke}.entry-marker text{fill:#155da1}.conflict-marker path{fill:none;stroke:#a32b70;stroke-width:5;stroke-linecap:round;vector-effect:non-scaling-stroke}.conflict-marker text{fill:#8d205d}.legend{display:flex;gap:9px 20px;flex-wrap:wrap;list-style:none;margin:16px 0 0;padding:0;color:#394b5b;font-size:.92rem}.legend li{display:flex;align-items:center;gap:8px}.swatch{width:26px;height:5px;border-radius:9px;display:inline-block;background:var(--blue)}.swatch.correction{background:var(--orange)}.swatch.replanned{background:var(--teal)}.swatch.terrain{background:var(--terrain)}.piece-list{list-style:none;padding:0;margin:17px 0 0;display:grid;gap:9px}.piece-button{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:3px 12px;width:100%;padding:14px 15px;border:1px solid var(--line);border-radius:11px;background:#fff;color:var(--ink);text-align:left;font:inherit;cursor:pointer}.piece-button:hover{border-color:#8ca5bb;background:#f8fbfd}.piece-button[aria-pressed="true"]{border:2px solid #236da7;background:#eef7ff;padding:13px 14px}.piece-title{font-weight:720}.piece-range{color:#435669;font-variant-numeric:tabular-nums}.piece-hint{grid-column:1/-1;color:var(--muted);font-size:.87rem}.piece-button:focus-visible,a:focus-visible,summary:focus-visible{outline:3px solid #2879c8;outline-offset:3px}.segment-detail{margin:16px 0 0;padding:18px;border-radius:12px;background:#f6f9fb;border:1px solid var(--line)}[hidden]{display:none!important}.segment-detail h3{margin:0 0 7px}.segment-detail>p{line-height:1.5}.reason,.handoff-copy{padding:12px 14px;border-radius:9px;background:#fff;border-left:4px solid var(--orange)}.handoff-copy{border-left-color:var(--teal)}.not-flown-note,.conflict-detail{padding:12px 14px;background:#fff7ea;border:1px solid #f0d4aa;border-radius:9px;line-height:1.5}.state-pair{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;margin:15px 0}.state-pair>div{min-width:0;padding:13px;background:#fff;border:1px solid var(--line);border-radius:9px}.state-pair h4,.conflict-detail h4{margin:0 0 9px}.state-grid,.provenance-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px;margin:0}.state-grid>div,.provenance-grid>div{min-width:0}.state-grid dt,.provenance-grid dt{color:var(--muted);font-size:.8rem}.state-grid dd,.provenance-grid dd{margin:2px 0 0;font-variant-numeric:tabular-nums;overflow-wrap:anywhere}.empty-state{color:var(--muted);padding:14px 0}.provenance{background:#fff;border:1px solid var(--line);border-radius:13px;padding:14px 17px;margin-top:20px}.provenance summary{cursor:pointer;font-weight:680}.provenance-grid{margin:15px 0}.source-links{padding-left:20px}.source-links a{color:#155da1;overflow-wrap:anywhere}
@media(max-width:620px){.page{padding:15px 12px 32px}.panel{border-radius:12px}.piece-button{grid-template-columns:1fr;gap:5px}.piece-range{font-size:.91rem}.state-pair{grid-template-columns:1fr}.state-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.trajectory-svg .tick-label{font-size:28px}.trajectory-svg .axis-label{font-size:31px}.trajectory-svg .source-pad text,.trajectory-svg .target-pad text,.trajectory-svg .start-marker text,.trajectory-svg .landing-marker text,.trajectory-svg .stop-marker text,.trajectory-svg .handoff-marker text,.trajectory-svg .entry-marker text,.trajectory-svg .conflict-marker text{font-size:34px}}
"#;

const EXACT_DETAIL_CSS: &str = r#"
.handoff-summary{padding:10px 14px;border-radius:9px;background:#e8f7f6;border-left:4px solid var(--teal);line-height:1.45}
.exact-detail{margin-top:14px;padding:13px 14px;background:#fff;border:1px solid var(--line);border-radius:9px}
.exact-detail summary{cursor:pointer;font-weight:700}
.exact-detail[open] summary{margin-bottom:13px}
.conflict-detail{margin-top:12px}
"#;

const SUITE_CSS: &str = r#"
:root{color-scheme:light;--ink:#172331;--muted:#566575;--line:#d8e0e7;--page:#f2f5f7;--blue:#155da1}*{box-sizing:border-box}html{background:var(--page);color:var(--ink);font-family:system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}body{margin:0;overflow-x:hidden}.page{max-width:1160px;margin:0 auto;padding:28px clamp(14px,3vw,34px) 48px}.eyebrow{color:#405466;font-weight:700;letter-spacing:.05em;text-transform:uppercase;font-size:.8rem;margin:0 0 8px}h1{font-size:clamp(1.9rem,4vw,3rem);line-height:1.1;margin:0 0 12px;overflow-wrap:anywhere}.hero>p{color:var(--muted);line-height:1.5}.hero .intro{font-size:1.1rem;color:var(--ink)}.wrap{overflow-wrap:anywhere}.feature-section,.all-cases{margin:28px 0}.feature-section h2,.all-cases h2{font-size:1.45rem}.feature-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,270px),1fr));gap:12px}.feature-card{background:#fff;border:1px solid var(--line);border-radius:13px;padding:16px;min-width:0;box-shadow:0 5px 18px rgba(22,37,52,.04)}.feature-card h3{margin:0 0 8px;font-size:1.1rem;overflow-wrap:anywhere}.feature-card h3 a,.case-row a{color:var(--blue);text-decoration-thickness:1px;text-underline-offset:3px}.feature-card p{line-height:1.45;overflow-wrap:anywhere}.inspect{color:#354a5c}.case-result{font-weight:650;margin:8px 0}.case-meta{font-size:.87rem;color:var(--muted);margin:0}.case-reason{color:#604419;font-size:.92rem}.not-simulated{display:inline-block;border-radius:999px;padding:3px 8px;background:#fff0dc;color:#744817;font-weight:700;font-size:.78rem}.related-controls{background:#eaf2f8;padding:12px 14px;border-radius:9px;line-height:1.8}.related-controls a{white-space:normal;overflow-wrap:anywhere}.case-group{background:#fff;border:1px solid var(--line);border-radius:11px;margin:9px 0;overflow:hidden}.case-group summary{padding:14px 16px;cursor:pointer;font-weight:700}.case-list{list-style:none;padding:0;margin:0;border-top:1px solid var(--line)}.case-row{padding:12px 16px;display:grid;grid-template-columns:minmax(170px,.8fr) minmax(0,1fr) auto;gap:8px 14px;align-items:start;border-bottom:1px solid #edf1f4;min-width:0}.case-row:last-child{border-bottom:0}.case-row-main{display:flex;flex-wrap:wrap;gap:8px;align-items:center;min-width:0;overflow-wrap:anywhere}.case-row .case-result{font-weight:400;color:#354a5c;margin:0;overflow-wrap:anywhere}.case-row .case-reason{grid-column:2/-1}.provenance{background:#fff;border:1px solid var(--line);border-radius:12px;padding:14px 17px;margin-top:25px}.provenance summary{cursor:pointer;font-weight:700}.provenance-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,230px),1fr));gap:12px;margin:16px 0}.provenance-grid dt{font-size:.8rem;color:var(--muted)}.provenance-grid dd{margin:3px 0;overflow-wrap:anywhere}.source-links{padding-left:20px}.source-links a{color:var(--blue);overflow-wrap:anywhere}a:focus-visible,summary:focus-visible{outline:3px solid #2879c8;outline-offset:3px}@media(max-width:640px){.page{padding:18px 12px 34px}.case-row{grid-template-columns:1fr;gap:6px}.case-row .case-reason{grid-column:auto}.feature-card{padding:14px}}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn point(step: u64, time_s: f64, x: f64, y: f64) -> data::FlightPoint {
        data::FlightPoint {
            physics_step: step,
            time_s,
            position_m: Vec2::new(x, y),
            velocity_mps: Vec2::new(2.5, -1.25),
        }
    }

    fn base_flight() -> data::FlightReport {
        let start = point(0, 0.0, 0.0, 40.0);
        let finish = point(40, 4.0, 100.0, 2.0);
        data::FlightReport {
            title: "Late ridge".to_owned(),
            case_id: "late_ridge".to_owned(),
            outcome: "Landed".to_owned(),
            landed: true,
            correction_count: 1,
            elapsed_s: 4.0,
            terrain: vec![Vec2::new(-20.0, 0.0), Vec2::new(120.0, 0.0)],
            source_pad: Vec2::new(0.0, 0.0),
            target_pad: Vec2::new(100.0, 0.0),
            start: start.clone(),
            finish: finish.clone(),
            segments: vec![
                data::FlownSegment {
                    kind: data::SegmentKind::InitialNominal,
                    points: vec![start, point(16, 1.6, 40.0, 27.0)],
                    correction: None,
                },
                data::FlownSegment {
                    kind: data::SegmentKind::LocalCorrection,
                    points: vec![point(16, 1.6, 40.0, 27.0), point(23, 2.3, 56.0, 21.0)],
                    correction: Some(data::Correction {
                        number: 1,
                        reason: "The selected direct proposal failed the recorded body-clearance reserve.".to_owned(),
                        after_handoff: "A new direct nominal flight followed and landed.".to_owned(),
                        not_flown_conflict: Some(point(24, 2.4, 61.0, 8.0)),
                    }),
                },
                data::FlownSegment {
                    kind: data::SegmentKind::ReplannedNominal,
                    points: vec![point(23, 2.3, 56.0, 21.0), finish.clone()],
                    correction: None,
                },
            ],
            index_href: Some("index.html".to_owned()),
            capture_label: "final policy 3 capture".to_owned(),
            policy_version: 3,
            diagnostics: vec![("Input digest".to_owned(), "sha256:abc".to_owned())],
            source_links: vec![data::ReportLink {
                label: "flight.json".to_owned(),
                href: "../capture/flight.json".to_owned(),
            }],
        }
    }

    fn card(id: &str, group: &str, featured: bool) -> data::CaseCard {
        data::CaseCard {
            case_id: id.to_owned(),
            title: id.replace('_', " "),
            group: group.to_owned(),
            outcome: "Landed directly".to_owned(),
            correction_count: 0,
            href: Some(format!("{id}.html")),
            inspect: featured.then(|| format!("Inspect {id} behavior.")),
            reason: None,
        }
    }

    fn base_suite() -> data::SuiteReport {
        let mut cases = vec![
            card("flat_direct", "clear", true),
            card("late_ridge", "ordinary", true),
            card("successive_rising", "ordinary", true),
            card("reference_plateau", "ordinary", true),
            card("near_target", "diagnostic", true),
        ];
        for index in 0..7 {
            cases.push(card(&format!("clear_{index}"), "clear", false));
        }
        for index in 0..13 {
            cases.push(card(&format!("ordinary_{index}"), "ordinary", false));
        }
        for index in 0..6 {
            cases.push(card(&format!("diagnostic_{index}"), "diagnostic", false));
        }
        cases.push(data::CaseCard {
            case_id: "unsupported_vehicle".to_owned(),
            title: "Unsupported vehicle".to_owned(),
            group: "diagnostic".to_owned(),
            outcome: "Preflight stopped".to_owned(),
            correction_count: 0,
            href: None,
            inspect: None,
            reason: Some("Vehicle data is unavailable.".to_owned()),
        });
        data::SuiteReport {
            title: "Waypoint V2 flights".to_owned(),
            capture_label: "final policy 3 retained capture".to_owned(),
            policy_version: 3,
            cases,
            diagnostics: vec![("Source digest".to_owned(), "sha256:123".to_owned())],
            source_links: vec![data::ReportLink {
                label: "Capture summary".to_owned(),
                href: "../capture/summary.json".to_owned(),
            }],
        }
    }

    #[test]
    fn flight_html_uses_exact_off_sample_handoff_and_selection_hooks() {
        let mut report = base_flight();
        let handoff = point(2426, 20.216667, -288.849785, 318.285445);
        let correction_entry = point(1512, 12.6, -420.0, 250.0);
        report.start = point(0, 0.0, -600.0, 500.0);
        report.finish = point(4080, 34.0, 70.0, 2.0);
        report.elapsed_s = 34.0;
        report.segments = vec![
            data::FlownSegment {
                kind: data::SegmentKind::InitialNominal,
                points: vec![report.start.clone(), correction_entry.clone()],
                correction: None,
            },
            data::FlownSegment {
                kind: data::SegmentKind::LocalCorrection,
                points: vec![correction_entry, handoff.clone()],
                correction: Some(data::Correction {
                    number: 1,
                    reason: "Body clearance fell below the 5 m reserve.".to_owned(),
                    after_handoff: "The new direct flight landed.".to_owned(),
                    not_flown_conflict: Some(point(2427, 20.225, -286.0, 200.0)),
                }),
            },
            data::FlownSegment {
                kind: data::SegmentKind::ReplannedNominal,
                points: vec![handoff.clone(), report.finish.clone()],
                correction: None,
            },
        ];
        report.correction_count = 1;
        let html = render_flight(&report).expect("fixture report should render");
        assert!(html.contains("Landed after one correction."));
        assert!(html.contains("data-step=\"2426\" data-time=\"20.216667\" data-x-m=\"-288.849785\" data-y-m=\"318.285445\""));
        assert!(html.contains("data-handoff-segment=\"1\""));
        assert!(html.contains("data-select-segment=\"1\""));
        assert!(html.contains("aria-pressed=\"false\""));
        assert!(html.contains("data-selected-segment=\"1\" transform="));
        assert!(html.contains("aria-pressed=\"true\""));
        assert!(html.contains(
            "marker.toggleAttribute(\"hidden\", marker.dataset.selectedSegment !== selected)"
        ));
        assert!(html.contains("Recorded hypothetical conflict · not flown"));
        assert!(html.contains("data-time=\"20.216667\""));
        assert!(html.contains("Step 1512–2426 · 12.6–20.216667 s"));
        assert!(
            html.contains("Waypoint handoff H1:</strong> 20.216667 s · replan from actual state.")
        );
        assert!(html.contains(
            "<details class=\"exact-detail\"><summary>Exact states and audit detail</summary>"
        ));
        let why = html
            .find("Why correction 1 began:")
            .expect("correction reason visible");
        let followed = html
            .find("What followed H1:")
            .expect("after-handoff result visible");
        let exact_details = html
            .find("Exact states and audit detail")
            .expect("exact details disclosure");
        assert!(why < exact_details && followed < exact_details);
        let conflict = html
            .find("Recorded hypothetical conflict · not flown")
            .expect("conflict detail exists");
        assert!(conflict > exact_details);
        assert!(!html.contains("class=\"conflict-marker\""));
        assert!(!html.contains("class=\"not-flown-note\""));
        assert!(html.contains("y=\"-15\">H1</text>"));
        assert!(html.contains("Replanned nominal flight</span>"));
        assert!(html.contains("data-flight-segment=\"2\""));
        assert!(!html.to_ascii_lowercase().contains("plotly"));
        assert!(!html.contains("<script src="));
    }

    #[test]
    fn repeated_handoffs_remain_visible_and_first_correction_is_selected() {
        let mut report = base_flight();
        let start = point(0, 0.0, 0.0, 40.0);
        let entry_one = point(10, 1.0, 20.0, 33.0);
        let handoff_one = point(20, 2.0, 42.0, 26.0);
        let entry_two = point(24, 2.4, 50.0, 21.0);
        let handoff_two = point(30, 3.0, 67.0, 14.0);
        let finish = point(40, 4.0, 100.0, 2.0);
        report.start = start.clone();
        report.finish = finish.clone();
        report.elapsed_s = 4.0;
        report.correction_count = 2;
        report.segments = vec![
            data::FlownSegment {
                kind: data::SegmentKind::InitialNominal,
                points: vec![start, entry_one.clone()],
                correction: None,
            },
            data::FlownSegment {
                kind: data::SegmentKind::LocalCorrection,
                points: vec![entry_one.clone(), handoff_one.clone()],
                correction: Some(data::Correction {
                    number: 1,
                    reason: "The first direct proposal was blocked by terrain.".to_owned(),
                    after_handoff: "A replanned flight continued to the next obstruction."
                        .to_owned(),
                    not_flown_conflict: Some(point(21, 2.1, 44.0, 18.0)),
                }),
            },
            data::FlownSegment {
                kind: data::SegmentKind::ReplannedNominal,
                points: vec![handoff_one, entry_two.clone()],
                correction: None,
            },
            data::FlownSegment {
                kind: data::SegmentKind::LocalCorrection,
                points: vec![entry_two, handoff_two.clone()],
                correction: Some(data::Correction {
                    number: 2,
                    reason: "The next selected direct proposal also failed clearance.".to_owned(),
                    after_handoff: "A final replanned flight landed.".to_owned(),
                    not_flown_conflict: Some(point(31, 3.1, 69.0, 7.0)),
                }),
            },
            data::FlownSegment {
                kind: data::SegmentKind::ReplannedNominal,
                points: vec![handoff_two, finish],
                correction: None,
            },
        ];
        let html = render_flight(&report).expect("repeated-correction flight should render");
        assert_eq!(html.matches("class=\"handoff-marker").count(), 2);
        assert_eq!(html.matches("data-select-segment=").count(), 5);
        assert!(html.contains("class=\"handoff-marker is-selected\" data-handoff-segment=\"1\""));
        assert!(html.contains("class=\"handoff-marker\" data-handoff-segment=\"3\""));
        assert!(html.contains("y=\"-15\">H1</text>"));
        assert!(html.contains("y=\"28\">H2</text>"));
        assert!(html.contains(
            "id=\"segment-detail-1\" data-segment-detail=\"1\"><h3>Executed local correction 1"
        ));
        assert!(html.contains("id=\"segment-detail-3\" data-segment-detail=\"3\" hidden"));
        assert!(html.contains("data-selected-segment=\"1\" transform="));
        assert!(html.contains("data-selected-segment=\"3\" hidden"));
        assert!(html.contains("data-flight-segment=\"2\""));
        assert!(html.contains("data-flight-segment=\"4\""));
    }

    #[test]
    fn compact_time_and_handoff_label_rules_are_stable() {
        assert_eq!(friendly_time(12.6), "12.6");
        assert_eq!(friendly_time(20.216667), "20.216667");
        assert_eq!(friendly_time(20.21666742), "20.216667");
        assert_eq!(axis_number(240.6), "241");
        assert_eq!(handoff_label_y(1), -15);
        assert_eq!(handoff_label_y(2), 28);
        assert_eq!(handoff_label_y(3), -15);
        let replanned = data::FlownSegment {
            kind: data::SegmentKind::ReplannedNominal,
            points: vec![point(0, 0.0, 0.0, 1.0), point(1, 0.1, 1.0, 0.0)],
            correction: None,
        };
        assert_eq!(segment_label(&replanned), "Replanned nominal flight");
    }

    #[test]
    fn flight_html_escapes_text_and_shows_no_departure_or_fake_landing() {
        let mut report = base_flight();
        report.title = "<script>alert('x')</script>".to_owned();
        report.outcome = "Stopped before departure: NoClearing & stopped.".to_owned();
        report.landed = false;
        report.correction_count = 0;
        report.elapsed_s = 0.0;
        report.segments.clear();
        report.start = point(0, 0.0, 0.0, 40.0);
        report.finish = report.start.clone();
        let html = render_flight(&report).expect("empty-flight report should render");
        assert!(html.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"));
        assert!(html.contains("Stopped before departure: NoClearing &amp; stopped."));
        assert!(html.contains("0</strong> corrections"));
        assert!(html.contains("0 s</strong> elapsed"));
        assert!(html.contains("No flight segment was executed"));
        assert!(!html.contains("class=\"landing-marker\""));
        assert!(!html.contains("class=\"stop-marker\""));
        assert!(!html.contains("crash"));
        assert!(!html.contains("&lt;/script&gt;<script>alert"));
    }

    #[test]
    fn partial_flight_has_a_stop_marker_and_no_landing_symbol() {
        let mut report = base_flight();
        report.landed = false;
        report.outcome = "Stopped after actual terrain contact.".to_owned();
        report.correction_count = 0;
        report.segments = vec![data::FlownSegment {
            kind: data::SegmentKind::InitialNominal,
            points: vec![report.start.clone(), report.finish.clone()],
            correction: None,
        }];
        let html = render_flight(&report).expect("partial report should render");
        assert!(html.contains("class=\"stop-marker\""));
        assert!(!html.contains("class=\"landing-marker\""));
        assert!(html.contains("Stopped after actual terrain contact."));
        assert!(html.contains("Actual stop at step"));
        assert!(html.contains("data-default-selection=\"0\""));
        assert!(html.contains("aria-pressed=\"true\""));
    }

    #[test]
    fn renderer_rejects_missing_or_non_finite_segment_geometry() {
        let mut report = base_flight();
        report.terrain.clear();
        assert!(
            render_flight(&report)
                .unwrap_err()
                .to_string()
                .contains("terrain")
        );

        let mut report = base_flight();
        report.segments[1].points.clear();
        assert!(
            render_flight(&report)
                .unwrap_err()
                .to_string()
                .contains("fewer than two")
        );

        let mut report = base_flight();
        report.segments[0].points[1].position_m.x = f64::NAN;
        assert!(
            render_flight(&report)
                .unwrap_err()
                .to_string()
                .contains("non-finite")
        );
    }

    #[test]
    fn suite_index_keeps_unsupported_cases_unlinked_and_renders_all_groups() {
        let report = base_suite();
        let html = render_suite(&report).expect("suite index should render");
        assert!(html.contains("Start with these five flights"));
        assert!(html.contains("Clear controls · 8 cases"));
        assert!(html.contains("Ordinary terrain missions · 16 cases"));
        assert!(html.contains("Diagnostics · 8 cases"));
        assert!(html.contains("Unsupported vehicle"));
        assert!(html.contains("Not simulated"));
        assert!(html.contains("Vehicle data is unavailable."));
        assert!(!html.contains("href=\"undefined\""));
        assert!(!html.contains("href=\"unsupported_vehicle.html\""));
        assert!(html.contains("href=\"flat_direct.html\""));
        assert!(html.find("flat direct").unwrap() < html.find("late ridge").unwrap());
        assert!(html.contains("Clear direct controls:"));
        assert!(html.contains("Policy v3"));
    }

    #[test]
    fn suite_index_escapes_and_rejects_unsafe_links() {
        let mut report = base_suite();
        report.cases[0].title = "<img src=x onerror=alert(1)>".to_owned();
        report.cases[0].href = Some("javascript:alert(1)".to_owned());
        let error = render_suite(&report).unwrap_err().to_string();
        assert!(error.contains("invalid report link"));

        report.cases[0].href = Some("flat.html?name=one&mode=two".to_owned());
        let html = render_suite(&report).expect("safe relative URL should render");
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(html.contains("flat.html?name=one&amp;mode=two"));
    }

    #[test]
    fn renderer_version_is_explicit_and_report_has_no_embedded_capture_json() {
        let html = render_flight(&base_flight()).expect("fixture report should render");
        assert_eq!(RENDERER_VERSION, "waypoint-v2-html-svg-1");
        assert!(!html.contains("application/json"));
        assert!(!html.contains("__REPORT_DATA__"));
    }
}
