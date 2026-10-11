//! Presentation of measured input populations. Callers own verification,
//! statistics and preview selection; this module knows no generator or agent.
use std::{collections::BTreeMap, fmt::Write as _};

use pd_core::{LandingPadSpec, Vec2};
use serde::{Deserialize, Serialize};

use crate::batch::{BatchPage, BatchTreeOptions, escape_html, render_review_first_batch_page};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributionView {
    pub key: String,
    pub label: String,
    pub bins: Vec<String>,
    pub proposed: Vec<usize>,
    pub admitted: Vec<usize>,
    pub admitted_right: Vec<usize>,
    pub admitted_left: Vec<usize>,
    /// Already-formatted physical-unit summaries, not inferred from the bins.
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JointView {
    pub label: String,
    pub x_bins: Vec<String>,
    pub y_bins: Vec<String>,
    pub proposed: Vec<Vec<usize>>,
    pub admitted: Vec<Vec<usize>>,
    pub correlation_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QaSetView {
    pub label: String,
    pub count: usize,
    pub rightward: usize,
    pub leftward: usize,
    pub raw_error_flags: usize,
    pub barrier_flags: usize,
    pub width_flags: usize,
    pub admission_flips: usize,
    pub maximum_raw_error_m: f64,
    pub flagged_inputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputCollectionView {
    pub id: String,
    pub label: String,
    pub scheduled: usize,
    pub proposed: usize,
    pub admitted: usize,
    pub dispositions: BTreeMap<String, usize>,
    pub distributions: Vec<DistributionView>,
    pub joints: Vec<JointView>,
    pub qa_sets: Vec<QaSetView>,
    pub scalar_rows: Vec<[String; 2]>,
    pub provenance_rows: Vec<[String; 2]>,
    pub concentration_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputBandView {
    pub collection: String,
    pub group: String,
    pub band: String,
    pub label: String,
    /// Full-population counts; deliberately independent of bounded previews.
    pub proposed: usize,
    pub admitted: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputPreviewView {
    pub collection: String,
    pub group: String,
    pub band: String,
    pub index: u64,
    pub direction: i32,
    pub physical_sha256: String,
    pub disposition: String,
    pub metrics: String,
    pub preview_href: String,
    pub detail_href: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputCoveragePage {
    pub title: String,
    pub subtitle: String,
    pub collections: Vec<InputCollectionView>,
    pub bands: Vec<InputBandView>,
    pub previews: Vec<InputPreviewView>,
    pub findings: Vec<String>,
    pub limits: Vec<String>,
    pub receipt_href: String,
}

fn safe_relative_href(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains([':', '\\', '?', '#'])
        && value
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
}

impl InputCoveragePage {
    pub fn validate(&self) -> Result<(), String> {
        if self.collections.is_empty()
            || self.collections.len() > 2
            || self.previews.len() > 384
            || self.bands.len() > 72
            || !safe_relative_href(&self.receipt_href)
        {
            return Err("unbounded input coverage projection or unsafe receipt link".into());
        }
        for (i, c) in self.collections.iter().enumerate() {
            if c.id.is_empty()
                || self.collections[..i].iter().any(|p| p.id == c.id)
                || c.scheduled > 20000
                || c.admitted > c.proposed
                || c.proposed > c.scheduled
                || c.dispositions.values().any(|&n| n > c.scheduled)
                || c.dispositions.values().sum::<usize>() != c.scheduled
                || c.distributions.is_empty()
                || c.distributions.len() > 10
                || c.joints.len() > 20
                || c.qa_sets.len() > 2
            {
                return Err("input coverage collection counts or bounds disagree".into());
            }
            for q in &c.qa_sets {
                if q.count > 1024
                    || q.rightward > q.count
                    || q.leftward > q.count
                    || q.rightward + q.leftward != q.count
                    || [
                        q.raw_error_flags,
                        q.barrier_flags,
                        q.width_flags,
                        q.admission_flips,
                        q.flagged_inputs.len(),
                    ]
                    .iter()
                    .any(|&n| n > q.count)
                    || !q.maximum_raw_error_m.is_finite()
                    || q.maximum_raw_error_m < 0.0
                {
                    return Err("resolution QA projection counts disagree".into());
                }
            }
            for d in &c.distributions {
                if d.bins.is_empty()
                    || d.bins.len() > 22
                    || d.proposed.len() != d.bins.len()
                    || d.admitted.len() != d.bins.len()
                    || d.admitted_right.len() != d.bins.len()
                    || d.admitted_left.len() != d.bins.len()
                    || d.proposed
                        .iter()
                        .chain(&d.admitted)
                        .chain(&d.admitted_right)
                        .chain(&d.admitted_left)
                        .any(|&n| n > c.scheduled)
                    || d.admitted
                        .iter()
                        .enumerate()
                        .any(|(i, n)| *n != d.admitted_right[i] + d.admitted_left[i])
                    || d.proposed.iter().sum::<usize>() != c.proposed
                    || d.admitted.iter().sum::<usize>() != c.admitted
                {
                    return Err("input distribution denominator disagrees".into());
                }
            }
            for j in &c.joints {
                for (matrix, count) in [(&j.proposed, c.proposed), (&j.admitted, c.admitted)] {
                    if j.x_bins.is_empty()
                        || j.y_bins.is_empty()
                        || j.x_bins.len() > 22
                        || j.y_bins.len() > 22
                        || matrix.len() != j.x_bins.len()
                        || matrix.iter().any(|r| r.len() != j.y_bins.len())
                        || matrix.iter().flatten().any(|&n| n > c.scheduled)
                        || matrix.iter().flatten().sum::<usize>() != count
                    {
                        return Err("input joint denominator disagrees".into());
                    }
                }
            }
        }
        let first = &self.collections[0];
        for c in &self.collections {
            if c.distributions.len() != first.distributions.len()
                || c.distributions.iter().any(|d| {
                    !first
                        .distributions
                        .iter()
                        .any(|f| f.key == d.key && f.bins == d.bins)
                })
            {
                return Err("backend charts require the same declared axes and bins".into());
            }
            let bands: Vec<_> = self.bands.iter().filter(|b| b.collection == c.id).collect();
            if bands
                .iter()
                .any(|b| b.proposed > c.proposed || b.admitted > c.admitted)
                || bands.iter().map(|b| b.proposed).sum::<usize>() != c.proposed
                || bands.iter().map(|b| b.admitted).sum::<usize>() != c.admitted
                || self
                    .previews
                    .iter()
                    .filter(|p| p.collection == c.id)
                    .count()
                    > 192
            {
                return Err("review band partition or preview bound disagrees".into());
            }
        }
        for (i, b) in self.bands.iter().enumerate() {
            if !self.collections.iter().any(|c| c.id == b.collection)
                || b.admitted > b.proposed
                || self.bands[..i].iter().any(|p| {
                    (&p.collection, &p.group, &p.band) == (&b.collection, &b.group, &b.band)
                })
            {
                return Err("input review band is ambiguous".into());
            }
        }
        for (i, p) in self.previews.iter().enumerate() {
            if ![-1, 1].contains(&p.direction)
                || !safe_relative_href(&p.preview_href)
                || !safe_relative_href(&p.detail_href)
                || p.physical_sha256.len() != 64
                || !p.physical_sha256.bytes().all(|v| v.is_ascii_hexdigit())
                || !self.bands.iter().any(|b| {
                    (&b.collection, &b.group, &b.band) == (&p.collection, &p.group, &p.band)
                })
                || self.previews[..i]
                    .iter()
                    .any(|q| q.collection == p.collection && q.index == p.index)
            {
                return Err("input preview identity, membership or link is invalid".into());
            }
        }
        Ok(())
    }
}

/// Full common report shell with bounded, lazy preview browsing. No flight
/// outcomes are manufactured for an input-only review.
pub fn render_input_coverage_page(page: &InputCoveragePage) -> Result<String, String> {
    page.validate()?;
    let e = escape_html;
    let scheduled: usize = page.collections.iter().map(|c| c.scheduled).sum();
    let admitted: usize = page.collections.iter().map(|c| c.admitted).sum();
    let qa: usize = page
        .collections
        .iter()
        .flat_map(|c| &c.qa_sets)
        .map(|q| q.count)
        .sum();
    let overview = format!(
        "<section class=\"panel\"><h2>Input calibration</h2><div class=\"input-kpis\"><div><strong>{scheduled}</strong>scheduled proposals</div><div><strong>{admitted}</strong>admitted terrain challenges</div><div><strong>{qa}</strong>finer-resolution inspections</div><div><strong>{}</strong>selected previews</div></div><ul>{}</ul><p class=\"section-note\">Every scheduled proposal is accounted for. No missions were flown. Admission measures declared geometry, not solvability.</p></section>",
        page.previews.len(),
        page.findings
            .iter()
            .map(|s| format!("<li>{}</li>", e(s)))
            .collect::<String>()
    );
    let mut distributions = String::new();
    let keys: Vec<_> = page.collections[0]
        .distributions
        .iter()
        .map(|d| &d.key)
        .collect();
    for key in keys {
        let matching: Vec<_> = page
            .collections
            .iter()
            .filter_map(|c| {
                c.distributions
                    .iter()
                    .find(|d| &d.key == key)
                    .map(|d| (c, d))
            })
            .collect();
        let max = matching
            .iter()
            .flat_map(|(_, d)| d.proposed.iter().chain(&d.admitted))
            .copied()
            .max()
            .unwrap_or(1)
            .max(1);
        let mut panels = String::new();
        for (c, d) in matching {
            let directions: Vec<_> = d
                .bins
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    [
                        b.clone(),
                        format!(
                            "{} right / {} left",
                            d.admitted_right[i], d.admitted_left[i]
                        ),
                    ]
                })
                .collect();
            write!(
                panels,
                "<div class=\"input-chart\"><h4>{}</h4>{}<p class=\"section-note\">{}</p><details><summary>Direction within admitted bins</summary>{}</details></div>",
                e(&c.label),
                histogram(d, max),
                e(&d.summary), pairs_table(&directions)
            )
            .unwrap();
        }
        let label = page.collections[0]
            .distributions
            .iter()
            .find(|d| &d.key == key)
            .unwrap();
        let open = if [
            "distance",
            "signed_rise",
            "chord_barrier",
            "sustained_barrier_width",
        ]
        .contains(&key.as_str())
        {
            " open"
        } else {
            ""
        };
        write!(distributions, "<details class=\"input-distribution\"{open}><summary>{}</summary><div class=\"input-chart-grid\">{panels}</div></details>", e(&label.label)).unwrap();
    }
    let mut joints = String::new();
    for c in &page.collections {
        for j in &c.joints {
            let max = j
                .proposed
                .iter()
                .flatten()
                .copied()
                .max()
                .unwrap_or(1)
                .max(1);
            write!(joints, "<details class=\"input-joint\"><summary>{} · {}</summary><p>{}</p><div class=\"input-chart-grid\">{}{}</div></details>", e(&c.label), e(&j.label), e(&j.correlation_note), heatmap(j, false, max), heatmap(j, true, max)).unwrap();
        }
    }
    let coverage = format!(
        "<section id=\"input-coverage\" class=\"panel\"><h2>Measured coverage</h2><p class=\"section-note\">Gray: all resolved proposals. Teal: admitted inputs. Shared count scales within each axis; all bins include explicit overflow and missing values.</p>{distributions}<h3>Selected joint interactions</h3><p class=\"section-note\">Open a pair to inspect exact counts, including empty and missing cells. Occupancy describes these declared bins; it does not establish complete coverage.</p>{joints}</section>"
    );
    let json = serde_json::to_string(page)
        .map_err(|e| e.to_string())?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    if json.len() > 2 * 1024 * 1024 {
        return Err("input report browser projection exceeds 2 MiB".into());
    }
    let review = format!(
        "<section id=\"input-review\" class=\"panel\"><h2>Review terrain inputs</h2><p class=\"section-note\">Collection → measured morphology → chord-barrier band → mission. Group counts cover every input; previews are a deterministic, bounded selection that includes excluded proposals.</p><div class=\"paired-controls\"><label>Physical direction<select id=\"input-direction\"><option value=\"all\">Both directions</option><option value=\"1\">Rightward</option><option value=\"-1\">Leftward</option></select></label><label>Find in selected previews<input type=\"search\" id=\"input-search\" placeholder=\"Index or physical identity\"></label><button id=\"input-collapse\" type=\"button\">Close preview band</button></div><div id=\"input-hierarchy\"></div><p id=\"input-review-count\" aria-live=\"polite\"></p><nav id=\"input-pager\" aria-label=\"Selected preview pages\"></nav><div id=\"input-preview-cards\"></div><script type=\"application/json\" id=\"input-coverage-data\">{json}</script><script>{}</script></section>",
        include_str!("input_coverage.js")
    );
    let mut diagnostics = String::new();
    let mut context = String::new();
    for c in &page.collections {
        let rows = c.qa_sets.iter().map(|q| format!("<tr><th>{}</th><td>{}</td><td>{}/{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2} m</td></tr>", e(&q.label), q.count, q.rightward, q.leftward, q.raw_error_flags, q.barrier_flags, q.width_flags, q.admission_flips, q.maximum_raw_error_m)).collect::<String>();
        write!(diagnostics, "<h3>{}</h3><div class=\"table-wrap\"><table class=\"context-table\"><thead><tr><th>QA selection</th><th>Inputs</th><th>Right/left</th><th>Raw error flags</th><th>Barrier flags</th><th>Width flags</th><th>Admission flips</th><th>Max raw error</th></tr></thead><tbody>{rows}</tbody></table></div><p>{}</p>", e(&c.label), e(&c.concentration_note)).unwrap();
        for q in &c.qa_sets {
            if !q.flagged_inputs.is_empty() {
                write!(diagnostics,"<details><summary>{} · flagged saved input indices</summary><ul>{}</ul><p class=\"section-note\">These indices identify saved proposals in the source audit. Preview selection remains the predeclared bounded inventory; flagged inputs do not automatically add images.</p></details>",e(&q.label),q.flagged_inputs.iter().map(|s|format!("<li>{}</li>",e(s))).collect::<String>()).unwrap();
            }
        }
        write!(context, "<details><summary>{} · complete geometry summaries</summary>{}</details><details><summary>{} · selection and provenance</summary>{}{}</details>", e(&c.label), pairs_table(&c.scalar_rows), e(&c.label), pairs_table(&c.dispositions.iter().map(|(k, v)| [k.clone(), v.to_string()]).collect::<Vec<_>>()), pairs_table(&c.provenance_rows)).unwrap();
    }
    let diagnostics = format!(
        "<section class=\"panel\"><h2>Resolution and concentration diagnostics</h2><p class=\"section-note\">Regular index sampling and targeted admission-boundary sampling have separate denominators. Targeted flags are not a population error rate. The 1 m reference is a counterfactual resampling; the sealed 4 m polyline remains the physical input.</p>{diagnostics}</section>"
    );
    let context = format!(
        "<section class=\"panel\"><h2>Input contracts and limits</h2><ul>{}</ul>{context}<p><a href=\"{}\">Projection receipt and source bindings</a></p></section>",
        page.limits
            .iter()
            .map(|s| format!("<li>{}</li>", e(s)))
            .collect::<String>(),
        e(&page.receipt_href)
    );
    let mut html = render_review_first_batch_page(BatchPage {
        title: &page.title,
        subtitle: &page.subtitle,
        chips_html: "<span class=\"chip\">V2 inputs</span><span class=\"chip\">Exposed development calibration</span><span class=\"chip\">0 flights / training</span>",
        actions_html: "<a href=\"#input-coverage\">Coverage</a><a href=\"#input-review\">Browse inputs</a>",
        before_hero_html: "",
        after_hero_html: "",
        overview_html: &overview,
        planner_html: &coverage,
        coverage_html: "",
        context_html: &context,
        diagnostics_html: &diagnostics,
        review_tree_html: &review,
        comparison_html: "",
        appendix_html: "",
        body_class: "input-coverage-report",
        tree: BatchTreeOptions {
            max_depth: 0,
            depth_by_kind: &[],
            leaf_selector: ".input-no-tree",
            default_expansion: 0,
        },
        coverage_tree_jump: false,
    });
    html = html.replacen(
        "</style>",
        &format!("{}\n</style>", include_str!("input_coverage.css")),
        1,
    );
    Ok(html)
}

fn histogram(d: &DistributionView, maximum: usize) -> String {
    let width = 600.0;
    let slot = width / d.bins.len() as f64;
    let mut bars = String::new();
    for (i, bin) in d.bins.iter().enumerate() {
        let x = i as f64 * slot;
        let hp = d.proposed[i] as f64 / maximum as f64 * 90.0;
        let ha = d.admitted[i] as f64 / maximum as f64 * 90.0;
        write!(bars, "<g><title>{}: proposed {}, admitted {}</title><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{hp:.1}\" fill=\"#b8b0a4\"/><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{ha:.1}\" fill=\"#356c69\"/><text x=\"{:.1}\" y=\"112\" text-anchor=\"middle\">{}</text><text x=\"{:.1}\" y=\"132\" text-anchor=\"middle\">{} / {}</text></g>", escape_html(bin), d.proposed[i], d.admitted[i], x+5.0, 95.0-hp, slot-10.0, x+slot*0.3, 95.0-ha, slot*0.4, x+slot/2.0, escape_html(bin), x+slot/2.0, d.proposed[i], d.admitted[i]).unwrap();
    }
    format!(
        "<svg role=\"img\" aria-label=\"{} bin counts\" viewBox=\"0 0 600 145\" class=\"input-histogram\"><line x1=\"0\" y1=\"95\" x2=\"600\" y2=\"95\" stroke=\"#b8b0a4\"/>{bars}</svg>",
        escape_html(&d.label)
    )
}

fn heatmap(j: &JointView, admitted: bool, maximum: usize) -> String {
    let matrix = if admitted { &j.admitted } else { &j.proposed };
    let mut rows = String::new();
    for (i, name) in j.x_bins.iter().enumerate() {
        write!(rows, "<tr><th>{}</th>", escape_html(name)).unwrap();
        for count in &matrix[i] {
            let opacity = *count as f64 / maximum as f64 * 0.5;
            write!(
                rows,
                "<td style=\"background:rgba(53,108,105,{opacity:.3})\">{count}</td>"
            )
            .unwrap();
        }
        rows.push_str("</tr>");
    }
    format!(
        "<div><h4>{}</h4><div class=\"table-wrap\"><table class=\"context-table input-heatmap\"><thead><tr><th>First axis ↓ / second →</th>{}</tr></thead><tbody>{rows}</tbody></table></div></div>",
        if admitted {
            "Admitted inputs"
        } else {
            "Resolved proposals"
        },
        j.y_bins
            .iter()
            .map(|s| format!("<th>{}</th>", escape_html(s)))
            .collect::<String>()
    )
}

fn pairs_table(rows: &[[String; 2]]) -> String {
    format!(
        "<div class=\"table-wrap\"><table class=\"context-table\"><tbody>{}</tbody></table></div>",
        rows.iter()
            .map(|r| format!(
                "<tr><th>{}</th><td>{}</td></tr>",
                escape_html(&r[0]),
                escape_html(&r[1])
            ))
            .collect::<String>()
    )
}

/// Shared source-relative metre axes for an entire selected preview inventory.
/// Display reflection changes neither the saved physical input nor its identity.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct InputPlotBounds {
    pub min_x_m: f64,
    pub max_x_m: f64,
    pub min_y_m: f64,
    pub max_y_m: f64,
}

pub struct InputTerrainPlot<'a> {
    pub raw: &'a [Vec2],
    pub prepared: &'a [Vec2],
    pub source: &'a LandingPadSpec,
    pub target: &'a LandingPadSpec,
    pub edit_intervals_x_m: &'a [[f64; 2]],
    pub bounds: InputPlotBounds,
}

/// Explicit geometry plot, with no flight, route, outcome or idealized arc.
pub fn render_input_terrain_svg(plot: InputTerrainPlot<'_>) -> Result<String, String> {
    let b = plot.bounds;
    if ![b.min_x_m, b.max_x_m, b.min_y_m, b.max_y_m]
        .iter()
        .all(|v| v.is_finite())
        || b.min_x_m >= b.max_x_m
        || b.min_y_m >= b.max_y_m
        || plot.raw.is_empty()
        || plot.prepared.is_empty()
        || plot.raw.len() > 100001
        || plot.prepared.len() > 100001
        || plot.edit_intervals_x_m.len() > 4
    {
        return Err("input terrain plot has invalid or unbounded geometry".into());
    }
    plot.source.validate()?;
    plot.target.validate()?;
    let sign = (plot.target.center_x_m - plot.source.center_x_m).signum();
    if sign == 0.0 {
        return Err("input terrain plot requires distinct pads".into());
    }
    let relative = |p: Vec2| Vec2 {
        x: (p.x - plot.source.center_x_m) * sign,
        y: p.y - plot.source.surface_y_m,
    };
    let px = |x: f64| 56.0 + (x - b.min_x_m) / (b.max_x_m - b.min_x_m) * 570.0;
    let py = |y: f64| 260.0 - (y - b.min_y_m) / (b.max_y_m - b.min_y_m) * 230.0;
    let line = |points: &[Vec2]| -> Result<String, String> {
        if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err("nonfinite input plot point".into());
        }
        if points.windows(2).any(|p| p[0].x >= p[1].x) {
            return Err("input plot requires ordered physical polyline points".into());
        }
        let mut points: Vec<_> = points.iter().copied().map(relative).collect();
        if sign < 0.0 {
            points.reverse();
        }
        if points
            .iter()
            .any(|p| p.x < b.min_x_m || p.x > b.max_x_m || p.y < b.min_y_m || p.y > b.max_y_m)
        {
            return Err("shared plot axes would hide saved terrain".into());
        }
        Ok(points
            .iter()
            .map(|p| format!("{:.2},{:.2}", px(p.x), py(p.y)))
            .collect::<Vec<_>>()
            .join(" "))
    };
    let mut grid = String::new();
    for i in 0..=4 {
        let x = b.min_x_m + (b.max_x_m - b.min_x_m) * i as f64 / 4.0;
        let y = b.min_y_m + (b.max_y_m - b.min_y_m) * i as f64 / 4.0;
        write!(grid,"<line x1=\"{:.2}\" y1=\"30\" x2=\"{:.2}\" y2=\"260\" stroke=\"#e4dccf\"/><text x=\"{:.2}\" y=\"279\" text-anchor=\"middle\">{x:.0}</text><line x1=\"56\" y1=\"{:.2}\" x2=\"626\" y2=\"{:.2}\" stroke=\"#e4dccf\"/><text x=\"48\" y=\"{:.2}\" text-anchor=\"end\">{y:.0}</text>",px(x),px(x),px(x),py(y),py(y),py(y)+4.0).unwrap();
    }
    let mut edits = String::new();
    for interval in plot.edit_intervals_x_m {
        if !interval.iter().all(|v| v.is_finite()) || interval[0] > interval[1] {
            return Err("invalid input plot edit interval".into());
        }
        let mut x = interval.map(|v| (v - plot.source.center_x_m) * sign);
        x.sort_by(f64::total_cmp);
        write!(edits,"<rect x=\"{:.2}\" y=\"30\" width=\"{:.2}\" height=\"230\" fill=\"#bd9140\" opacity=\".16\"/>",px(x[0]),px(x[1])-px(x[0])).unwrap();
    }
    let target = relative(Vec2 {
        x: plot.target.center_x_m,
        y: plot.target.surface_y_m,
    });
    let mut pads = String::new();
    for (pad, label) in [(plot.source, "Source"), (plot.target, "Target")] {
        let p = relative(Vec2 {
            x: pad.center_x_m,
            y: pad.surface_y_m,
        });
        write!(pads,"<g><title>{label}: width {:.1} m</title><line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#aa653b\" stroke-width=\"4\"/><circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"3\" fill=\"#aa653b\"/><text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\">{label}</text></g>",pad.width_m,px(p.x-pad.width_m/2.0),py(p.y),px(p.x+pad.width_m/2.0),py(p.y),px(p.x),py(p.y),px(p.x),py(p.y)-10.0).unwrap();
    }
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 640 300\" role=\"img\" aria-label=\"Raw and prepared mission terrain, source-relative shared metre axes\"><rect width=\"640\" height=\"300\" fill=\"#fbf7ee\"/><style>text{{font:11px ui-monospace,monospace;fill:#675847}}</style><defs><clipPath id=\"input-plot-clip\"><rect x=\"56\" y=\"30\" width=\"570\" height=\"230\"/></clipPath></defs>{grid}<g clip-path=\"url(#input-plot-clip)\">{edits}<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#aa9b88\" stroke-dasharray=\"4 4\"/><polyline points=\"{}\" fill=\"none\" stroke=\"#9c9388\" stroke-width=\"3\"/><polyline points=\"{}\" fill=\"none\" stroke=\"#356c69\" stroke-width=\"1.5\"/>{pads}</g><text x=\"320\" y=\"14\" text-anchor=\"middle\">Gray raw · teal prepared · amber pad edits · dashed pad chord</text><text x=\"340\" y=\"297\" text-anchor=\"middle\">Progress from source (m) · height relative to source (m)</text></svg>",
        px(0.0),
        py(0.0),
        px(target.x),
        py(target.y),
        line(plot.raw)?,
        line(plot.prepared)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> InputCoveragePage {
        InputCoveragePage {
            title: "Input <review>".into(),
            subtitle: "No flights".into(),
            collections: vec![InputCollectionView {
                id: "example".into(),
                label: "Example <backend>".into(),
                scheduled: 3,
                proposed: 2,
                admitted: 1,
                dispositions: BTreeMap::from([
                    ("accepted".into(), 1),
                    ("mild".into(), 1),
                    ("unresolved".into(), 1),
                ]),
                distributions: vec![DistributionView {
                    key: "distance".into(),
                    label: "Distance (m)".into(),
                    bins: vec!["<100".into(), "≥100".into(), "Missing".into()],
                    proposed: vec![1, 1, 0],
                    admitted: vec![0, 1, 0],
                    admitted_right: vec![0, 0, 0],
                    admitted_left: vec![0, 1, 0],
                    summary: "Median 200 m".into(),
                }],
                joints: vec![JointView {
                    label: "Distance × width".into(),
                    x_bins: vec!["<100".into(), "≥100".into()],
                    y_bins: vec!["<40".into(), "≥40".into()],
                    proposed: vec![vec![1, 0], vec![0, 1]],
                    admitted: vec![vec![0, 0], vec![0, 1]],
                    correlation_note: "Missing stays visible".into(),
                }],
                qa_sets: vec![],
                scalar_rows: vec![],
                provenance_rows: vec![],
                concentration_note: "No near-neighbor claim".into(),
            }],
            bands: vec![InputBandView {
                collection: "example".into(),
                group: "mixed".into(),
                band: "b0".into(),
                label: "Barrier <100 m".into(),
                proposed: 2,
                admitted: 1,
            }],
            previews: vec![InputPreviewView {
                collection: "example".into(),
                group: "mixed".into(),
                band: "b0".into(),
                index: 1,
                direction: -1,
                physical_sha256: "a".repeat(64),
                disposition: "Admitted".into(),
                metrics: "100 m".into(),
                preview_href: "previews/1.svg".into(),
                detail_href: "details/1.html".into(),
            }],
            findings: vec!["Input-only </script> review".into()],
            limits: vec!["Selected interactions only".into()],
            receipt_href: "render.json".into(),
        }
    }

    #[test]
    fn input_page_retains_common_shell_and_no_initial_preview_requests() {
        let html = render_input_coverage_page(&fixture()).unwrap();
        assert!(html.contains("data-batch-template=\"common-v1\""));
        assert!(html.contains("Input &lt;review&gt;"));
        assert!(html.contains("\\u003c/script\\u003e"));
        assert!(!html.contains("<img src=\"previews/1.svg\""));
        assert!(
            html.find("Measured coverage").unwrap() < html.find("Review terrain inputs").unwrap()
        );
        assert!(html.contains("Targeted flags are not a population error rate"));
    }

    #[test]
    fn report_refuses_mismatched_denominators_and_unsafe_preview_links() {
        let mut p = fixture();
        p.collections[0].distributions[0].admitted[0] = 1;
        assert!(render_input_coverage_page(&p).is_err());
        let mut p = fixture();
        p.collections[0].joints[0].admitted[1][1] = 0;
        assert!(render_input_coverage_page(&p).is_err());
        let mut p = fixture();
        p.previews[0].preview_href = "javascript:alert(1)".into();
        assert!(render_input_coverage_page(&p).is_err());
        let mut p = fixture();
        p.previews[0].detail_href = "../another-report.html".into();
        assert!(render_input_coverage_page(&p).is_err());
    }

    #[test]
    fn terrain_plot_is_translation_reflection_invariant_and_never_hides_geometry() {
        let bounds = InputPlotBounds {
            min_x_m: -50.0,
            max_x_m: 150.0,
            min_y_m: -50.0,
            max_y_m: 150.0,
        };
        let points = [
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 50.0, y: 100.0 },
            Vec2 { x: 100.0, y: 0.0 },
        ];
        let source = LandingPadSpec {
            id: "s".into(),
            center_x_m: 0.0,
            surface_y_m: 0.0,
            width_m: 10.0,
        };
        let target = LandingPadSpec {
            id: "t".into(),
            center_x_m: 100.0,
            surface_y_m: 0.0,
            width_m: 10.0,
        };
        let render = |points: &[Vec2], source: &LandingPadSpec, target: &LandingPadSpec, bounds| {
            render_input_terrain_svg(InputTerrainPlot {
                raw: points,
                prepared: points,
                source,
                target,
                edit_intervals_x_m: &[],
                bounds,
            })
        };
        let svg = render(&points, &source, &target, bounds).unwrap();
        let shifted: Vec<_> = points
            .iter()
            .map(|p| Vec2 {
                x: p.x + 1000.0,
                y: p.y + 400.0,
            })
            .collect();
        let shift_pad = |pad: &LandingPadSpec| LandingPadSpec {
            center_x_m: pad.center_x_m + 1000.0,
            surface_y_m: pad.surface_y_m + 400.0,
            ..pad.clone()
        };
        assert_eq!(
            svg,
            render(&shifted, &shift_pad(&source), &shift_pad(&target), bounds).unwrap()
        );
        let mirrored: Vec<_> = points
            .iter()
            .rev()
            .map(|p| Vec2 { x: -p.x, y: p.y })
            .collect();
        let target = LandingPadSpec {
            center_x_m: -100.0,
            ..target
        };
        assert_eq!(svg, render(&mirrored, &source, &target, bounds).unwrap());
        assert!(
            render(
                &points,
                &source,
                &LandingPadSpec {
                    center_x_m: 100.0,
                    ..target
                },
                InputPlotBounds {
                    max_y_m: 90.0,
                    ..bounds
                }
            )
            .is_err()
        );
        assert!(!svg.contains("trajectory"));
    }
}
