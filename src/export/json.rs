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
