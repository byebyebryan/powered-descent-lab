//! Shared, strategy-independent display vocabulary for saved campaign attempts.
use pd_core::{BoundedRunStopCauseV1, PhysicalOutcome};
use serde::{Deserialize, Serialize};

use crate::campaign::{CampaignRowV1, escape, verified};

/// Renderer-only value row derived from saved campaign or study records.
/// This stays outside the native campaign/evaluation contracts.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MissionDisplayValueV1 {
    pub key: String,
    pub label: String,
    pub value: Option<f64>,
    pub unit: String,
    pub text: Option<String>,
}

pub fn display_value(
    key: &str,
    label: &str,
    value: Option<f64>,
    unit: &str,
) -> MissionDisplayValueV1 {
    MissionDisplayValueV1 {
        key: key.into(),
        label: label.into(),
        value,
        unit: unit.into(),
        text: None,
    }
}

pub fn display_text(key: &str, label: &str, text: impl Into<String>) -> MissionDisplayValueV1 {
    MissionDisplayValueV1 {
        key: key.into(),
        label: label.into(),
        value: None,
        unit: String::new(),
        text: Some(text.into()),
    }
}

pub fn display_values(row: &CampaignRowV1, integrity: bool) -> Vec<MissionDisplayValueV1> {
    let mut values = vec![
        display_value("flight_time", "Flight time", row.sim_time_s, "s"),
        display_value("fuel_remaining", "Fuel remaining", row.fuel_kg, "kg"),
    ];
    if let Some(presentation) = &row.presentation {
        values.extend([
            display_value(
                "target_horizontal_offset",
                "Target horizontal offset",
                presentation.target_horizontal_offset_m,
                "m",
            ),
            display_value(
                "height_above_touchdown",
                "Height above upright touchdown",
                presentation.height_above_upright_touchdown_m,
                "m",
            ),
            display_value(
                "endpoint_speed",
                "Endpoint speed",
                presentation.endpoint_speed_mps,
                "m/s",
            ),
        ]);
    }
    values.push(display_text(
        "proof",
        "Integrity and replay",
        proof_label(row, integrity),
    ));
    values.push(display_text("stop", "Stop", stop_label(row)));
    values
}

/// Measured V2 input metadata. Labels and bands organize browsing only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignGeometryV2 {
    pub schema_version: u32,
    pub morphology: String,
    pub chord_barrier_m: f64,
    pub distance_m: f64,
    pub signed_pad_rise_m: f64,
    pub source_width_m: f64,
    pub target_width_m: f64,
    pub crest_count: usize,
}

impl CampaignGeometryV2 {
    pub(crate) fn validate(&self) -> bool {
        self.schema_version == 2
            && matches!(
                self.morphology.as_str(),
                "Mixed morphology"
                    | "Unclassified"
                    | "Mountain crossing"
                    | "Broad massif"
                    | "Successive ridges"
                    | "Elevation transition"
            )
            && [
                self.chord_barrier_m,
                self.distance_m,
                self.signed_pad_rise_m,
                self.source_width_m,
                self.target_width_m,
            ]
            .into_iter()
            .all(f64::is_finite)
            && self.distance_m > 0.0
            && self.source_width_m > 0.0
            && self.target_width_m > 0.0
    }

