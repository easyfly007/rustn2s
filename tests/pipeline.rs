//! End-to-end pipeline tests.
//!
//! These exercise `convert_full` (parse → analyze → place → route) on every
//! SPICE example under `tests/examples/`, then run the eval module against
//! the resulting schematic. The asserts are loose — they verify the pipeline
//! produces a structurally sane schematic, not a specific score. Tighter
//! score regression guards belong in a separate benchmark.

use n2s::{convert_full, ConvertOptions};
use n2s::eval::{evaluate, score::{compute_score, ScoreWeights}};
use n2s::parser::SpiceParser;

fn read_example(name: &str) -> String {
    let path = format!("tests/examples/{}", name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path, e))
}

fn run_pipeline(spice: &str) -> n2s::model::Schematic {
    let opts = ConvertOptions::default();
    convert_full(spice, &opts)
        .expect("pipeline must succeed for valid SPICE")
        .schematic
}

#[test]
fn empty_input_errors_cleanly() {
    let opts = ConvertOptions::default();
    match convert_full("* just a comment\n", &opts) {
        Ok(_) => panic!("expected error for input with no devices"),
        Err(e) => assert!(e.to_lowercase().contains("no devices"), "wrong error: {}", e),
    }
}

#[test]
fn voltage_divider_pipeline_produces_components() {
    let s = run_pipeline(&read_example("01_voltage_divider.sp"));
    // V1 + R1 + R2 = 3 components
    assert_eq!(s.components.len(), 3);
    let names: Vec<&str> = s.components.iter().map(|c| c.instance_name.as_str()).collect();
    for n in ["V1", "R1", "R2"] {
        assert!(names.contains(&n), "missing {} in {:?}", n, names);
    }
}

#[test]
fn rc_lowpass_creates_power_symbol_for_ground() {
    let s = run_pipeline(&read_example("02_rc_lowpass_filter.sp"));
    // Net "0" is GND → expect at least one PowerSymbol
    assert!(!s.power_symbols.is_empty(), "expected at least one power symbol for GND");
}

#[test]
fn diff_pair_pipeline_pairs_devices_symmetrically() {
    let s = run_pipeline(&read_example("06_bjt_diff_pair.sp"));
    // Q1 and Q2 are matched NPN; placer should put them at the same y.
    let q1 = s.components.iter().find(|c| c.instance_name == "Q1").expect("Q1 missing");
    let q2 = s.components.iter().find(|c| c.instance_name == "Q2").expect("Q2 missing");
    assert!(
        (q1.position.y - q2.position.y).abs() < 1.0,
        "Q1 and Q2 should be at the same y — got {} vs {}",
        q1.position.y, q2.position.y,
    );
}

#[test]
fn all_examples_pass_tier1_safety_metrics() {
    // After the 2026-05-01 metric-reform analysis (docs/metric_reform.md)
    // the score system is conceptually split into two tiers:
    //   Tier 1 (safety):  overlap, symmetry, power_convention — these
    //                     should always be 1.0 on a correct pipeline.
    //                     Anything below 1.0 means a real bug like
    //                     overlapping components or PMOS-below-NMOS.
    //   Tier 2 (quality): aspect_ratio, crossings, wire_length,
    //                     label_ratio — continuous metrics, vary per
    //                     circuit.
    // This test guards Tier 1 across all 25 example circuits. Bugs 1
    // and 2 in the expansion findings each broke this on at least one
    // circuit; we want a regression alert if either resurfaces.
    use n2s::eval::score::{compute_score, ScoreWeights};

    let examples = std::fs::read_dir("tests/examples")
        .expect("tests/examples must exist")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "sp"))
        .map(|e| e.file_name().into_string().unwrap())
        .collect::<Vec<_>>();

    let weights = ScoreWeights::default();
    let mut failures: Vec<String> = Vec::new();

    for name in &examples {
        let spice = read_example(name);
        let schematic = run_pipeline(&spice);
        let parsed = SpiceParser::new().parse(&spice);
        let report = n2s::eval::evaluate(&parsed, &schematic);
        let breakdown = compute_score(&report, &weights);

        if breakdown.overlap_score < 0.999 {
            failures.push(format!("{}: overlap < 1.0 ({:.3})", name, breakdown.overlap_score));
        }
        if breakdown.symmetry_score < 0.999 {
            failures.push(format!("{}: symmetry < 1.0 ({:.3})", name, breakdown.symmetry_score));
        }
        if breakdown.power_convention_score < 0.999 {
            failures.push(format!("{}: power_convention < 1.0 ({:.3})", name, breakdown.power_convention_score));
        }
    }

    assert!(failures.is_empty(),
        "Tier 1 safety metrics regressed on {} circuit(s):\n  {}",
        failures.len(), failures.join("\n  "));
}

