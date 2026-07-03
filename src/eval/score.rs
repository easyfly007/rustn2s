use super::EvalReport;
use serde::Serialize;

/// Weights for combining individual metrics into a single quality score.
/// Each weight is in [0, 1] and they sum to 1.0.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreWeights {
    pub overlap: f64,
    pub crossings: f64,
    pub aspect_ratio: f64,
    pub wire_length: f64,
    pub label_ratio: f64,
    pub symmetry: f64,
    pub power_convention: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            overlap: 0.20,
            crossings: 0.15,
            aspect_ratio: 0.20,
            wire_length: 0.10,
            label_ratio: 0.10,
            symmetry: 0.15,
            power_convention: 0.10,
        }
    }
}

/// Breakdown of how each metric contributed to the overall score.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreBreakdown {
    pub overall: f64,
    pub overlap_score: f64,
    pub crossings_score: f64,
    pub aspect_ratio_score: f64,
    pub wire_length_score: f64,
    pub label_ratio_score: f64,
    pub symmetry_score: f64,
    pub power_convention_score: f64,
}

/// Tier 1 — pass/fail safety constraints. Each field is true when the
/// schematic is correct on that dimension; false means a real bug
/// (overlap, mismatched pair, PMOS-below-NMOS), not a quality
/// trade-off. Reported separately from continuous quality metrics
/// because mixing them in a weighted sum is misleading — see
/// `docs/metric_reform.md`.
#[derive(Debug, Clone, Serialize)]
pub struct SafetyReport {
    pub no_overlap: bool,
    pub power_convention_clean: bool,
    pub symmetry_clean: bool,
}

impl SafetyReport {
    /// True when every safety constraint passes.
    pub fn passes(&self) -> bool {
        self.no_overlap && self.power_convention_clean && self.symmetry_clean
    }

    /// Names of the failing constraints (empty when `passes()` is true).
    pub fn failures(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if !self.no_overlap {
            out.push("overlap");
        }
        if !self.power_convention_clean {
            out.push("power_convention");
        }
        if !self.symmetry_clean {
            out.push("symmetry");
        }
        out
    }
}

/// Tier 2 — continuous quality sub-scores in `[0, 1]`. These vary
/// meaningfully across circuits and reflect routing/placer choices,
/// not bugs. Don't combine them with a fixed weighted sum; lex-min or
/// per-circuit weighting is more honest.
#[derive(Debug, Clone, Serialize)]
pub struct QualityProfile {
    pub aspect_ratio: f64,
    pub crossings: f64,
    pub wire_length: f64,
    pub label_ratio: f64,
    /// Fraction of text elements (net labels, instance captions) that do
    /// NOT collide with other text or component bodies. Added 2026-07-04:
    /// this is the readability signal every earlier score missed (case 27
    /// scored 1.000 while its text layers were illegible).
    pub text_clarity: f64,
}

impl QualityProfile {
    /// All five sub-score values as a fixed-order array, useful for
    /// lexicographic comparisons. Order is canonical and stable so
    /// callers can sort or compare profiles without naming fields.
    pub fn as_array(&self) -> [f64; 5] {
        [
            self.aspect_ratio,
            self.crossings,
            self.wire_length,
            self.label_ratio,
            self.text_clarity,
        ]
    }

    /// Lexicographic minimum across all five sub-scores. Returns the
    /// worst sub-score paired with its name.
    pub fn worst(&self) -> (&'static str, f64) {
        let mut worst = ("aspect_ratio", self.aspect_ratio);
        for (name, value) in [
            ("crossings", self.crossings),
            ("wire_length", self.wire_length),
            ("label_ratio", self.label_ratio),
            ("text_clarity", self.text_clarity),
        ] {
            if value < worst.1 {
                worst = (name, value);
            }
        }
        worst
    }

    /// Lex-min worst across the *quality* sub-scores only — i.e.
    /// excludes `aspect_ratio`, which is treated as a shape signal
    /// rather than a quality metric. See docs/metric_reform.md
    /// step 7. This is the recommended comparator for new code:
    /// it stops penalizing legitimately wide signal chains
    /// (circuits 02, 03, 16, 21 in the test suite).
    pub fn worst_quality(&self) -> (&'static str, f64) {
        let mut worst = ("crossings", self.crossings);
        for (name, value) in [
            ("wire_length", self.wire_length),
            ("label_ratio", self.label_ratio),
            ("text_clarity", self.text_clarity),
        ] {
            if value < worst.1 {
                worst = (name, value);
            }
        }
        worst
    }

