//! Additive, display-only annotations for executed flight handoffs.
//! This is deliberately independent of evaluator and planner contracts.

use anyhow::{Result, ensure};
use pd_core::{RunManifest, Vec2};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightBoundary {
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub fuel_kg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutedCorrection {
    pub number: usize,
    pub entry: FlightBoundary,
    pub handoff: FlightBoundary,
    pub reason: String,
    pub after_handoff: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NavigationLink {
    pub label: String,
    pub href: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationNavigation {
    pub home: Option<NavigationLink>,
    pub collection: Option<NavigationLink>,
    pub previous: Option<NavigationLink>,
    pub next: Option<NavigationLink>,
    pub source_links: Vec<NavigationLink>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightAnnotations {
    pub caption: String,
    pub corrections: Vec<ExecutedCorrection>,
    pub navigation: AnnotationNavigation,
}

impl FlightAnnotations {
    pub fn is_empty(&self) -> bool {
        self.caption.is_empty()
            && self.corrections.is_empty()
            && self.navigation == AnnotationNavigation::default()
    }

    pub(crate) fn validate(&self, manifest: &RunManifest) -> Result<()> {
        for link in self
            .navigation
            .home
            .iter()
            .chain(self.navigation.collection.iter())
            .chain(self.navigation.previous.iter())
            .chain(self.navigation.next.iter())
            .chain(self.navigation.source_links.iter())
        {
            ensure!(safe_href(&link.href), "unsafe annotation navigation link");
        }
        let mut previous_end = 0;
        for (i, correction) in self.corrections.iter().enumerate() {
            ensure!(correction.number == i + 1, "nonconsecutive handoff number");
            for state in [&correction.entry, &correction.handoff] {
                ensure!(
                    manifest.physics_hz > 0
                        && [
                            state.sim_time_s,
                            state.position_m.x,
                            state.position_m.y,
                            state.velocity_mps.x,
                            state.velocity_mps.y,
                            state.attitude_rad,
                            state.fuel_kg
                        ]
                        .iter()
                        .all(|v| v.is_finite())
                        && state.sim_time_s >= 0.0
                        && state.fuel_kg >= 0.0
                        && state.physics_step <= manifest.physics_steps,
                    "invalid flight annotation state"
                );
                ensure!(
                    (state.sim_time_s - state.physics_step as f64 / f64::from(manifest.physics_hz))
                        .abs()
                        < 1e-8,
                    "annotation clock disagrees with physics step"
                );
            }
            ensure!(
                correction.entry.physics_step >= previous_end
                    && correction.entry.physics_step < correction.handoff.physics_step,
                "overlapping or empty executed correction"
            );
            previous_end = correction.handoff.physics_step;
        }
        Ok(())
    }

    pub(crate) fn banner_html(&self) -> String {
        let mut links = String::new();
        for link in self
            .navigation
            .home
            .iter()
            .chain(self.navigation.collection.iter())
            .chain(self.navigation.previous.iter())
            .chain(self.navigation.next.iter())
        {
            links.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                super::escape_html(&link.href),
                super::escape_html(&link.label)
            ));
        }
        format!(
            "<div class=\"flight-annotation-banner\"><p>{}</p><nav aria-label=\"Preview report navigation\">{links}</nav></div>",
            super::escape_html(&self.caption)
        )
    }

    pub(crate) fn panel_html(&self) -> String {
        if self.corrections.is_empty() {
            return String::new();
        }
        let mut rows = String::new();
        for (i, c) in self.corrections.iter().enumerate() {
            let h = &c.handoff;
            rows.push_str(&format!(
                "<div class=\"flight-correction\" data-flight-correction=\"{i}\"><button type=\"button\" data-select-correction=\"{i}\" aria-pressed=\"{}\">H{} · {:.6} s</button><p>{}</p><p>{}</p><details><summary>Exact handoff state and correction entry</summary><dl><dt>Handoff step / time</dt><dd>{} / {} s</dd><dt>Position</dt><dd>({}, {}) m</dd><dt>Velocity</dt><dd>({}, {}) m/s</dd><dt>Attitude / fuel</dt><dd>{} rad / {} kg</dd><dt>Correction entry</dt><dd>step {} at {} s</dd></dl></details></div>",
                i == 0, c.number, h.sim_time_s,
                super::escape_html(&c.reason), super::escape_html(&c.after_handoff),
                h.physics_step, h.sim_time_s, h.position_m.x, h.position_m.y,
                h.velocity_mps.x, h.velocity_mps.y, h.attitude_rad, h.fuel_kg,
                c.entry.physics_step, c.entry.sim_time_s
            ));
        }
        let links = self
            .navigation
            .source_links
            .iter()
            .map(|link| {
                format!(
                    "<li><a href=\"{}\">{}</a></li>",
                    super::escape_html(&link.href),
                    super::escape_html(&link.label)
                )
            })
            .collect::<String>();
        format!(
            "<section class=\"panel\" id=\"flight-corrections-panel\"><div class=\"panel-head\"><div><div class=\"eyebrow\">Planner</div><h2>Waypoint corrections</h2></div></div><p>Waypoint handoff: replan from actual state.</p><p class=\"flight-reference-note\">H marks executed handoffs. Existing reference overlays are unchanged, not a reconstruction of the rejected proposal.</p><label class=\"flight-overlay-toggle\"><input type=\"checkbox\" id=\"flight-handoffs-visible\" checked> Show handoffs on both plots</label>{rows}<details><summary>Original capture and rendering provenance</summary><ul>{links}</ul></details></section>"
        )
    }
}

fn safe_href(href: &str) -> bool {
    if href.is_empty()
        || href.trim() != href
        || href.chars().any(char::is_control)
        || href.contains('\\')
        || href.starts_with("//")
    {
        return false;
    }
    let lower = href.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || !href
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .contains(':')
}

pub(crate) const CSS: &str = r#"
.flight-annotation-banner { margin-bottom: 14px; padding: 10px 12px; border: 1px solid var(--line); border-radius: 10px; background: var(--paper); }
.flight-annotation-banner p { margin: 0 0 8px; font-size: 0.85rem; }
.flight-annotation-banner nav { display: flex; gap: 10px; flex-wrap: wrap; font-size: 0.82rem; }
.flight-overlay-toggle { display: block; margin: 12px 0; font-size: 0.85rem; }
.flight-reference-note { font-size: 0.8rem; line-height: 1.4; color: var(--muted); }
.flight-correction { border-top: 1px solid var(--line); padding: 12px 0; }
.flight-correction button { border: 1px solid var(--line); border-radius: 8px; padding: 6px 10px; font: inherit; cursor: pointer; background: var(--paper); }
.flight-correction button[aria-pressed="true"] { border-color: #b45309; background: #fff3df; }
.flight-correction button:focus-visible { outline: 3px solid #315f86; outline-offset: 2px; }
.flight-correction p { margin: 8px 0; font-size: 0.86rem; line-height: 1.5; }
.flight-correction details { font-size: 0.8rem; overflow-wrap: anywhere; }
.flight-correction dt { color: var(--muted); margin-top: 8px; }
.flight-correction dd { margin-left: 0; }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_annotations_and_link_escaping_are_explicit() {
        assert!(FlightAnnotations::default().is_empty());
        let mut annotations = FlightAnnotations {
            caption: "<script>".into(),
            ..Default::default()
        };
        annotations.navigation.home = Some(NavigationLink {
            label: "<home>".into(),
            href: "/reports/?x=1&y=2".into(),
        });
        assert!(!annotations.is_empty());
        let html = annotations.banner_html();
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;home&gt;"));
        assert!(html.contains("&amp;y=2"));
        assert!(annotations.panel_html().is_empty());
    }

    #[test]
    fn annotation_links_reject_active_and_ambiguous_urls() {
        for href in [
            "javascript:alert(1)",
            "data:text/html,hi",
            " //example.org",
            "//example.org",
            "\\evil",
            "/reports/\n",
        ] {
            assert!(!safe_href(href), "{href}");
        }
        for href in [
            "/reports/",
            "../../index.html#case",
            "https://example.org/source",
            "http://localhost:8000/",
        ] {
            assert!(safe_href(href), "{href}");
        }
    }
}