#[test]
fn all_examples_run_through_pipeline_and_eval() {
    let examples = std::fs::read_dir("tests/examples")
        .expect("tests/examples must exist")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "sp"))
        .map(|e| e.file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    assert!(examples.len() >= 11, "expected ≥11 example netlists, found {}", examples.len());

    for name in &examples {
        let spice = read_example(name);
        let schematic = run_pipeline(&spice);
        assert!(!schematic.components.is_empty(),
            "{}: schematic has no components", name);

        // Eval must run without panicking and produce a valid score in [0, 1].
        let parsed = SpiceParser::new().parse(&spice);
        let report = evaluate(&parsed, &schematic);
        let score = compute_score(&report, &ScoreWeights::default());
        assert!(score.overall >= 0.0 && score.overall <= 1.0,
            "{}: score {} out of range", name, score.overall);
    }
}

#[test]
fn pipeline_is_deterministic_across_runs() {
    // Same input → same component count, wire count, label count, power-symbol count.
    // (We don't compare positions because HashMap iteration ordering can shuffle
    //  ties in the placer/router, but counts are stable.)
    let spice = read_example("04_nmos_common_source.sp");
    let s1 = run_pipeline(&spice);
    let s2 = run_pipeline(&spice);
    assert_eq!(s1.components.len(), s2.components.len());
    assert_eq!(s1.power_symbols.len(), s2.power_symbols.len());
}