    pub fn barrier_band(&self) -> &'static str {
        if self.chord_barrier_m < 100.0 {
            "Chord barrier <100 m"
        } else if self.chord_barrier_m < 200.0 {
            "Chord barrier 100–<200 m"
        } else if self.chord_barrier_m < 400.0 {
            "Chord barrier 200–<400 m"
        } else {
            "Chord barrier ≥400 m"
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignPresentationV1 {
    pub schema_version: u32,
    pub preview_href: String,
    pub caption: String,
    #[serde(default)]
    pub target_horizontal_offset_m: Option<f64>,
    #[serde(default)]
    pub height_above_upright_touchdown_m: Option<f64>,
    #[serde(default)]
    pub endpoint_speed_mps: Option<f64>,
}

impl CampaignPresentationV1 {
    pub(crate) fn validate(&self) -> bool {
        self.schema_version == 1
            && safe_asset_href(&self.preview_href)
            && [
                self.target_horizontal_offset_m,
                self.height_above_upright_touchdown_m,
                self.endpoint_speed_mps,
            ]
            .into_iter()
            .flatten()
            .all(f64::is_finite)
    }
}

fn safe_asset_href(href: &str) -> bool {
    !href.is_empty()
        && href.trim() == href
        && !href.starts_with('/')
        && !href.contains([':', '\\', '?', '#'])
        && !href.chars().any(char::is_control)
        && href.ends_with(".svg")
        && href
            .split('/')
            .all(|part| !part.is_empty() && part != ".." && part != ".")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisplayOutcome {
    Landed,
    Crashed,
    TimedOut,
    Stopped,
    NotLaunched,
    OffTarget,
    Unsupported,
    Failed,
    Unverified,
}

impl DisplayOutcome {
    pub const ALL: [Self; 9] = [
        Self::Landed,
        Self::Crashed,
        Self::TimedOut,
        Self::Stopped,
        Self::NotLaunched,
        Self::OffTarget,
        Self::Unsupported,
        Self::Failed,
        Self::Unverified,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Landed => "landed",
            Self::Crashed => "crashed",
            Self::TimedOut => "timed-out",
            Self::Stopped => "stopped",
            Self::NotLaunched => "not-launched",
            Self::OffTarget => "off-target",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
            Self::Unverified => "unverified",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Landed => "Verified landing",
            Self::Crashed => "Crashed",
            Self::TimedOut => "Timed out",
            Self::Stopped => "Stopped airborne",
            Self::NotLaunched => "Stopped before departure",
            Self::OffTarget => "Landed off target",
            Self::Unsupported => "Unsupported",
            Self::Failed => "Execution / capture failure",
            Self::Unverified => "Unverified outcome",
        }
    }
}

pub fn outcome(row: &CampaignRowV1, campaign_integrity: bool) -> DisplayOutcome {
    if row.disposition == "unsupported" {
        return DisplayOutcome::Unsupported;
    }
    if row.disposition != "executed" {
        return DisplayOutcome::Failed;
    }
    if !campaign_integrity
        || !row.execution_valid
        || !row.integrity_passed
        || !row.physical_replay_verified
        || !row.complete_replay_verified
    {
        return DisplayOutcome::Unverified;
    }
    if verified(row) {
        return DisplayOutcome::Landed;
    }
    match row.physical_outcome {
        Some(PhysicalOutcome::Crashed) => DisplayOutcome::Crashed,
        Some(PhysicalOutcome::TimedOut) => DisplayOutcome::TimedOut,
        Some(PhysicalOutcome::Flying) if row.sim_time_s == Some(0.0) => DisplayOutcome::NotLaunched,
        Some(PhysicalOutcome::Flying) => DisplayOutcome::Stopped,
        Some(PhysicalOutcome::LandedOffTarget) => DisplayOutcome::OffTarget,
        _ => DisplayOutcome::Unverified,
    }
}

pub fn stop_label(row: &CampaignRowV1) -> &'static str {
    match row.stop {
        Some(BoundedRunStopCauseV1::MissionTerminal) => "Mission ended",
        Some(BoundedRunStopCauseV1::ScenarioHorizonReached) => "Mission time limit",
        Some(BoundedRunStopCauseV1::CoverageExhausted) => "Saved command program ended",
        Some(BoundedRunStopCauseV1::HardDeadlineReached) => "Finite execution deadline",
        Some(BoundedRunStopCauseV1::SafetyRejected) => "Safety / solver stop",
        Some(BoundedRunStopCauseV1::ExecutionInvalid) => "Invalid execution",
        None => "No recorded flight",
    }
}

pub fn proof_label(row: &CampaignRowV1, integrity: bool) -> &'static str {
    if row.disposition != "executed" {
        "No flight proof"
    } else if integrity
        && row.execution_valid
        && row.integrity_passed
        && row.physical_replay_verified
        && row.complete_replay_verified
    {
        "Integrity and replay passed"
    } else {
        "Verification incomplete or failed"
    }
}