    /// Weighted quality score that excludes `aspect_ratio`. The three
    /// continuous quality sub-scores (crossings, wire_length,
    /// label_ratio) are combined with weights renormalized so they
    /// sum to 1.0; aspect_ratio is reported separately as a shape
    /// signal. `text_clarity` is deliberately NOT in this weighted sum
    /// yet — it has no tuned weight and the overfitting audit warns
    /// against inventing one; lex-min consumers (`worst_quality`) see
    /// it instead. Recommended over `compute_score(...).overall` for
    /// new consumers — see docs/metric_reform.md step 7.
    pub fn quality_score(&self, weights: &ScoreWeights) -> f64 {
        let total = weights.crossings + weights.wire_length + weights.label_ratio;
        if total <= 0.0 {
            return 1.0;
        }
        (weights.crossings * self.crossings
            + weights.wire_length * self.wire_length
            + weights.label_ratio * self.label_ratio)
            / total
    }
}

/// Compute the two-tier evaluation: safety pass/fail + quality
/// profile. Both tiers are derived from the same raw `EvalReport`,
/// so this is a pure projection of `compute_score`'s sub-scores into
/// the cleaner shape recommended by `docs/metric_reform.md`.
pub fn compute_profile(report: &EvalReport) -> (SafetyReport, QualityProfile) {
    // Safety: derived from the same per-metric reports as the legacy
    // sub-scores, but reported as booleans rather than weighted real
    // numbers.
    let safety = SafetyReport {
        no_overlap: report.component_overlap.overlap_count == 0,
        power_convention_clean: report.power_convention.score >= 0.999,
        symmetry_clean: report.symmetry.overall_score >= 0.999,
    };

    // Quality: identical formulas to compute_score, just packaged
    // into the four-dim continuous profile.
    let breakdown = compute_score(report, &ScoreWeights::default());
    let quality = QualityProfile {
        aspect_ratio: breakdown.aspect_ratio_score,
        crossings: breakdown.crossings_score,
        wire_length: breakdown.wire_length_score,
        label_ratio: breakdown.label_ratio_score,
        text_clarity: report.text_overlap.score,
    };

    (safety, quality)
}

/// Compute a single quality score in [0, 1] from an EvalReport.
/// Higher is better.
pub fn compute_score(report: &EvalReport, weights: &ScoreWeights) -> ScoreBreakdown {
    // 1. Overlap: 1.0 if zero overlaps, 0.0 if any
    let overlap_score = if report.component_overlap.overlap_count == 0 {
        1.0
    } else {
        0.0
    };

    // 2. Crossings: 1.0 if zero, decays with count
    let crossings_score = 1.0 / (1.0 + report.wire_crossings.crossing_count as f64);

    // 3. Aspect ratio: 1.0 at ratio 1.5, decays toward 0 as ratio grows
    //    Ideal range: [1.0, 2.5]. Penalty for ratios outside this range.
    let ar = report.bounding_box.aspect_ratio;
    let aspect_ratio_score = if ar <= 2.5 {
        1.0
    } else if ar <= 5.0 {
        1.0 - (ar - 2.5) / 2.5 * 0.5 // 1.0 → 0.5
    } else if ar <= 10.0 {
        0.5 - (ar - 5.0) / 5.0 * 0.3 // 0.5 → 0.2
    } else {
        (0.2 - (ar - 10.0) / 40.0 * 0.2).max(0.0) // 0.2 → 0.0
    };

    // 4. Wire length: normalized score. Shorter total is better.
    //    Use component count as baseline: ideal ~100 units per component.
    let comp_count = report.bounding_box.component_count.max(1) as f64;
    let ideal_length = comp_count * 100.0;
    let length_ratio = report.wire_length.total_length / ideal_length;
    let wire_length_score = if length_ratio <= 1.0 {
        1.0
    } else {
        1.0 / length_ratio
    };

    // 5. Label ratio: prefer fewer labels relative to wires.
    //    Score 1.0 when ratio = 0, decays as more labels are used.
    let lr = report.label_usage.label_to_wire_ratio;
    let label_ratio_score = if lr <= 0.0 || lr.is_infinite() {
        if report.label_usage.direct_wires > 0 {
            1.0
        } else {
            0.5
        }
    } else {
        1.0 / (1.0 + lr * 2.0)
    };

    // 6. Symmetry: directly from eval (already 0-1)
    let symmetry_score = report.symmetry.overall_score;

    // 7. Power convention: directly from eval (already 0-1)
    let power_convention_score = report.power_convention.score;

    // Weighted sum
    let overall = weights.overlap * overlap_score
        + weights.crossings * crossings_score
        + weights.aspect_ratio * aspect_ratio_score
        + weights.wire_length * wire_length_score
        + weights.label_ratio * label_ratio_score
        + weights.symmetry * symmetry_score
        + weights.power_convention * power_convention_score;

    ScoreBreakdown {
        overall: round3(overall),
        overlap_score: round3(overlap_score),
        crossings_score: round3(crossings_score),
        aspect_ratio_score: round3(aspect_ratio_score),
        wire_length_score: round3(wire_length_score),
        label_ratio_score: round3(label_ratio_score),
        symmetry_score: round3(symmetry_score),
        power_convention_score: round3(power_convention_score),
    }
}

