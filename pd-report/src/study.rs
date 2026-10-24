//! Optional presentation contract; no agents, planning, execution or inference.
use crate::{campaign::escape, paired_review::PairedReviewV1};
use serde::{Deserialize, Serialize};
#[cfg(test)]
#[path = "study_tests.rs"]
mod tests;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariantV1 {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContrastV1 {
    pub id: String,
    pub label: String,
    pub baseline: String,
    pub candidate: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimV1 {
    pub label: String,
    pub status: String,
    pub detail: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohortV1 {
    pub id: String,
    pub label: String,
    pub role: String,
    pub originals: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricV1 {
    pub id: String,
    pub label: String,
    pub unit: String,
    pub lower_is_better: Option<bool>,
    pub successful_pairs_only: bool,
    pub measurement: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceV1 {
    pub variant: String,
    pub label: String,
    pub unit: String,
    pub originals: usize,
    pub measured_repeats: usize,
    pub values: Vec<f64>,
    pub scope: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkV1 {
    pub label: String,
    pub href: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectV1 {
    pub cohort: String,
    pub contrast: String,
    pub metric: String,
    pub pairs: usize,
    pub estimate: f64,
    pub low: f64,
    pub high: f64,
    pub method: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationV1 {
    pub mission: String,
    pub cohort: String,
    pub variant: String,
    pub physical_sha256: String,
    pub occurrence: usize,
    pub outcome: String,
    pub verified_landing: bool,
    pub values: BTreeMap<String, f64>,
    pub attributes: BTreeMap<String, String>,
    pub capture_sha256: String,
    pub descriptor_sha256: String,
    pub evidence_href: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StudyV1 {
    pub schema_version: u32,
    pub id: String,
    pub question: String,
    pub motivation: String,
    pub changed: Vec<String>,
    pub fixed: Vec<String>,
    pub variants: Vec<VariantV1>,
    pub contrasts: Vec<ContrastV1>,
    pub cohorts: Vec<CohortV1>,
    pub metrics: Vec<MetricV1>,
    pub claims: Vec<ClaimV1>,
    pub effects: Vec<EffectV1>,
    pub performance: Vec<PerformanceV1>,
    pub links: Vec<LinkV1>,
    pub default_cohort: String,
    pub default_contrast: String,
    pub default_metric: String,
    pub condition_axes: Vec<String>,
    pub timing_explanation: bool,
    #[serde(default)]
    pub observations: Vec<ObservationV1>,
}

fn unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let v: Vec<_> = values.collect();
    !v.is_empty()
        && v.iter().all(|s| !s.is_empty())
        && v.iter().copied().collect::<BTreeSet<_>>().len() == v.len()
}
fn safe_href(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with("//")
        && !s.contains('\\')
        && !s.chars().any(char::is_control)
        && (!s.contains(':') || s.starts_with("https://") || s.starts_with("http://"))
}

impl StudyV1 {
    pub fn validate(&self, review: &PairedReviewV1) -> Result<(), String> {
        let require = |ok: bool, message: &str| if ok { Ok(()) } else { Err(message.to_string()) };
        require(
            self.schema_version == 1 && !self.id.is_empty(),
            "unsupported study identity",
        )?;
        require(
            unique(self.variants.iter().map(|x| x.id.as_str()))
                && unique(self.contrasts.iter().map(|x| x.id.as_str()))
                && unique(self.cohorts.iter().map(|x| x.id.as_str()))
                && unique(self.metrics.iter().map(|x| x.id.as_str())),
            "duplicate or absent study declarations",
        )?;
        require(
            self.variants
                .iter()
                .map(|v| v.label.as_str())
                .eq(review.labels.iter().map(String::as_str)),
            "study variant order differs from review",
        )?;
        let variants: BTreeSet<_> = self.variants.iter().map(|x| x.id.as_str()).collect();
        let cohorts: BTreeSet<_> = self.cohorts.iter().map(|x| x.id.as_str()).collect();
        let metrics: BTreeSet<_> = self.metrics.iter().map(|x| x.id.as_str()).collect();
        require(
            self.contrasts.iter().all(|c| {
                c.baseline != c.candidate
                    && variants.contains(c.baseline.as_str())
                    && variants.contains(c.candidate.as_str())
            }),
            "invalid study contrast",
        )?;
        require(
            cohorts.contains(self.default_cohort.as_str())
                && metrics.contains(self.default_metric.as_str())
                && self.contrasts.iter().any(|c| c.id == self.default_contrast),
            "unknown study default",
        )?;
        require(
            self.condition_axes.len() <= 2 && self.links.iter().all(|l| safe_href(&l.href)),
            "invalid study axes/link",
        )?;
        require(
            self.claims.iter().all(|c| {
                ["passed", "failed", "descriptive", "limited"].contains(&c.status.as_str())
            }),
            "invalid frozen claim status",
        )?;
        let mut keys = BTreeSet::new();
        let mut attempts = BTreeSet::new();
        let mut mission_cohorts = BTreeMap::new();
        for o in &self.observations {
            require(
                attempts.insert((&o.mission, &o.variant)),
                "duplicate study attempt binding",
            )?;
            if let Some(previous) = mission_cohorts.insert(&o.mission, &o.cohort) {
                require(
                    previous == &o.cohort,
                    "one mission occurrence cannot span cohort roles",
                )?;
            }
            require(
                cohorts.contains(o.cohort.as_str()) && variants.contains(o.variant.as_str()),
                "observation has undeclared role/variant",
            )?;
            let mission = review
                .missions
                .iter()
                .find(|m| m.id == o.mission)
                .ok_or("observation mission missing")?;
            let approach = self
                .variants
                .iter()
                .position(|v| v.id == o.variant)
                .unwrap();
            let attempt = mission
                .attempts
                .iter()
                .find(|a| a.approach == approach)
                .ok_or("observation attempt missing")?;
            require(
                mission.physical_sha256 == o.physical_sha256
                    && mission.occurrence == o.occurrence
                    && attempt.outcome == o.outcome
                    && o.verified_landing == (attempt.outcome == "landed"),
                "study observation contradicts saved attempt",
            )?;
            require(
                keys.insert((&o.cohort, &o.physical_sha256, o.occurrence, &o.variant)),
                "duplicate study pairing key",
            )?;
            require(
                o.values
                    .iter()
                    .all(|(k, v)| metrics.contains(k.as_str()) && v.is_finite())
                    && [o.capture_sha256.as_str(), o.descriptor_sha256.as_str()]
                        .iter()
                        .all(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                    && safe_href(&o.evidence_href),
                "unbound or nonfinite study observation",
            )?;
        }
        require(
            self.observations.len()
                == review
                    .missions
                    .iter()
                    .map(|m| m.attempts.len())
                    .sum::<usize>(),
            "study omits scheduled attempts",
        )?;
        for c in &self.cohorts {
            let originals: BTreeSet<_> = self
                .observations
                .iter()
                .filter(|o| o.cohort == c.id)
                .map(|o| (&o.physical_sha256, o.occurrence))
                .collect();
            require(
                originals.len() == c.originals,
                "study original denominator differs",
            )?;
        }
        for p in &self.performance {
            require(
                variants.contains(p.variant.as_str())
                    && p.originals > 0
                    && p.measured_repeats > 0
                    && p.values.len() == p.originals * p.measured_repeats
                    && p.values.iter().all(|v| v.is_finite() && *v >= 0.0),
                "invalid independent performance sample count",
            )?;
        }
        for e in &self.effects {
            require(
                cohorts.contains(e.cohort.as_str())
                    && metrics.contains(e.metric.as_str())
                    && self.contrasts.iter().any(|c| c.id == e.contrast)
                    && [e.estimate, e.low, e.high].iter().all(|v| v.is_finite())
                    && e.low <= e.estimate
                    && e.estimate <= e.high,
                "invalid retained effect interval",
            )?;
            let contrast = self.contrasts.iter().find(|c| c.id == e.contrast).unwrap();
            let eligible = self
                .observations
                .iter()
                .filter(|a| {
                    a.cohort == e.cohort
                        && a.variant == contrast.baseline
                        && a.verified_landing
                        && a.values.contains_key(&e.metric)
                })
                .filter(|a| {
                    self.observations.iter().any(|b| {
                        b.cohort == a.cohort
                            && b.physical_sha256 == a.physical_sha256
                            && b.occurrence == a.occurrence
                            && b.variant == contrast.candidate
                            && b.verified_landing
                            && b.values.contains_key(&e.metric)
                    })
                })
                .count();
            require(eligible == e.pairs, "retained effect eligibility differs")?;
        }
        Ok(())
    }
    pub fn render(&self, review: &PairedReviewV1) -> Result<String, String> {
        self.validate(review)?;
        let claims=self.claims.iter().map(|c|format!("<article class=\"study-claim\" data-status=\"{}\"><span class=\"study-kicker\">{} · {}</span><strong>{}</strong></article>",escape(&c.status),escape(&c.label),escape(&c.status),escape(&c.detail))).collect::<String>();
        let links = self
            .links
            .iter()
            .map(|l| format!("<a href=\"{}\">{}</a>", escape(&l.href), escape(&l.label)))
            .collect::<Vec<_>>()
            .join(" · ");
        let list = |v: &[String]| {
            v.iter()
                .map(|s| format!("<li>{}</li>", escape(s)))
                .collect::<String>()
        };
        let json = serde_json::to_string(self)
            .map_err(|e| e.to_string())?
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026");
        Ok(format!(
            r#"<style>{css}</style><section class="study-view" id="study-overview" aria-label="Study context and comparison"><div class="study-intro"><span class="study-kicker">The question · retained evidence</span><h2>{question}</h2><p>{motivation}</p><details class="study-design-disclosure"><summary>What changed and what stayed fixed</summary><div class="study-design"><div><strong>Changed</strong><ul>{changed}</ul></div><div><strong>Held fixed</strong><ul>{fixed}</ul></div></div></details></div><div class="study-claims" id="study-fixed-claims">{claims}</div><p class="study-note">Frozen study conclusions above do not change with exploratory selections below. Qualification, engineering diagnostics and repeated timing measurements retain separate denominators.</p><div class="study-controls" id="study-controls"></div><div class="study-scope" id="study-scope" aria-live="polite"></div><div class="study-grid"><section class="study-panel"><h3>Which outcomes changed?</h3><div id="study-outcomes"></div></section><section class="study-panel"><h3 id="study-metric-title">How large is the change?</h3><div id="study-differences"></div></section><section class="study-panel"><h3>Where does the change occur?</h3><div id="study-conditions"></div></section><section class="study-panel"><h3>Separate planning-cost benchmark</h3><div id="study-performance"></div></section></div><section class="study-panel" id="study-explanation" hidden></section><details class="study-sources"><summary>Definitions, interpretation and source evidence</summary><div id="study-definitions"></div><p>{links}</p><p>Actual captured outcomes, native forecasts and explanatory hypotheses have distinct status. This presentation performs no flight, policy execution or new statistical acceptance test.</p></details></section><script type="application/json" id="study-review-data">{json}</script><script>{model}
globalThis.PdStudyProjection=JSON.parse(document.getElementById('study-review-data').textContent);
{script}</script>"#,
            css = include_str!("study.css"),
            question = escape(&self.question),
            motivation = escape(&self.motivation),
            changed = list(&self.changed),
            fixed = list(&self.fixed),
            model = include_str!("study_model.js"),
            script = include_str!("study_review.js")
        ))
    }
}
