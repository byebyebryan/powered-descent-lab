use crate::{CoverageDesignV1, InputAuditRowV1, ReferenceQaSpecV1, morphology_group_key};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn qa_indices(rows: &[InputAuditRowV1], spec: &ReferenceQaSpecV1) -> Vec<u64> {
    let regular: BTreeSet<_> = rows
        .iter()
        .filter(|r| spec.is_regular(r.index))
        .map(|r| r.index)
        .collect();
    let mut boundary: Vec<_> = rows
        .iter()
        .filter(|r| !regular.contains(&r.index))
        .collect();
    boundary.sort_by(|a, b| {
        a.boundary_score
            .total_cmp(&b.boundary_score)
            .then_with(|| a.index.cmp(&b.index))
    });
    regular
        .into_iter()
        .chain(
            boundary
                .into_iter()
                .take(spec.boundary_extra_per_backend)
                .map(|r| r.index),
        )
        .collect()
}

/// Queue order is the predeclared marginal/joint/morphology/boundary sequence.
/// Each round takes at most one new input per queue; no population quota changes.
pub(super) fn preview_indices(
    rows: &[InputAuditRowV1],
    design: &CoverageDesignV1,
    budget: usize,
) -> Vec<u64> {
    if budget == 0 {
        return Vec::new();
    }
    let mut queues = Vec::<VecDeque<u64>>::new();
    let value = |r: &InputAuditRowV1, axis| {
        r.values
            .iter()
            .find(|v| v.axis == axis)
            .and_then(|v| v.value)
    };
    for axis in &design.axes {
        let mut sorted: Vec<_> = rows
            .iter()
            .filter_map(|r| value(r, axis.axis).map(|v| (v, r.index)))
            .collect();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        if !sorted.is_empty() {
            queues.push(
                [0.0, 0.1, 0.5, 0.9, 1.0]
                    .into_iter()
                    .map(|q| sorted[((sorted.len() - 1) as f64 * q).round() as usize].1)
                    .collect(),
            );
        }
    }
    for pair in &design.joints {
        let x = design.axis(pair[0]).expect("validated joint");
        let y = design.axis(pair[1]).expect("validated joint");
        let mut cells = BTreeMap::<(usize, usize), Vec<u64>>::new();
        for r in rows {
            cells
                .entry((x.slot(value(r, pair[0])), y.slot(value(r, pair[1]))))
                .or_default()
                .push(r.index);
        }
        let mut cell_queues: Vec<VecDeque<u64>> = cells
            .into_values()
            .map(|mut indices| {
                indices.sort_unstable();
                indices.into()
            })
            .collect();
        // A joint gets one outer queue. Cells take turns inside it so hundreds
        // of occupied cells cannot starve morphology or boundary review.
        let mut queue = VecDeque::new();
        loop {
            let mut progress = false;
            for cell in &mut cell_queues {
                if let Some(index) = cell.pop_front() {
                    queue.push_back(index);
                    progress = true;
                }
            }
            if !progress {
                break;
            }
        }
        queues.push(queue);
    }
    let mut groups = BTreeMap::<&str, Vec<u64>>::new();
    for r in rows {
        groups
            .entry(morphology_group_key(&r.labels))
            .or_default()
            .push(r.index);
    }
    for mut indices in groups.into_values() {
        indices.sort_unstable();
        queues.push(indices.into());
    }
    let mut boundary: Vec<_> = rows.iter().collect();
    boundary.sort_by(|a, b| {
        a.boundary_score
            .total_cmp(&b.boundary_score)
            .then_with(|| a.index.cmp(&b.index))
    });
    queues.push(boundary.into_iter().map(|r| r.index).collect());
    let mut selected = BTreeSet::new();
    let mut result = Vec::new();
    loop {
        let mut progress = false;
        for queue in &mut queues {
            while let Some(index) = queue.pop_front() {
                if selected.insert(index) {
                    result.push(index);
                    progress = true;
                    break;
                }
            }
            if result.len() == budget {
                return result;
            }
        }
        if !progress {
            return result;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    fn rows() -> Vec<InputAuditRowV1> {
        [0, 1, 2, 3, 79, 80, 81]
            .into_iter()
            .map(|index| {
                let measured = ChallengeMeasurementV1 {
                    sustained_barrier: true,
                    elevation_transition: false,
                    successive_crests: false,
                    qualifying_crest_count: 0,
                };
                InputAuditRowV1 {
                    index,
                    proposal: CollectionEntryV1 {
                        index,
                        file: format!("proposal-{index:020}.json"),
                        sha256: "a".repeat(64),
                        physical_sha256: format!("{index:064x}"),
                    },
                    disposition: SelectionDispositionV2::Accepted,
                    direction: if index.is_multiple_of(2) { 1 } else { -1 },
                    labels: vec![],
                    values: vec![
                        AxisValueV1 {
                            axis: CoverageAxisV1::Distance,
                            value: Some(index as f64),
                        },
                        AxisValueV1 {
                            axis: CoverageAxisV1::SignedRise,
                            value: Some(-(index as f64)),
                        },
                    ],
                    scalar_metrics: BTreeMap::new(),
                    challenge: ChallengeReceiptV1 {
                        raw: measured.clone(),
                        prepared: measured,
                        qualifies: true,
                    },
                    boundary_score: if [3, 79].contains(&index) { 0.1 } else { 0.4 },
                    shape_signature_sha256: format!("{index:064x}"),
                }
            })
            .collect()
    }
    #[test]
    fn qa_keeps_regular_pairs_then_closest_unselected_boundary_ties() {
        let mut rows = rows();
        let spec = ReferenceQaSpecV1 {
            spacing_m: 1.0,
            regular_index_stride: 80,
            regular_index_offsets: vec![0, 1],
            boundary_extra_per_backend: 2,
            maximum_per_backend: 6,
            diagnostic_flags: ResolutionDiagnosticFlagsV1 {
                maximum_raw_interpolation_error_m: 25.0,
                absolute_chord_barrier_change_m: 25.0,
                absolute_sustained_width_change_m: 40.0,
                admission_disagreement: true,
            },
        };
        assert_eq!(qa_indices(&rows, &spec), vec![0, 1, 80, 81, 3, 79]);
        rows.reverse();
        assert_eq!(qa_indices(&rows, &spec), vec![0, 1, 80, 81, 3, 79]);
    }
    #[test]
    fn preview_round_robin_is_bounded_unique_and_independent_of_input_order() {
        let rows = rows();
        let axis = |axis| DiagnosticAxisV1 {
            axis,
            interior_boundaries: vec![40.0],
            show_underflow_overflow: true,
            show_missing: true,
        };
        let design = CoverageDesignV1 {
            schema_version: 1,
            axes: vec![
                axis(CoverageAxisV1::Distance),
                axis(CoverageAxisV1::SignedRise),
            ],
            joints: vec![[CoverageAxisV1::Distance, CoverageAxisV1::SignedRise]],
        };
        assert_eq!(preview_indices(&rows, &design, 2), vec![0, 81]);
        let all = preview_indices(&rows, &design, 20);
        assert_eq!(all.len(), 7);
        assert_eq!(all.iter().collect::<BTreeSet<_>>().len(), 7);
        let reversed: Vec<_> = rows.into_iter().rev().collect();
        assert_eq!(all, preview_indices(&reversed, &design, 20));
        assert!(preview_indices(&reversed, &design, 0).is_empty());
    }
}