pub fn render_mission_cell(row: &CampaignRowV1) -> String {
    let label = format!(
        "Mission {} · attempt {}",
        row.mission_index, row.attempt_index
    );
    let link = row.report_href.as_ref().map_or_else(
        || escape(&label),
        |href| {
            format!(
                "<a class=\"mission-link\" href=\"{}\">{}</a>",
                escape(href),
                escape(&label)
            )
        },
    );
    let direction = if row.input_metadata_unavailable.is_some() {
        "Direction unavailable"
    } else if row.direction < 0 {
        "Travel left"
    } else if row.direction > 0 {
        "Travel right"
    } else {
        "Vertical"
    };
    format!(
        "<div class=\"preview-cell\">{link}<span class=\"row-note mission-direction\">{direction}</span></div>"
    )
}

fn display_value_label(value: &MissionDisplayValueV1, compact: bool) -> &str {
    if !compact {
        return &value.label;
    }
    match value.key.as_str() {
        "flight_time" => "Time",
        "fuel_remaining" => "Fuel left",
        "contact_speed" | "contact_speed_mps" => "Contact speed",
        "endpoint_speed" => "End speed",
        _ => &value.label,
    }
}

fn display_value_cell(value: &MissionDisplayValueV1, compact: bool) -> String {
    let rendered = if let Some(text) = &value.text {
        escape(text)
    } else if let Some(number) = value.value {
        if number.is_finite() {
            let fixed = format!("{number:.6}");
            let concise = fixed.trim_end_matches('0').trim_end_matches('.');
            format!(
                "{} {}",
                if number == 0.0 { "0" } else { concise },
                escape(&value.unit)
            )
        } else {
            "Unavailable".into()
        }
    } else {
        "Unavailable".into()
    };
    let label = display_value_label(value, compact);
    let full_label = format!(
        "{}{}",
        value.label,
        if value.unit.is_empty() {
            String::new()
        } else {
            format!(" · {}", value.unit)
        }
    );
    format!(
        "<td data-label=\"{}\" title=\"{}\" data-value-key=\"{}\">{rendered}</td>",
        escape(label),
        escape(&full_label),
        escape(&value.key)
    )
}

fn display_value_heading(value: &MissionDisplayValueV1, compact: bool) -> String {
    let label = display_value_label(value, compact);
    let full_label = format!(
        "{}{}",
        value.label,
        if value.unit.is_empty() {
            String::new()
        } else {
            format!(" · {}", value.unit)
        }
    );
    format!(
        "<th scope=\"col\" title=\"{}\" aria-label=\"{}\">{}{}</th>",
        escape(&full_label),
        escape(&full_label),
        escape(label),
        if value.unit.is_empty() {
            String::new()
        } else {
            format!(" · {}", escape(&value.unit))
        }
    )
}

fn display_values_table(
    values: &[MissionDisplayValueV1],
    status: DisplayOutcome,
    attempt_index: u64,
    compact: bool,
) -> String {
    let headings = values
        .iter()
        .map(|value| display_value_heading(value, compact))
        .collect::<String>();
    let cells = values
        .iter()
        .map(|value| display_value_cell(value, compact))
        .collect::<String>();
    format!(
        "<div class=\"mission-run-values-wrap\"><table class=\"mission-run-values mission-run-values-{}\"><thead><tr><th scope=\"col\">Run</th><th scope=\"col\">Outcome</th>{headings}</tr></thead><tbody><tr{}><th scope=\"row\" data-label=\"Run\" title=\"Saved run · attempt {attempt_index}\" aria-label=\"Saved run · attempt {attempt_index}\">Saved run</th><td data-label=\"Outcome\"><span class=\"outcome-badge outcome-{}\">{}</span></td>{cells}</tr></tbody></table></div>",
        if compact { "overview" } else { "full" },
        if compact {
            " data-run-role=\"single\""
        } else {
            ""
        },
        status.key(),
        status.label()
    )
}

