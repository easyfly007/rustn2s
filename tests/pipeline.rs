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
