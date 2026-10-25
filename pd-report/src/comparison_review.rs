//! Optional neutral comparison references and sourced target declarations.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceV1 {
    pub variant: String,
    pub label: String,
    pub run_id: String,
    pub projection_sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetV1 {
    pub id: String,
    pub label: String,
    pub scope: String,
    pub metric: String,
    pub unit: String,
    pub operator: String,
    pub threshold: f64,
    pub observed: Option<f64>,
    pub cohort: Option<String>,
    pub source_href: String,
    pub source_sha256: String,
    pub method: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EarlierV1 {
    pub label: String,
    pub projection_sha256: String,
    pub source_href: String,
    pub baseline_total: usize,
    pub candidate_total: usize,
    pub matched: usize,
    pub baseline_only: usize,
    pub candidate_only: usize,
    pub same_captures_verified: bool,
    pub scientific_changes: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonReviewV1 {
    pub schema_version: u32,
    pub references: Vec<ReferenceV1>,
    pub targets: Vec<TargetV1>,
    pub earlier: Option<EarlierV1>,
    pub single_preview_suffix: Option<String>,
}
fn digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn href(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with("//")
        && !s.contains('\\')
        && !s.chars().any(char::is_control)
        && (!s.contains(':') || s.starts_with("http://") || s.starts_with("https://"))
}
impl ComparisonReviewV1 {
    pub fn validate(&self, study: &crate::study::StudyV1) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "unknown comparison review version"
        );
        let variants: BTreeSet<_> = study.variants.iter().map(|v| v.id.as_str()).collect();
        let supplied: BTreeSet<_> = self.references.iter().map(|v| v.variant.as_str()).collect();
        ensure!(
            variants == supplied && supplied.len() == self.references.len(),
            "run references do not bind the declared variants"
        );
        ensure!(
            self.references
                .iter()
                .all(|r| !r.run_id.is_empty() && digest(&r.projection_sha256)),
            "unbound run reference"
        );
        let ids: BTreeSet<_> = self.targets.iter().map(|t| t.id.as_str()).collect();
        ensure!(
            ids.len() == self.targets.len(),
            "duplicate target declaration"
        );
        for t in &self.targets {
            ensure!(
                matches!(t.scope.as_str(), "mission" | "cohort")
                    && matches!(t.operator.as_str(), "eq" | "gte" | "lte")
                    && t.threshold.is_finite()
                    && t.observed.is_none_or(f64::is_finite)
                    && digest(&t.source_sha256)
                    && href(&t.source_href),
                "invalid sourced target"
            );
            if t.scope == "mission" {
                ensure!(
                    t.metric == "verified_target_landing"
                        && t.operator == "eq"
                        && t.threshold == 1.0,
                    "unsupported mission target projection"
                );
            } else {
                ensure!(
                    t.observed.is_some()
                        && study
                            .cohorts
                            .iter()
                            .any(|c| Some(&c.id) == t.cohort.as_ref()),
                    "cohort target lacks its frozen source value"
                );
            }
        }
        if let Some(e) = &self.earlier {
            ensure!(
                digest(&e.projection_sha256)
                    && href(&e.source_href)
                    && e.matched + e.baseline_only == e.baseline_total
                    && e.matched + e.candidate_only == e.candidate_total,
                "earlier run join accounting differs"
            );
            ensure!(
                e.same_captures_verified && e.scientific_changes == 0 && e.candidate_only == 0,
                "earlier edition reuse requires a complete proven scientific no-op"
            );
        }
        ensure!(
            self.single_preview_suffix
                .as_deref()
                .is_none_or(|s| s == "-single.svg"),
            "unknown preview transport"
        );
        Ok(())
    }
    pub fn decorate(&self, html: &str) -> Result<String> {
        let json = serde_json::to_string(self)?.replace('<', "\\u003c");
        Ok(html.replacen("</head>", &format!("<script type=\"application/json\" id=\"comparison-review-data\">{json}</script><script>{}</script></head>", include_str!("comparison_review_model.js")), 1))
    }
    pub fn pair_page(&self) -> Result<String> {
        let body = format!(
            "<div id=\"pair-status\" aria-live=\"polite\">Loading the selected saved pair…</div><div id=\"pair-summary\"></div><div id=\"pair-inspection\"></div><script>{}\n{}\n{}\n{}\n{}\n{}</script>",
            include_str!("study_model.js"),
            include_str!("paired_model.js"),
            include_str!("flight_model.js"),
            include_str!("flight_inspection.js"),
            include_str!("mission_inspection.js"),
            include_str!("pair_review.js")
        );
        self.decorate(&crate::batch::render_status_page(
            "Paired flight inspection",
            "<a href=\"report.html\" id=\"pair-back\">Back to mission tree</a>",
            &body,
        ))
    }
}
