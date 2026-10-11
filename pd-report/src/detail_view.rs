//! Display-only additions to the maintained rich flight template.
use anyhow::{Result, ensure};
use pd_core::SampleRecord;

use crate::campaign::escape;

pub fn metric_fallback(samples: &[SampleRecord]) -> String {
    let points = |value: fn(&SampleRecord) -> f64| {
        samples
            .iter()
            .map(|s| (s.sim_time_s, value(s)))
            .collect::<Vec<_>>()
    };
    format!(
        "{}{}",
        static_plot(
            "Velocity · m/s",
            &[
                ("vx", "#3568a8", points(|s| s.observation.velocity_mps.x)),
                ("vy", "#b95024", points(|s| s.observation.velocity_mps.y))
            ]
        ),
        static_plot(
            "Commanded throttle · %",
            &[(
                "throttle",
                "#467a57",
                points(|s| 100.0 * s.held_command.throttle_frac)
            )]
        )
    )
}

type StaticSeries<'a> = (&'a str, &'a str, Vec<(f64, f64)>);

fn static_plot(title: &str, series: &[StaticSeries<'_>]) -> String {
    let finite: Vec<_> = series
        .iter()
        .flat_map(|s| s.2.iter().copied())
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    if finite.is_empty() {
        return format!("<p>{}: no recorded samples</p>", escape(title));
    }
    let xmax = finite.iter().map(|(x, _)| *x).fold(1.0, f64::max);
    let ymin = finite.iter().map(|(_, y)| *y).fold(0.0, f64::min);
    let ymax = finite
        .iter()
        .map(|(_, y)| *y)
        .fold(0.0, f64::max)
        .max(ymin + 1.0);
    let mut paths = String::new();
    for (label, color, points) in series {
        let path = points
            .iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .enumerate()
            .map(|(i, (x, y))| {
                format!(
                    "{} {:.2} {:.2}",
                    if i == 0 { "M" } else { "L" },
                    45.0 + 400.0 * x / xmax,
                    150.0 - 115.0 * (y - ymin) / (ymax - ymin)
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        paths.push_str(&format!("<path d=\"{path}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"><title>{}</title></path>",escape(label)));
    }
    let legend = series
        .iter()
        .map(|(name, _, _)| *name)
        .collect::<Vec<_>>()
        .join(" / ");
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 480 190\" role=\"img\" aria-label=\"{}\"><rect width=\"480\" height=\"190\" fill=\"#fbf7ee\"/><g font-family=\"system-ui,sans-serif\" font-size=\"12\" fill=\"#51483d\"><text x=\"45\" y=\"20\">{} · {}</text><text x=\"5\" y=\"40\">{ymax:.1}</text><text x=\"5\" y=\"150\">{ymin:.1}</text><text x=\"45\" y=\"174\">0 s</text><text x=\"405\" y=\"174\">{xmax:.1} s</text></g><path d=\"M45 35V150H445\" fill=\"none\" stroke=\"#cbbda9\"/>{paths}</svg>",
        escape(title),
        escape(title),
        escape(&legend)
    )
}

/// Keeps the original report payload and every established rich inspection tool.
pub fn with_static_fallback(html: &str, svg: &str, metrics: &str, caption: &str) -> Result<String> {
    const SPATIAL: &str = "<div id=\"chart-spatial\" class=\"chart\"></div>";
    const METRICS: &str = "<div id=\"chart-metrics\" class=\"chart\"></div>";
    ensure!(
        html.matches(SPATIAL).count() == 1 && html.matches(METRICS).count() == 1,
        "saved rich detail has ambiguous chart boundaries"
    );
    let fallback = format!(
        "<div id=\"static-flight-fallback\" class=\"static-plot\">{svg}<p>{} · static view. Interactive charts are unavailable until the chart library loads.</p></div>",
        escape(caption)
    );
    let metric_fallback = format!(
        "<div id=\"static-metrics-fallback\" class=\"static-plot\">{metrics}<p>Recorded samples; commanded throttle is shown separately from velocity.</p></div>"
    );
    let old =
        "      buildSpatialPlot();\n      buildMetricsPlot();\n      initFlightCorrections();";
    let guarded = "      if (typeof Plotly !== \"undefined\") {\n        buildSpatialPlot();\n        buildMetricsPlot();\n        initFlightCorrections();\n      }";
    let spatial_count = html.matches("id=\"static-flight-fallback\"").count();
    let metric_count = html.matches("id=\"static-metrics-fallback\"").count();
    ensure!(
        (spatial_count, metric_count) == (0, 0) || (spatial_count, metric_count) == (1, 1),
        "saved rich detail has ambiguous static fallbacks"
    );
    if spatial_count == 1 {
        ensure!(
            html.matches(guarded).count() == 1 && html.matches(old).count() == 0,
            "saved rich detail has unsupported guarded initialization"
        );
        let html = replace_static_container(html, "static-flight-fallback", &fallback)?;
        return replace_static_container(&html, "static-metrics-fallback", &metric_fallback);
    }
    let html = html
        .replacen(SPATIAL, &format!("{SPATIAL}{fallback}"), 1)
        .replacen(METRICS, &format!("{METRICS}{metric_fallback}"), 1);
    ensure!(
        html.matches(old).count() == 1,
        "saved rich detail has unsupported initialization"
    );
    let html = html.replacen(old, guarded, 1);
    let css = "<style>.static-plot svg{display:block;width:100%;height:auto;max-height:460px}.static-plot p{font-size:.8rem;color:var(--muted)}.plots-unavailable .chart{display:none}.review-detail-nav{display:flex;flex-wrap:wrap;gap:12px;margin-bottom:12px;font-size:.85rem}.detail-outcome{margin:10px 0;padding:12px;border:1px solid var(--line);border-radius:10px}.detail-outcome p{margin:5px 0}.detail-outcome .outcome-badge{font-weight:700}.detail-refined .flight-annotation-banner{font-size:.78rem}.detail-refined .flight-annotation-banner details{margin-bottom:5px}.detail-refined header{margin-bottom:12px}.detail-refined main{width:100%;min-width:0}.detail-nonlanding .key-grid>.stat:first-child{display:none}</style>";
    let js = r#"<script>(()=>{const unavailable=typeof Plotly==='undefined';document.body.classList.toggle('plots-unavailable',unavailable);if(unavailable){document.querySelectorAll('[data-mode]').forEach(b=>b.disabled=true);return;}let remaining=100;const check=()=>{if(document.getElementById('chart-spatial')?._fullLayout&&document.getElementById('chart-metrics')?._fullLayout){document.getElementById('static-flight-fallback').hidden=true;document.getElementById('static-metrics-fallback').hidden=true;}else if(remaining--)setTimeout(check,50);};check();})();</script>"#;
    Ok(html
        .replacen("</head>", &format!("{css}</head>"), 1)
        .replacen("</body>", &format!("{js}</body>"), 1))
}

fn replace_static_container(html: &str, id: &str, replacement: &str) -> Result<String> {
    let opening = format!("<div id=\"{id}\" class=\"static-plot\">");
    ensure!(
        html.matches(&opening).count() == 1,
        "saved rich detail has unsupported static container"
    );
    let start = html.find(&opening).unwrap();
    let content_start = start + opening.len();
    let end = html[content_start..]
        .find("</div>")
        .map(|offset| content_start + offset)
        .ok_or_else(|| anyhow::anyhow!("saved static container is unclosed"))?;
    ensure!(
        !html[content_start..end].contains("<div"),
        "saved static container has nested or unclosed content"
    );
    Ok(format!(
        "{}{replacement}{}",
        &html[..start],
        &html[end + "</div>".len()..]
    ))
}

/// Navigation and concise endpoint summary for a create-only review edition.
pub fn with_review_context(
    html: &str,
    navigation: &str,
    outcome_html: &str,
    root_href: &str,
    mission: &str,
    approach: usize,
    landed: bool,
) -> Result<String> {
    ensure!(
        root_href.starts_with('/')
            && !root_href.starts_with("//")
            && !root_href.contains(['\\', '?', '#', ':'])
            && root_href.ends_with('/'),
        "unsafe review root"
    );
    ensure!(
        mission.starts_with("mission-") && mission[8..].chars().all(|c| c.is_ascii_digit()),
        "unsafe mission identity"
    );
    let main = if html.contains("<main>") {
        "<main>"
    } else {
        "<div id=\"report-page\" class=\"page\" data-batch-template=\"common-v1\">"
    };
    ensure!(
        html.matches(main).count() == 1,
        "saved detail has ambiguous main boundary"
    );
    let html=html.replacen("<body>",&format!("<body class=\"detail-refined {}\" data-review-root=\"{}\" data-review-mission=\"{mission}\" data-review-approach=\"{approach}\">",if landed{""}else{"detail-nonlanding"},escape(root_href)),1).replacen(main,&format!("{main}{navigation}<section class=\"detail-outcome\">{outcome_html}</section>"),1);
    let html=html.replacen("<div class=\"flight-annotation-banner\"><p>","<div class=\"flight-annotation-banner\"><details><summary>Saved capture provenance</summary><p>",1);
    let html = if html.contains("<summary>Saved capture provenance</summary>") {
        html.replacen(
            "</p><nav aria-label=\"Flight report navigation\">",
            "</p></details><nav aria-label=\"Flight report navigation\">",
            1,
        )
    } else {
        html
    };
    let html = if html.contains("<div class=\"hero\">")
        && html.contains("      </div>\n    </header>")
    {
        html.replacen("<div class=\"hero\">", "<details class=\"saved-run-summary\"><summary>Run summary, phases and source configuration</summary><div class=\"hero\">",1).replacen("      </div>\n    </header>", "      </div></details>\n    </header>",1)
    } else {
        html
    };
    let script = format!(
        "<script>{}{}</script>",
        include_str!("paired_model.js"),
        include_str!("detail_navigation.js")
    );
    Ok(html.replacen("</head>", &format!("<style>{}.detail-refined .status-page svg{{width:min(600px,100%);height:auto}}.detail-outcome .row-note{{display:block;margin-top:4px;font-size:.85rem;color:var(--muted)}}.review-detail-nav a{{color:var(--accent)}}.saved-run-summary{{margin-bottom:14px}}.saved-run-summary>summary{{cursor:pointer;padding:10px 0;font-size:.85rem}}.detail-outcome h1{{font-size:1.25rem;margin:0 0 8px}}</style></head>",crate::campaign_view::STYLE),1).replacen("</body>", &format!("{script}</body>"), 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_rejects_ambiguous_templates_and_preserves_payload() {
        let source = "<html><head></head><body><main><div id=\"chart-spatial\" class=\"chart\"></div><div id=\"chart-metrics\" class=\"chart\"></div></main><script>const reportData = {\"untouched\":true};\n      buildSpatialPlot();\n      buildMetricsPlot();\n      initFlightCorrections();</script></body></html>";
        let html = with_static_fallback(source, "<svg/>", "<svg/>", "saved <flight>").unwrap();
        assert!(html.contains("const reportData = {\"untouched\":true};"));
        assert!(html.contains("typeof Plotly !== \"undefined\""));
        assert!(html.contains("saved &lt;flight&gt;"));
        assert_eq!(
            with_static_fallback(&html, "<svg/>", "<svg/>", "saved <flight>").unwrap(),
            html
        );
        let updated = with_static_fallback(
            &html,
            "<svg id=\"shared-world\"/>",
            "<svg id=\"updated-metrics\"/>",
            "shared bounds",
        )
        .unwrap();
        assert!(updated.contains("const reportData = {\"untouched\":true};"));
        assert!(updated.contains("<svg id=\"shared-world\"/>"));
        assert!(updated.contains("<svg id=\"updated-metrics\"/>"));
        assert!(!updated.contains("saved &lt;flight&gt;"));
        assert_eq!(updated.matches("id=\"static-flight-fallback\"").count(), 1);
        assert_eq!(updated.matches("id=\"static-metrics-fallback\"").count(), 1);
        assert_eq!(
            updated.matches("typeof Plotly !== \"undefined\"").count(),
            1
        );
        assert_eq!(updated.matches(".static-plot svg{").count(), 1);
        assert_eq!(updated.matches("let remaining=100;").count(), 1);
        for malformed in [
            html.replace("static-metrics-fallback", "missing-metrics"),
            html.replacen("<svg/>", "<div>nested</div>", 1),
            format!("{html}<div id=\"static-flight-fallback\" class=\"static-plot\"></div>"),
        ] {
            assert!(with_static_fallback(&malformed, "", "", "").is_err());
        }
        assert!(with_static_fallback(&(source.to_string() + source), "", "", "").is_err());
        assert!(with_review_context(&html, "", "", "//outside/", "mission-0", 0, false).is_err());
    }
}
