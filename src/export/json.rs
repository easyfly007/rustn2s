use crate::model::Schematic;

/// Export schematic as structured JSON (.n2s.json)
pub fn render_to_json(schematic: &Schematic) -> Result<String, String> {
    serde_json::to_string_pretty(schematic)
        .map_err(|e| format!("JSON serialization error: {}", e))
}

pub fn render_to_file(schematic: &Schematic, path: &str) -> Result<(), String> {
    let json = render_to_json(schematic)?;
    std::fs::write(path, &json).map_err(|e| format!("Cannot write {}: {}", path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Point, Wire};

    #[test]
    fn render_empty_schematic_is_valid_json() {
        let s = Schematic::new("empty");
        let body = render_to_json(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["title"], "empty");
        assert!(v["components"].as_array().unwrap().is_empty());
    }

    #[test]
    fn render_preserves_component_and_wire_data() {
        let mut s = Schematic::new("t");
        s.components.push(Component {
            instance_name: "R1".into(),
            symbol_name: "resistor".into(),
            position: Point::new(10.0, 20.0),
            rotation: 90,
            mirrored: true,
            properties: vec![("model".into(), "1k".into())],
        });
        s.wires.push(Wire { points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)] });

        let body = render_to_json(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let comp = &v["components"][0];
        assert_eq!(comp["instance_name"], "R1");
        assert_eq!(comp["symbol_name"], "resistor");
        assert_eq!(comp["position"]["x"], 10.0);
        assert_eq!(comp["rotation"], 90);
        assert_eq!(comp["mirrored"], true);
        assert_eq!(v["wires"][0]["points"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn output_is_pretty_printed() {
        // render_to_json uses to_string_pretty → must contain newlines + indentation.
        let s = Schematic::new("t");
        let body = render_to_json(&s).unwrap();
        assert!(body.contains('\n'), "expected multi-line pretty JSON");
    }
}