/// A one-run leaf rendered with the same card and value-table vocabulary as a
/// selected side of the interactive mission comparison.
pub fn render_single_run(row: &CampaignRowV1, integrity: bool) -> String {
    let status = outcome(row, integrity);
    let preview = row.presentation.as_ref().map_or_else(
        || if status == DisplayOutcome::Unsupported {
            "<span class=\"mission-preview mission-preview-unavailable\">No flight recorded for unsupported input.</span>".into()
        } else {
            "<span class=\"mission-preview mission-preview-unavailable\">Preview unavailable in this saved projection.</span>".into()
        },
        |view| {
            let image = format!(
                "<img class=\"mission-preview-image\" data-preview-src=\"{}\" width=\"156\" height=\"92\" alt=\"{}\" decoding=\"async\">",
                escape(&view.preview_href),
                escape(&view.caption)
            );
            row.report_href.as_ref().map_or_else(
                || format!("<span class=\"run-preview mission-preview\">{image}</span>"),
                |href| format!("<a class=\"run-preview mission-preview\" href=\"{}\" aria-label=\"Open saved run\">{image}</a>", escape(href)),
            )
        },
    );
    let values = display_values(row, integrity);
    let overview = values
        .iter()
        .filter(|value| {
            matches!(
                value.key.as_str(),
                "flight_time" | "fuel_remaining" | "endpoint_speed"
            )
        })
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    let overview_table = display_values_table(&overview, status, row.attempt_index, true);
    let full_table = display_values_table(&values, status, row.attempt_index, false);
    let caption = row.presentation.as_ref().map_or_else(String::new, |view| {
        format!(
            "<p class=\"preview-caption muted\" title=\"{}\">{}</p>",
            escape(&view.caption),
            escape(&view.caption)
        )
    });
    let href = row.report_href.as_deref().map_or_else(String::new, escape);
    format!(
        "<section class=\"mission-presentation mission-presentation-single\" data-presentation-mode=\"single\"><div class=\"mission-run-grid single\"><article class=\"mission-run-card mission-run-single series-single\"><div class=\"mission-run-heading\"><strong title=\"Saved run · attempt {}\" aria-label=\"Saved run · attempt {}\">Saved run</strong></div><div class=\"mission-run-preview\">{preview}</div></article></div><div class=\"mission-results\">{overview_table}<details class=\"mission-run-details\"><summary>Details and recorded time</summary><div class=\"mission-run-detail-body\">{caption}{full_table}<div class=\"mission-run-evidence\">{}</div><details class=\"mission-time-inspection\" data-campaign-inspection data-attempt-index=\"{}\" data-attempt-href=\"{}\" data-label=\"Saved run · attempt {}\" data-outcome=\"{}\"><summary>Inspect recorded time</summary><div class=\"mission-inspection-host\" data-inspection-host></div></details></div></details></div></section>",
        row.attempt_index,
        row.attempt_index,
        render_evidence_cell(row, integrity),
        row.attempt_index,
        href,
        row.attempt_index,
        status.key()
    )
}

pub fn render_direction_filter(id: &str) -> String {
    format!(
        "<details class=\"review-more-filters\"><summary>More filters</summary><div class=\"paired-controls\"><label>Travel direction<select id=\"{}\"><option value=\"all\">All directions</option><option value=\"-1\">Left</option><option value=\"1\">Right</option><option value=\"0\">Vertical</option><option value=\"unavailable\">Unavailable</option></select></label></div><p class=\"section-note\">Travel direction varies within each terrain stratum. Preview plots normalize left/right for comparison.</p></details>",
        escape(id)
    )
}

pub fn render_outcome_cell(row: &CampaignRowV1, integrity: bool) -> String {
    let status = outcome(row, integrity);
    let note = row.detail.as_ref().map_or_else(String::new, |value| {
        format!("<span class=\"row-note\">{}</span>", escape(value))
    });
    format!(
        "<span class=\"outcome-badge outcome-{}\">{}</span><span class=\"row-note\">{}</span>{note}",
        status.key(),
        status.label(),
        stop_label(row)
    )
}

pub fn render_evidence_cell(row: &CampaignRowV1, integrity: bool) -> String {
    format!(
        "<span class=\"proof-status\">{}</span><details class=\"row-evidence\"><summary>Identity and checks</summary><dl><dt>Physical input</dt><dd><code>{}</code></dd><dt>Execution valid</dt><dd>{}</dd><dt>Capture integrity</dt><dd>{}</dd><dt>Physical replay</dt><dd>{}</dd><dt>Complete replay</dt><dd>{}</dd><dt>Application wall time</dt><dd>{}</dd></dl></details>",
        proof_label(row, integrity),
        escape(&row.physical_sha256),
        row.execution_valid,
        row.integrity_passed,
        row.physical_replay_verified,
        row.complete_replay_verified,
        row.wall_time_us.map_or_else(
            || "Unavailable".into(),
            |n| format!("{:.2} ms", n as f64 / 1000.0)
        )
    )
}