/// Identify which metrics are dragging the score down and suggest parameter changes.
#[derive(Debug, Clone, Serialize)]
pub struct TuningAdvice {
    pub parameter: String,
    pub current_value: f64,
    pub suggested_value: f64,
    pub reason: String,
}

pub fn suggest_tuning(
    report: &EvalReport,
    breakdown: &ScoreBreakdown,
    layer_spacing: f64,
    block_spacing: f64,
    device_spacing: f64,
    label_threshold: f64,
) -> Vec<TuningAdvice> {
    let mut advice = Vec::new();

    // High aspect ratio → spread horizontally
    if breakdown.aspect_ratio_score < 0.8 {
        let ar = report.bounding_box.aspect_ratio;
        if report.bounding_box.height > report.bounding_box.width {
            // Too tall: increase layer spacing to spread horizontally
            let factor = (ar / 2.0).min(3.0);
            advice.push(TuningAdvice {
                parameter: "layer_spacing".into(),
                current_value: layer_spacing,
                suggested_value: round1(layer_spacing * factor),
                reason: format!(
                    "Aspect ratio {:.1} is too tall; increase horizontal spread",
                    ar
                ),
            });
            // Also reduce device spacing to compress vertically
            advice.push(TuningAdvice {
                parameter: "device_spacing".into(),
                current_value: device_spacing,
                suggested_value: round1((device_spacing * 0.7).max(40.0)),
                reason: format!("Reduce vertical stacking to improve aspect ratio {:.1}", ar),
            });
        } else {
            // Too wide: increase block/device spacing vertically
            let factor = (ar / 2.0).min(3.0);
            advice.push(TuningAdvice {
                parameter: "layer_spacing".into(),
                current_value: layer_spacing,
                suggested_value: round1((layer_spacing / factor).max(100.0)),
                reason: format!(
                    "Aspect ratio {:.1} is too wide; reduce horizontal spread",
                    ar
                ),
            });
        }
    }

    // Component overlap → increase spacing
    if breakdown.overlap_score < 1.0 {
        advice.push(TuningAdvice {
            parameter: "block_spacing".into(),
            current_value: block_spacing,
            suggested_value: round1(block_spacing * 1.5),
            reason: format!(
                "{} component overlaps detected; increase block spacing",
                report.component_overlap.overlap_count
            ),
        });
        advice.push(TuningAdvice {
            parameter: "device_spacing".into(),
            current_value: device_spacing,
            suggested_value: round1(device_spacing * 1.3),
            reason: "Increase device spacing to resolve overlaps".into(),
        });
    }

    // Too many labels → increase threshold
    if breakdown.label_ratio_score < 0.7 && report.label_usage.label_pairs > 0 {
        advice.push(TuningAdvice {
            parameter: "label_threshold".into(),
            current_value: label_threshold,
            suggested_value: round1(label_threshold * 1.5),
            reason: format!(
                "{} label pairs used; increase threshold for more direct wires",
                report.label_usage.label_pairs
            ),
        });
    }

    advice
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{
        BoundingBoxReport, ConnectivityReport, LabelUsageReport, OverlapReport,
        PowerConventionReport, SymmetryReport, TextOverlapReport, WireBendReport,
        WireCrossingReport, WireLengthReport,
    };

    /// Build a perfect-score EvalReport baseline; tweak fields per test.
    fn perfect_report() -> EvalReport {
        EvalReport {
            connectivity: ConnectivityReport {
                expected_net_count: 0,
                found_net_count: 0,
                all_nets_connected: true,
                missing_connections: vec![],
                orphan_labels: vec![],
                duplicate_label_positions: 0,
            },
            component_overlap: OverlapReport {
                overlap_count: 0,
                overlapping_pairs: vec![],
            },
            wire_crossings: WireCrossingReport {
                crossing_count: 0,
                crossings: vec![],
            },
            wire_length: WireLengthReport {
                total_length: 100.0,
                wire_count: 1,
                avg_length: 100.0,
                max_length: 100.0,
                min_length: 100.0,
            },
            wire_bends: WireBendReport {
                total_bends: 0,
                wires_with_bends: 0,
                max_bends_per_wire: 0,
            },
            bounding_box: BoundingBoxReport {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 100.0,
                max_y: 100.0,
                width: 100.0,
                height: 100.0,
                area: 10000.0,
                aspect_ratio: 1.0,
                component_count: 1,
            },
            label_usage: LabelUsageReport {
                total_labels: 0,
                unique_label_names: 0,
                label_pairs: 0,
                direct_wires: 1,
                label_to_wire_ratio: 0.0,
            },
            symmetry: SymmetryReport {
                matched_pairs: vec![],
                overall_score: 1.0,
            },
            power_convention: PowerConventionReport {
                pmos_count: 0,
                nmos_count: 0,
                violations: vec![],
                score: 1.0,
            },
            text_overlap: TextOverlapReport {
                total_texts: 0,
                dirty_texts: 0,
                collisions: vec![],
                score: 1.0,
            },
        }
    }

    #[test]
    fn weights_default_sums_to_one() {
        let w = ScoreWeights::default();
        let sum = w.overlap
            + w.crossings
            + w.aspect_ratio
            + w.wire_length
            + w.label_ratio
            + w.symmetry
            + w.power_convention;
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn perfect_report_scores_one() {
        let r = perfect_report();
        let s = compute_score(&r, &ScoreWeights::default());
        assert_eq!(s.overall, 1.0);
        assert_eq!(s.overlap_score, 1.0);
        assert_eq!(s.crossings_score, 1.0);
        assert_eq!(s.aspect_ratio_score, 1.0);
        assert_eq!(s.symmetry_score, 1.0);
        assert_eq!(s.power_convention_score, 1.0);
    }

    #[test]
    fn any_overlap_zeroes_overlap_subscore() {
        let mut r = perfect_report();
        r.component_overlap.overlap_count = 1;
        let s = compute_score(&r, &ScoreWeights::default());
        assert_eq!(s.overlap_score, 0.0);
        // overall = 1.0 - 0.20 (overlap weight)
        assert!((s.overall - 0.8).abs() < 1e-3);
    }

    #[test]
    fn crossings_decay_with_count() {
        let mut r = perfect_report();
        r.wire_crossings.crossing_count = 1;
        let s1 = compute_score(&r, &ScoreWeights::default());
        // 1 / (1 + 1) = 0.5
        assert_eq!(s1.crossings_score, 0.5);

        r.wire_crossings.crossing_count = 9;
        let s9 = compute_score(&r, &ScoreWeights::default());
        // 1 / (1 + 9) = 0.1
        assert_eq!(s9.crossings_score, 0.1);
    }

    #[test]
    fn aspect_ratio_brackets() {
        let mut r = perfect_report();

        r.bounding_box.aspect_ratio = 2.5;
        assert_eq!(
            compute_score(&r, &ScoreWeights::default()).aspect_ratio_score,
            1.0
        );

        r.bounding_box.aspect_ratio = 5.0;
        // At ar=5.0: 1.0 - (5.0-2.5)/2.5 * 0.5 = 0.5
        assert_eq!(
            compute_score(&r, &ScoreWeights::default()).aspect_ratio_score,
            0.5
        );

        r.bounding_box.aspect_ratio = 10.0;
        // At ar=10.0: 0.5 - (10-5)/5 * 0.3 = 0.2
        assert_eq!(
            compute_score(&r, &ScoreWeights::default()).aspect_ratio_score,
            0.2
        );

        r.bounding_box.aspect_ratio = 50.0;
        // At ar=50: 0.2 - (50-10)/40 * 0.2 = 0.0
        assert_eq!(
            compute_score(&r, &ScoreWeights::default()).aspect_ratio_score,
            0.0
        );

        r.bounding_box.aspect_ratio = 100.0;
        // Clamped at 0.0
        assert_eq!(
            compute_score(&r, &ScoreWeights::default()).aspect_ratio_score,
            0.0
        );
    }

    #[test]
    fn wire_length_score_caps_at_one_when_under_ideal() {
        // 1 component → ideal length = 100. Total 50 → ratio 0.5 < 1 → score 1.0.
        let mut r = perfect_report();
        r.wire_length.total_length = 50.0;
        assert_eq!(
            compute_score(&r, &ScoreWeights::default()).wire_length_score,
            1.0
        );

        // Total 200 → ratio 2 → score 0.5.
        r.wire_length.total_length = 200.0;
        assert_eq!(
            compute_score(&r, &ScoreWeights::default()).wire_length_score,
            0.5
        );
    }

    #[test]
    fn no_advice_when_score_is_perfect() {
        let r = perfect_report();
        let breakdown = compute_score(&r, &ScoreWeights::default());
        let advice = suggest_tuning(&r, &breakdown, 200.0, 100.0, 80.0, 300.0);
        assert!(advice.is_empty());
    }

    #[test]
    fn tall_aspect_ratio_advises_more_horizontal_spread() {
        let mut r = perfect_report();
        r.bounding_box.aspect_ratio = 10.0;
        r.bounding_box.height = 1000.0;
        r.bounding_box.width = 100.0;
        let breakdown = compute_score(&r, &ScoreWeights::default());
        let advice = suggest_tuning(&r, &breakdown, 200.0, 100.0, 80.0, 300.0);

        // Should bump layer_spacing up and device_spacing down.
        let layer = advice
            .iter()
            .find(|a| a.parameter == "layer_spacing")
            .unwrap();
        assert!(layer.suggested_value > layer.current_value);
        let device = advice
            .iter()
            .find(|a| a.parameter == "device_spacing")
            .unwrap();
        assert!(device.suggested_value < device.current_value);
    }

    #[test]
    fn overlap_advises_more_block_and_device_spacing() {
        let mut r = perfect_report();
        r.component_overlap.overlap_count = 2;
        let breakdown = compute_score(&r, &ScoreWeights::default());
        let advice = suggest_tuning(&r, &breakdown, 200.0, 100.0, 80.0, 300.0);

        let block = advice
            .iter()
            .find(|a| a.parameter == "block_spacing")
            .unwrap();
        assert!(block.suggested_value > block.current_value);
        let device = advice
            .iter()
            .find(|a| a.parameter == "device_spacing")
            .unwrap();
        assert!(device.suggested_value > device.current_value);
    }

    // ---- two-tier compute_profile ----

    #[test]
    fn compute_profile_perfect_report_passes_safety() {
        let r = perfect_report();
        let (safety, _q) = compute_profile(&r);
        assert!(safety.passes());
        assert!(safety.failures().is_empty());
    }

    #[test]
    fn compute_profile_overlap_violation_flips_safety() {
        let mut r = perfect_report();
        r.component_overlap.overlap_count = 1;
        let (safety, _q) = compute_profile(&r);
        assert!(!safety.passes());
        assert!(!safety.no_overlap);
        assert_eq!(safety.failures(), vec!["overlap"]);
    }

    #[test]
    fn compute_profile_power_convention_failure_is_distinct() {
        let mut r = perfect_report();
        r.power_convention.score = 0.5;
        let (safety, _q) = compute_profile(&r);
        assert!(!safety.passes());
        assert!(!safety.power_convention_clean);
        assert!(safety.no_overlap);
        assert!(safety.symmetry_clean);
    }

    #[test]
    fn quality_profile_worst_picks_minimum() {
        let q = QualityProfile {
            aspect_ratio: 0.9,
            crossings: 0.4,
            wire_length: 0.7,
            label_ratio: 0.6,
            text_clarity: 0.8,
        };
        let (name, value) = q.worst();
        assert_eq!(name, "crossings");
        assert!((value - 0.4).abs() < 1e-9);
    }

    #[test]
    fn worst_quality_excludes_aspect_ratio() {
        // aspect_ratio is the lowest, but worst_quality skips it.
        let q = QualityProfile {
            aspect_ratio: 0.20, // shape signal — should be ignored
            crossings: 0.45,
            wire_length: 0.80,
            label_ratio: 0.60,
            text_clarity: 0.90,
        };
        // worst() includes ar → returns ("aspect_ratio", 0.20)
        assert_eq!(q.worst().0, "aspect_ratio");
        // worst_quality() excludes ar → returns ("crossings", 0.45)
        let (name, value) = q.worst_quality();
        assert_eq!(name, "crossings");
        assert!((value - 0.45).abs() < 1e-9);
    }

    #[test]
    fn quality_score_excludes_aspect_ratio() {
        let q = QualityProfile {
            aspect_ratio: 0.0, // would tank the score if included
            crossings: 1.0,
            wire_length: 1.0,
            label_ratio: 1.0,
            text_clarity: 1.0,
        };
        let s = q.quality_score(&ScoreWeights::default());
        assert!(
            (s - 1.0).abs() < 1e-9,
            "quality_score should ignore aspect_ratio, got {}",
            s
        );
    }

    #[test]
    fn quality_score_renormalizes_remaining_weights() {
        let q = QualityProfile {
            aspect_ratio: 0.5, // ignored
            crossings: 0.5,
            wire_length: 0.5,
            label_ratio: 0.5,
            text_clarity: 0.5,
        };
        // With default weights: crossings=0.15, wire_length=0.10,
        // label_ratio=0.10. Sum 0.35. Renormalized contributions all
        // multiply 0.5 → 0.5 weighted average.
        let s = q.quality_score(&ScoreWeights::default());
        assert!((s - 0.5).abs() < 1e-9);
    }

    #[test]
    fn quality_profile_as_array_has_canonical_order() {
        let q = QualityProfile {
            aspect_ratio: 0.1,
            crossings: 0.2,
            wire_length: 0.3,
            label_ratio: 0.4,
            text_clarity: 0.5,
        };
        let arr = q.as_array();
        assert_eq!(arr, [0.1, 0.2, 0.3, 0.4, 0.5]);
    }

    #[test]
    fn compute_profile_quality_matches_compute_score_subscores() {
        // The two functions derive Tier 2 from the same formulas, so
        // their outputs should agree on the four continuous metrics.
        let mut r = perfect_report();
        r.bounding_box.aspect_ratio = 7.0; // ar bracket 5..10
        r.wire_crossings.crossing_count = 2; // 1/3
        r.wire_length.total_length = 200.0; // ratio 2 (1 component → ideal 100)
        r.label_usage.total_labels = 4;
        r.label_usage.label_pairs = 2;
        r.label_usage.label_to_wire_ratio = 2.0; // → 1/(1+4) = 0.2

        let breakdown = compute_score(&r, &ScoreWeights::default());
        let (_safety, quality) = compute_profile(&r);

        assert!((quality.aspect_ratio - breakdown.aspect_ratio_score).abs() < 1e-9);
        assert!((quality.crossings - breakdown.crossings_score).abs() < 1e-9);
        assert!((quality.wire_length - breakdown.wire_length_score).abs() < 1e-9);
        assert!((quality.label_ratio - breakdown.label_ratio_score).abs() < 1e-9);
    }
}