#[test]
fn export_json_round_trip() {
    use n2s::export::json;
    let s = run_pipeline(&read_example("01_voltage_divider.sp"));
    let serialized = serde_json::to_string(&s).expect("serialize");
    let parsed: n2s::model::Schematic =
        serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(s.components.len(), parsed.components.len());
    assert_eq!(s.wires.len(), parsed.wires.len());

    // Also confirm the public render-to-file helper writes valid JSON.
    let tmp = std::env::temp_dir().join("n2s_test_voltage_divider.json");
    json::render_to_file(&s, tmp.to_str().unwrap()).expect("render_to_file");
    let body = std::fs::read_to_string(&tmp).expect("read back");
    let _: serde_json::Value = serde_json::from_str(&body).expect("output is valid JSON");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn obstacle_avoidance_keeps_wires_off_component_bodies() {
    // For example 04 (NMOS common-source), the default L-router walks at
    // least one wire straight through a transistor body. With A* obstacle
    // avoidance enabled, no signal-net wire should pass through any
    // component's bounding rect.
    use n2s::model::{builtin_symbols, Point};
    let spice = read_example("04_nmos_common_source.sp");

    let opts_off = ConvertOptions {
        router: n2s::router::RouterOptions {
            avoid_obstacles: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let opts_on = ConvertOptions {
        router: n2s::router::RouterOptions {
            avoid_obstacles: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let s_off = convert_full(&spice, &opts_off).unwrap().schematic;
    let s_on = convert_full(&spice, &opts_on).unwrap().schematic;

    // Build an obstacle map per component by transforming each builtin
    // symbol's bounding rect into world space.
    let symbols = builtin_symbols::all();
    let body_rects: Vec<(f64, f64, f64, f64)> = s_on.components.iter()
        .filter_map(|c| {
            let sym = symbols.get(&c.symbol_name)?;
            let base = sym.bounding_rect();
            let corners = [
                Point::new(base.left(), base.top()),
                Point::new(base.right(), base.top()),
                Point::new(base.left(), base.bottom()),
                Point::new(base.right(), base.bottom()),
            ];
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;
            for cp in &corners {
                let world = c.position + cp.transform(c.rotation, c.mirrored);
                min_x = min_x.min(world.x);
                min_y = min_y.min(world.y);
                max_x = max_x.max(world.x);
                max_y = max_y.max(world.y);
            }
            Some((min_x, min_y, max_x, max_y))
        })
        .collect();

    // Sample each wire's interior at fine resolution; count how many sample
    // points land strictly inside any component body (excluding endpoints).
    fn count_body_intrusions(
        wires: &[n2s::model::Wire], rects: &[(f64, f64, f64, f64)],
    ) -> usize {
        let mut count = 0;
        for wire in wires {
            for window in wire.points.windows(2) {
                let a = window[0];
                let b = window[1];
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-6 { continue; }
                let steps = (len / 2.0).ceil() as usize;
                for s in 1..steps {
                    let t = s as f64 / steps as f64;
                    let p = Point::new(a.x + dx * t, a.y + dy * t);
                    for &(min_x, min_y, max_x, max_y) in rects {
                        if p.x > min_x + 0.5 && p.x < max_x - 0.5
                            && p.y > min_y + 0.5 && p.y < max_y - 0.5
                        {
                            count += 1;
                            break;
                        }
                    }
                }
            }
        }
        count
    }

    let off_intrusions = count_body_intrusions(&s_off.wires, &body_rects);
    let on_intrusions = count_body_intrusions(&s_on.wires, &body_rects);

    // The whole point of A*: drive intrusions toward zero. We require strict
    // improvement on this circuit, where the L-router has known intrusions.
    assert!(off_intrusions > 0,
        "expected L-router to walk through at least one body in example 04 \
         (otherwise this test isn't measuring anything)");
    assert!(on_intrusions < off_intrusions,
        "A* did not reduce body intrusions: off={} on={}",
        off_intrusions, on_intrusions);
}

#[test]
fn hierarchical_netlist_renders_top_level_by_default() {
    // Bug 3 in the 2026-05-01 expansion findings: when a netlist had
    // both .subckt definitions and top-level X instances, the default
    // pipeline silently rendered only the first subckt's interior,
    // dropping all top-level content (including X instances, V sources,
    // and load components). The fix is to auto-enable hierarchical
    // mode whenever has_x_instances is true.
    //
    // 25_subckt_array.sp has eight top-level items: V1, V2, three X
    // instances (X1/X2/X3 of INV), and three load caps (C1/C2/C3).
    // Default mode must render all eight.
    let spice = read_example("25_subckt_array.sp");
    let s = run_pipeline(&spice);
    assert_eq!(s.components.len(), 8,
        "default mode should render the eight top-level items, got: {:?}",
        s.components.iter().map(|c| &c.instance_name).collect::<Vec<_>>());
    let names: Vec<&str> = s.components.iter()
        .map(|c| c.instance_name.as_str()).collect();
    for n in ["V1", "V2", "X1", "X2", "X3", "C1", "C2", "C3"] {
        assert!(names.contains(&n), "missing top-level device {}", n);
    }
    // X instances should render as subcircuit boxes (subckt_INV symbol),
    // not as their internal MOSFETs.
    assert!(s.components.iter().any(|c| c.symbol_name == "subckt_INV"),
        "X instances should render as subckt_INV boxes by default");
}

#[test]
fn standalone_subckt_definition_still_renders_interior() {
    // The companion case: a netlist that defines a subckt without
    // instantiating it at the top level. This is a valid library-style
    // input — the user wants to view the subckt itself.
    let spice = "* lib-style: just a subckt definition, no top-level uses\n\
                 .subckt INV in out vdd vss\n\
                 M1 out in vdd vdd pch W=4u L=0.18u\n\
                 M2 out in vss vss nch W=2u L=0.18u\n\
                 .ends INV\n";
    let s = run_pipeline(spice);
    // Two MOSFETs from inside INV.
    assert_eq!(s.components.len(), 2);
    let symbols: Vec<&str> = s.components.iter()
        .map(|c| c.symbol_name.as_str()).collect();
    assert!(symbols.contains(&"pmos4"));
    assert!(symbols.contains(&"nmos4"));
}

#[test]
fn adaptive_label_ratio_yields_fewer_labels_on_large_circuit() {
    // The two-stage opamp has bbox diagonal ~1250. With the adaptive
    // floor disabled (ratio = 0) the absolute threshold (default 300)
    // promotes several mid-distance edges to labels. With ratio = 0.8
    // the effective threshold becomes ~1000, so the same edges stay as
    // wires. The invariant is just: more aggressive ratio → strictly
    // fewer labels on a large circuit.
    let spice = read_example("07_two_stage_opamp.sp");

    let opts_off = ConvertOptions {
        router: n2s::router::RouterOptions {
            adaptive_label_ratio: 0.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let opts_high = ConvertOptions {
        router: n2s::router::RouterOptions {
            adaptive_label_ratio: 0.8,
            ..Default::default()
        },
        ..Default::default()
    };

    let s_off = convert_full(&spice, &opts_off).unwrap().schematic;
    let s_high = convert_full(&spice, &opts_high).unwrap().schematic;

    assert!(s_high.labels.len() < s_off.labels.len(),
        "expected ratio=0.8 to use fewer labels than ratio=0; got {} vs {}",
        s_high.labels.len(), s_off.labels.len());
}

#[test]
fn adaptive_label_ratio_is_no_op_on_small_circuit() {
    // The voltage divider has bbox diagonal ~230, so even ratio=1.0
    // doesn't push the effective threshold above the user default
    // (300). Output should be identical regardless.
    let spice = read_example("01_voltage_divider.sp");

    let opts_off = ConvertOptions {
        router: n2s::router::RouterOptions {
            adaptive_label_ratio: 0.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let opts_high = ConvertOptions {
        router: n2s::router::RouterOptions {
            adaptive_label_ratio: 1.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let s_off = convert_full(&spice, &opts_off).unwrap().schematic;
    let s_high = convert_full(&spice, &opts_high).unwrap().schematic;

    assert_eq!(s_off.labels.len(), s_high.labels.len(),
        "adaptive ratio should not change small-circuit label count");
    assert_eq!(s_off.wires.len(), s_high.wires.len());
}

#[test]
fn improve_lex_min_reports_worst_subscore_as_final() {
    // In --lex-min mode the optimizer compares configurations by the
    // worst Tier 2 sub-score (worst-first lexicographic). The reported
    // final_score should therefore be ≤ the legacy weighted-sum
    // overall, because the worst sub-score is by definition no greater
    // than the weighted average. Circuit 02 (RC filter) has its
    // aspect_ratio sub-score at ~0.22 — much lower than the weighted
    // overall ~0.84. The lex-min final_score should reflect the
    // weakest dim, not the average.
    let bin = env!("CARGO_BIN_EXE_n2s-improve");
    let out = std::process::Command::new(bin)
        .args([
            "tests/examples/02_rc_lowpass_filter.sp",
            "--lex-min",
            "--max-iter", "3",
            "--target-score", "0.99",
            "--quiet",
        ])
        .output()
        .expect("invoke n2s-improve");
    assert!(out.status.success());
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let final_score = report["final_score"].as_f64().unwrap();
    // The legacy weighted sum on 02 lands around 0.84 in default mode.
    // The lex-min worst-sub-score should be much lower (~0.22 from
    // aspect_ratio alone). Asserting <= 0.5 is conservative.
    assert!(final_score <= 0.5,
        "lex-min final_score should reflect the worst sub-score (~0.22), \
         got {} — looks like the comparator wasn't wired through",
        final_score);
}

#[test]
fn improve_search_runs_multiple_restarts_when_target_unreachable() {
    // Drive n2s-improve at a target score that example 03 (halfwave
    // rectifier) cannot hit (it's a 4-device linear circuit, capped well
    // below 0.95 by aspect-ratio inherent limits). With --search, the
    // tool should run the configured number of restarts and pick the
    // best across them.
    let bin = env!("CARGO_BIN_EXE_n2s-improve");
    let out = std::process::Command::new(bin)
        .args([
            "tests/examples/03_halfwave_rectifier.sp",
            "--search", "--search-restarts", "3",
            "--max-iter", "3",
            "--target-score", "0.99",
            "--quiet",
        ])
        .output()
        .expect("invoke n2s-improve");
    assert!(out.status.success(),
        "n2s-improve failed: {}", String::from_utf8_lossy(&out.stderr));

    let report: serde_json::Value = serde_json::from_slice(&out.stdout)
        .expect("stdout should be valid JSON");
    assert_eq!(report["restarts"], 3, "expected 3 restarts, got {}", report["restarts"]);
    let summaries = report["restart_summary"].as_array().unwrap();
    assert_eq!(summaries.len(), 3);
    // Each restart records its starting params + best score
    for s in summaries {
        assert!(s["initial_params"].is_object());
        assert!(s["best_score"].is_number());
        assert!(s["iterations"].is_number());
    }
    // Final score is the best across all restarts
    let final_score = report["final_score"].as_f64().unwrap();
    let max_per_restart = summaries.iter()
        .map(|s| s["best_score"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((final_score - max_per_restart).abs() < 1e-3,
        "final_score {} should match max across restarts {}",
        final_score, max_per_restart);
}

#[test]
fn improve_without_search_runs_a_single_restart() {
    let bin = env!("CARGO_BIN_EXE_n2s-improve");
    let out = std::process::Command::new(bin)
        .args([
            "tests/examples/03_halfwave_rectifier.sp",
            "--max-iter", "3",
            "--target-score", "0.99",
            "--quiet",
        ])
        .output()
        .expect("invoke n2s-improve");
    assert!(out.status.success());
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["restarts"], 1);
    assert_eq!(report["restart_summary"].as_array().unwrap().len(), 1);
}

#[test]
fn export_kicad_writes_kicad_sch_envelope() {
    use n2s::export::kicad;
    use std::collections::HashMap;
    let s = run_pipeline(&read_example("01_voltage_divider.sp"));
    let tmp = std::env::temp_dir().join("n2s_test_voltage_divider.kicad_sch");
    let extra: HashMap<String, n2s::model::SymbolDef> = HashMap::new();
    kicad::render_to_file(&s, tmp.to_str().unwrap(), &extra).expect("render_to_file");
    let body = std::fs::read_to_string(&tmp).expect("read back");
    // KiCad files are S-expressions; the top form must be (kicad_sch ...).
    assert!(body.trim_start().starts_with("(kicad_sch"),
        "expected (kicad_sch top-level form, got: {}",
        &body[..50.min(body.len())]);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_svg_writes_well_formed_xml() {
    use n2s::export::svg;
    let s = run_pipeline(&read_example("01_voltage_divider.sp"));
    let tmp = std::env::temp_dir().join("n2s_test_voltage_divider.svg");
    svg::render_to_file(&s, tmp.to_str().unwrap(), &svg::SvgOptions::default())
        .expect("render_to_file");
    let body = std::fs::read_to_string(&tmp).expect("read back");
    assert!(body.starts_with("<?xml") || body.starts_with("<svg"),
        "SVG output should start with <?xml or <svg, got: {}", &body[..50.min(body.len())]);
    assert!(body.contains("</svg>"), "SVG output should close the <svg> tag");
    let _ = std::fs::remove_file(&tmp);
}
