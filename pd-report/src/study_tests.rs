use super::*;
use crate::paired_review::{PairedAttemptV1, PairedMissionV1};
use serde_json::json;

fn fixture() -> (StudyV1, PairedReviewV1) {
    let review = PairedReviewV1 {
        schema_version: 1,
        labels: vec!["A".into(), "B".into()],
        groups: vec![],
        branches: vec![],
        missions: vec![PairedMissionV1 {
            id: "m".into(),
            group: "g".into(),
            path: vec!["g".into()],
            direction: 0,
            direction_unavailable: false,
            label: "Original".into(),
            mission_index: 0,
            physical_sha256: "a".repeat(64),
            occurrence: 0,
            attempts: (0..2)
                .map(|approach| PairedAttemptV1 {
                    approach,
                    attempt_index: approach as u64,
                    outcome: "landed".into(),
                    cells: String::new(),
                    href: None,
                    preview: None,
                    caption: String::new(),
                })
                .collect(),
        }],
    };
    let study:StudyV1=serde_json::from_value(json!({"schema_version":1,"id":"example","question":"Does <B> improve?","motivation":"A bounded comparison","changed":["timing"],"fixed":["plant"],"variants":[{"id":"a","label":"A"},{"id":"b","label":"B"}],"contrasts":[{"id":"ab","label":"A to B","baseline":"a","candidate":"b"}],"cohorts":[{"id":"primary","label":"Primary","role":"independent","originals":1}],"metrics":[{"id":"fuel","label":"Fuel","unit":"kg","lower_is_better":true,"successful_pairs_only":true,"measurement":"Initial minus final fuel"}],"claims":[{"label":"Fixed","status":"passed","detail":"Original frozen result"}],"effects":[{"cohort":"primary","contrast":"ab","metric":"fuel","pairs":1,"estimate":-1.,"low":-2.,"high":0.,"method":"Retained source method"}],"performance":[],"links":[{"label":"Evidence","href":"proof.json"}],"default_cohort":"primary","default_contrast":"ab","default_metric":"fuel","condition_axes":[],"timing_explanation":false,"observations":[{"mission":"m","cohort":"primary","variant":"a","physical_sha256":"a".repeat(64),"occurrence":0,"outcome":"landed","verified_landing":true,"values":{"fuel":10.},"attributes":{},"capture_sha256":"b".repeat(64),"descriptor_sha256":"c".repeat(64),"evidence_href":"flight-0-evidence.json"},{"mission":"m","cohort":"primary","variant":"b","physical_sha256":"a".repeat(64),"occurrence":0,"outcome":"landed","verified_landing":true,"values":{"fuel":9.},"attributes":{},"capture_sha256":"d".repeat(64),"descriptor_sha256":"c".repeat(64),"evidence_href":"flight-1-evidence.json"}]})).unwrap();
    (study, review)
}

#[test]
fn neutral_projection_binds_pairs_denominators_and_retained_effect_eligibility() {
    let (study, review) = fixture();
    study.validate(&review).unwrap();
    let mut invalid = study.clone();
    invalid.cohorts[0].originals = 2;
    assert!(invalid.validate(&review).is_err());
    let mut invalid = study.clone();
    invalid.observations[1].physical_sha256 = "f".repeat(64);
    assert!(invalid.validate(&review).is_err());
    let mut invalid = study.clone();
    invalid.observations[1] = invalid.observations[0].clone();
    assert!(invalid.validate(&review).is_err());
    let mut invalid = study.clone();
    invalid.effects[0].pairs = 2;
    assert!(invalid.validate(&review).is_err());
    let mut invalid = study.clone();
    invalid.performance.push(PerformanceV1 {
        variant: "b".into(),
        label: "B".into(),
        unit: "ms".into(),
        originals: 2,
        measured_repeats: 2,
        values: vec![1.],
        scope: "separate".into(),
    });
    assert!(invalid.validate(&review).is_err());
}

#[test]
fn study_render_escapes_authored_context_and_never_promotes_bad_links_or_claims() {
    let (study, review) = fixture();
    let html = study.render(&review).unwrap();
    assert!(html.contains("Does &lt;B&gt; improve?"));
    assert!(html.contains("id=\"study-fixed-claims\""));
    assert!(html.contains("Frozen study conclusions"));
    let mut invalid = study.clone();
    invalid.links[0].href = "javascript:alert(1)".into();
    assert!(invalid.render(&review).is_err());
    let mut invalid = study.clone();
    invalid.claims[0].status = "winner".into();
    assert!(invalid.render(&review).is_err());
}