/// Mutually exclusive scheduled-attempt partition; proof is reported separately.
pub fn render_outcome_bar(label: &str, rows: &[(&CampaignRowV1, bool)], approach: usize) -> String {
    let total = rows.len();
    let counts = DisplayOutcome::ALL.map(|status| {
        rows.iter()
            .filter(|(row, integrity)| outcome(row, *integrity) == status)
            .count()
    });
    let mut segments = String::new();
    let mut legend = String::new();
    for (status, count) in DisplayOutcome::ALL.into_iter().zip(counts) {
        if count == 0 {
            continue;
        }
        let pct = 100.0 * count as f64 / total.max(1) as f64;
        let title = format!("{}: {count}/{total} ({pct:.1}%)", status.label());
        segments.push_str(&format!(
            "<span class=\"bar-{}\" style=\"width:{pct:.5}%\" title=\"{}\"></span>",
            status.key(),
            escape(&title)
        ));
        legend.push_str(&format!("<button type=\"button\" data-review-outcome=\"{}\" data-approach=\"{approach}\"><span class=\"legend-dot bar-{}\"></span>{} <strong>{count}</strong> <span class=\"muted\">{pct:.1}%</span></button>", status.key(), status.key(), status.label()));
    }
    let landed = counts[0];
    let executed = rows
        .iter()
        .filter(|(r, _)| r.disposition == "executed")
        .count();
    let proof = rows
        .iter()
        .filter(|(r, integrity)| {
            *integrity
                && r.disposition == "executed"
                && r.execution_valid
                && r.integrity_passed
                && r.physical_replay_verified
                && r.complete_replay_verified
        })
        .count();
    format!(
        "<article class=\"outcome-card\" data-outcome-approach=\"{approach}\"><div class=\"outcome-card-head\"><h3>{}</h3><strong>{landed}/{total} <span class=\"muted\">verified landings · {:.1}%</span></strong></div><div class=\"outcome-bar\" role=\"img\" aria-label=\"{}\">{segments}</div><div class=\"outcome-legend\">{legend}</div><p class=\"proof-note\">Saved integrity and complete replay: {proof}/{executed} executed attempts. All {total} scheduled attempts remain in the outcome denominator.</p></article>",
        escape(label),
        100.0 * landed as f64 / total.max(1) as f64,
        escape(&format!("{label}: {landed}/{total} verified landings"))
    )
}

pub const STYLE: &str = include_str!("campaign_view.css");
pub const SCRIPT: &str = include_str!("campaign_view.js");

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_assets_are_relative_bounded_and_finite() {
        let mut view = CampaignPresentationV1 {
            schema_version: 1,
            preview_href: "previews/flight.svg".into(),
            caption: "Saved flight".into(),
            target_horizontal_offset_m: None,
            height_above_upright_touchdown_m: None,
            endpoint_speed_mps: None,
        };
        assert!(view.validate());
        for href in [
            "javascript:x.svg",
            "//host/x.svg",
            "../x.svg",
            "bad\\x.svg",
            "x.svg#fragment",
            "x.svg\n",
        ] {
            view.preview_href = href.into();
            assert!(!view.validate(), "{href}");
        }
        view.preview_href = "flight.svg".into();
        view.endpoint_speed_mps = Some(f64::NAN);
        assert!(!view.validate());
    }

    fn row() -> CampaignRowV1 {
        CampaignRowV1 {
            attempt_index: 0,
            mission_index: 0,
            physical_sha256: "1".repeat(64),
            terrain_family: "test".into(),
            direction: 1,
            terrain_stratum: Some(0),
            geometry: None,
            input_metadata_unavailable: None,
            disposition: "executed".into(),
            detail: None,
            stop: Some(BoundedRunStopCauseV1::MissionTerminal),
            physical_outcome: Some(PhysicalOutcome::LandedOnTarget),
            mission_outcome: Some(pd_core::MissionOutcome::Success),
            execution_valid: true,
            integrity_passed: true,
            physical_replay_verified: true,
            complete_replay_verified: true,
            verified_target_landing: true,
            sim_time_s: Some(10.0),
            fuel_kg: Some(1.0),
            wall_time_us: None,
            report_href: Some("attempt/report.html".into()),
            presentation: None,
        }
    }

    #[test]
    fn physical_misses_unsupported_and_missing_proof_cannot_become_landings() {
        let mut value = row();
        assert_eq!(outcome(&value, true), DisplayOutcome::Landed);
        assert_eq!(outcome(&value, false), DisplayOutcome::Unverified);
        value.complete_replay_verified = false;
        assert_eq!(outcome(&value, true), DisplayOutcome::Unverified);
        value.complete_replay_verified = true;
        value.verified_target_landing = false;
        for (physical, expected) in [
            (PhysicalOutcome::Crashed, DisplayOutcome::Crashed),
            (PhysicalOutcome::TimedOut, DisplayOutcome::TimedOut),
            (PhysicalOutcome::Flying, DisplayOutcome::Stopped),
            (PhysicalOutcome::LandedOffTarget, DisplayOutcome::OffTarget),
        ] {
            value.physical_outcome = Some(physical);
            assert_eq!(outcome(&value, true), expected);
        }
        value.physical_outcome = Some(PhysicalOutcome::Flying);
        value.sim_time_s = Some(0.0);
        assert_eq!(outcome(&value, true), DisplayOutcome::NotLaunched);
        value.disposition = "unsupported".into();
        assert_eq!(outcome(&value, true), DisplayOutcome::Unsupported);
        value.disposition = "initialization_failed".into();
        assert_eq!(outcome(&value, true), DisplayOutcome::Failed);
    }

    #[test]
    fn old_projections_keep_their_encoding_and_single_run_leaf_is_typed_and_deferred() {
        let mut value = row();
        let encoded = serde_json::to_value(&value).unwrap();
        assert!(encoded.get("presentation").is_none());
        let restored: CampaignRowV1 = serde_json::from_value(encoded.clone()).unwrap();
        assert!(restored.presentation.is_none());
        assert_eq!(serde_json::to_value(restored).unwrap(), encoded);
        value.presentation = Some(CampaignPresentationV1 {
            schema_version: 1,
            preview_href: "previews/test.svg".into(),
            caption: "Recorded <flight>".into(),
            target_horizontal_offset_m: None,
            height_above_upright_touchdown_m: None,
            endpoint_speed_mps: None,
        });
        let html = render_single_run(&value, true);
        assert!(html.contains("class=\"mission-presentation mission-presentation-single\""));
        assert!(html.contains("class=\"mission-run-grid single\""));
        assert!(html.contains("class=\"mission-results\""));
        assert!(html.contains("class=\"run-preview mission-preview\""));
        assert!(html.contains("data-preview-src=\"previews/test.svg\""));
        assert!(!html.contains("<img src="));
        assert!(html.contains("Recorded &lt;flight&gt;"));
        assert!(html.contains("<th scope=\"col\">Run</th><th scope=\"col\">Outcome</th>"));
        assert!(html.contains("data-value-key=\"flight_time\""));
        assert!(html.contains("data-value-key=\"fuel_remaining\""));
        assert!(html.contains("<summary>Details and recorded time</summary>"));
        assert!(!html.contains("<details class=\"mission-run-details\" open"));
        assert!(html.contains("data-value-key=\"target_horizontal_offset\""));
        assert!(html.contains("data-value-key=\"height_above_touchdown\""));
        assert!(html.contains("data-value-key=\"endpoint_speed\""));
        assert_eq!(html.matches("<tbody><tr").count(), 2);
        let overview = html
            .split("mission-run-values-overview")
            .nth(1)
            .unwrap()
            .split("</table>")
            .next()
            .unwrap();
        assert_eq!(overview.matches("data-value-key=").count(), 3);
        let preview = html
            .split("<article class=\"mission-run-card")
            .nth(1)
            .unwrap()
            .split("</article>")
            .next()
            .unwrap();
        assert!(!preview.contains("outcome-badge"));
        assert_eq!(html.matches("data-run-role=\"single\"").count(), 1);
        assert!(!html.contains("Compare A"));
        assert!(html.contains("Physical input"));
        assert!(html.contains("summary>Inspect recorded time</summary>"));
    }
}
