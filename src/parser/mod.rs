use std::collections::HashMap;

/// A parsed SPICE device.
#[derive(Debug, Clone)]
pub struct SpiceDevice {
    pub device_type: char,
    pub instance_name: String,
    pub nodes: Vec<String>,
    pub model_or_value: String,
    pub parameters: HashMap<String, String>,
    pub line_number: usize,
}

/// A parsed subcircuit definition.
#[derive(Debug, Clone)]
pub struct SpiceSubcircuitDef {
    pub name: String,
    pub ports: Vec<String>,
    pub devices: Vec<SpiceDevice>,
    pub parameters: HashMap<String, String>,
}

/// Result of parsing a SPICE netlist.
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub title: String,
    pub devices: Vec<SpiceDevice>,
    pub subcircuits: Vec<SpiceSubcircuitDef>,
    pub includes: Vec<String>,
    pub parameters: HashMap<String, String>,
    pub warnings: Vec<String>,
}

pub struct SpiceParser {
    warnings: Vec<String>,
}

impl SpiceParser {
    pub fn new() -> Self {
        Self { warnings: Vec::new() }
    }

    pub fn parse(&mut self, spice_text: &str) -> ParseResult {
        self.warnings.clear();
        let mut result = ParseResult {
            title: String::new(),
            devices: Vec::new(),
            subcircuits: Vec::new(),
            includes: Vec::new(),
            parameters: HashMap::new(),
            warnings: Vec::new(),
        };

        let raw_lines: Vec<&str> = spice_text.lines().collect();
        let merged = Self::merge_continuation_lines(&raw_lines);

        if merged.is_empty() {
            return result;
        }

        // First line is title (SPICE convention)
        let first = &merged[0];
        let start_idx;
        if first.starts_with("* ") {
            result.title = first[2..].trim().to_string();
            start_idx = 1;
        } else if first.starts_with('*') {
            result.title = first[1..].trim().to_string();
            start_idx = 1;
        } else if !first.starts_with('.') && !Self::is_device_line(first) {
            result.title = first.trim().to_string();
            start_idx = 1;
        } else {
            start_idx = 0;
        }

        let mut subckt_stack: Vec<SpiceSubcircuitDef> = Vec::new();
        let mut line_num = 0usize;

        for line in &merged[start_idx..] {
            line_num += 1;
            let tokens = Self::tokenize(line);
            if tokens.is_empty() {
                continue;
            }

            let first_lower = tokens[0].to_lowercase();

            // Directives
            if first_lower == ".subckt" {
                let mut subckt = SpiceSubcircuitDef {
                    name: tokens.get(1).cloned().unwrap_or_default(),
                    ports: Vec::new(),
                    devices: Vec::new(),
                    parameters: HashMap::new(),
                };
                for tok in &tokens[2..] {
                    if tok.contains('=') {
                        Self::parse_param(tok, &mut subckt.parameters);
                    } else {
                        subckt.ports.push(tok.clone());
                    }
                }
                subckt_stack.push(subckt);
                continue;
            }

            if first_lower == ".ends" {
                if let Some(subckt) = subckt_stack.pop() {
                    result.subcircuits.push(subckt);
                }
                continue;
            }

            if first_lower == ".param" {
                let target = if let Some(s) = subckt_stack.last_mut() {
                    &mut s.parameters
                } else {
                    &mut result.parameters
                };
                for tok in &tokens[1..] {
                    if tok.contains('=') {
                        Self::parse_param(tok, target);
                    }
                }
                continue;
            }

            if first_lower == ".include" || first_lower == ".lib" {
                if let Some(path) = tokens.get(1) {
                    let p = path.trim_matches('"').trim_matches('\'').to_string();
                    result.includes.push(p);
                }
                continue;
            }

            if first_lower.starts_with('.') {
                continue;
            }

            // Device lines
            if Self::is_device_line(&tokens[0]) {
                let device = self.parse_device_line(&tokens, line_num);
                if let Some(s) = subckt_stack.last_mut() {
                    s.devices.push(device);
                } else {
                    result.devices.push(device);
                }
                continue;
            }

            self.warnings.push(format!("Line {}: unrecognized: {}", line_num, line));
        }

        // Unclosed subcircuits
        while let Some(subckt) = subckt_stack.pop() {
            self.warnings.push(format!("Unclosed .subckt: {}", subckt.name));
            result.subcircuits.push(subckt);
        }

        result.warnings = self.warnings.clone();
        result
    }

    pub fn parse_file(&mut self, path: &str) -> ParseResult {
        match std::fs::read_to_string(path) {
            Ok(text) => self.parse(&text),
            Err(e) => {
                let mut r = ParseResult {
                    title: String::new(), devices: Vec::new(), subcircuits: Vec::new(),
                    includes: Vec::new(), parameters: HashMap::new(),
                    warnings: vec![format!("Cannot open file: {}: {}", path, e)],
                };
                r.warnings = r.warnings.clone();
                r
            }
        }
    }

    // ========================================================================
    // NMOS/PMOS inference
    // ========================================================================

    pub fn infer_mos_type(device: &SpiceDevice) -> &'static str {
        let model = device.model_or_value.to_lowercase();
        if model.contains("nch") || model.contains("nmos") || model == "n" {
            return "nmos4";
        }
        if model.contains("pch") || model.contains("pmos") || model == "p" {
            return "pmos4";
        }
        // Bulk connection heuristic (SPICE order: D G S B → index 3)
        if device.nodes.len() >= 4 {
            let bulk = device.nodes[3].to_lowercase();
            if matches!(bulk.as_str(), "0" | "gnd" | "vss" | "gnd!") {
                return "nmos4";
            }
            if matches!(bulk.as_str(), "vdd" | "vdd!" | "vcc" | "avdd") {
                return "pmos4";
            }
        }
        "nmos4"
    }

    pub fn infer_bjt_type(device: &SpiceDevice) -> &'static str {
        let model = device.model_or_value.to_lowercase();
        if model.contains("pnp") || model == "p" {
            return "pnp";
        }
        "npn"
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    fn merge_continuation_lines(lines: &[&str]) -> Vec<String> {
        let mut merged: Vec<String> = Vec::new();
        let mut first_line = true;

        for raw in lines {
            let line = *raw;

            if first_line {
                first_line = false;
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    merged.push(trimmed.to_string());
                }
                continue;
            }

            // Strip inline comments
            let line = if let Some(idx) = line.find('$') { &line[..idx] } else { line };
            let line = if let Some(idx) = line.find(';') { &line[..idx] } else { line };
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if trimmed.starts_with('*') { continue; }

            if trimmed.starts_with('+') {
                if let Some(last) = merged.last_mut() {
                    last.push(' ');
                    last.push_str(trimmed[1..].trim());
                }
                continue;
            }

            merged.push(trimmed.to_string());
        }

        merged
    }

    fn tokenize(line: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;
        let mut quote_char = ' ';

        for ch in line.chars() {
            if in_quote {
                current.push(ch);
                if ch == quote_char {
                    in_quote = false;
                }
            } else if ch == '"' || ch == '\'' {
                in_quote = true;
                quote_char = ch;
                current.push(ch);
            } else if ch == ' ' || ch == '\t' || ch == ',' {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }
        tokens
    }

    fn is_device_line(first_token: &str) -> bool {
        if first_token.is_empty() { return false; }
        matches!(
            first_token.chars().next().unwrap().to_ascii_uppercase(),
            'M' | 'R' | 'C' | 'L' | 'D' | 'Q' | 'V' | 'I' | 'E' | 'F' | 'G' | 'H' | 'X'
        )
    }

    fn parse_device_line(&mut self, tokens: &[String], line_number: usize) -> SpiceDevice {
        let mut device = SpiceDevice {
            instance_name: tokens[0].clone(),
            device_type: tokens[0].chars().next().unwrap().to_ascii_uppercase(),
            nodes: Vec::new(),
            model_or_value: String::new(),
            parameters: HashMap::new(),
            line_number,
        };

        if tokens.len() < 2 {
            self.warnings.push(format!("Line {}: device line too short", line_number));
            return device;
        }

        match device.device_type {
            'X' => {
                // Subcircuit: X1 node1 node2 ... subckt_name [param=val ...]
                let mut param_start = tokens.len();
                for i in (1..tokens.len()).rev() {
                    if tokens[i].contains('=') {
                        param_start = i;
                    } else {
                        break;
                    }
                }
                for tok in &tokens[param_start..] {
                    Self::parse_param(tok, &mut device.parameters);
                }
                if param_start >= 2 {
                    device.model_or_value = tokens[param_start - 1].clone();
                    for tok in &tokens[1..param_start - 1] {
                        device.nodes.push(tok.clone());
                    }
                }
            }
            'M' => {
                // MOSFET: M1 drain gate source bulk model [params]
                if tokens.len() < 6 {
                    self.warnings.push(format!("Line {}: MOSFET needs at least 6 tokens", line_number));
                    for tok in &tokens[1..] { device.nodes.push(tok.clone()); }
                    return device;
                }
                for i in 1..=4 { device.nodes.push(tokens[i].clone()); }
                device.model_or_value = tokens[5].clone();
                for tok in &tokens[6..] { Self::parse_param(tok, &mut device.parameters); }
            }
            'Q' => {
                // BJT: Q1 C B E [substrate] model [params]
                if tokens.len() < 5 {
                    self.warnings.push(format!("Line {}: BJT needs at least 5 tokens", line_number));
                    for tok in &tokens[1..] { device.nodes.push(tok.clone()); }
                    return device;
                }
                for i in 1..=3 { device.nodes.push(tokens[i].clone()); }
                let mut next = 4;
                // Check for 4-terminal BJT
                if next < tokens.len() && !tokens[next].contains('=')
                    && next + 1 < tokens.len() && !tokens[next + 1].contains('=')
                {
                    device.nodes.push(tokens[next].clone());
                    next += 1;
                }
                if next < tokens.len() && !tokens[next].contains('=') {
                    device.model_or_value = tokens[next].clone();
                    next += 1;
                }
                for tok in &tokens[next..] { Self::parse_param(tok, &mut device.parameters); }
            }
            'E' | 'G' | 'H' | 'F' => {
                // Controlled sources: 4 nodes + gain
                if tokens.len() < 6 {
                    self.warnings.push(format!("Line {}: controlled source needs at least 6 tokens", line_number));
                    for tok in &tokens[1..] { device.nodes.push(tok.clone()); }
                    return device;
                }
                for i in 1..=4 { device.nodes.push(tokens[i].clone()); }
                device.model_or_value = tokens[5].clone();
                for tok in &tokens[6..] { Self::parse_param(tok, &mut device.parameters); }
            }
            _ => {
                // 2-node: R, C, L, D, V, I
                if tokens.len() < 3 {
                    self.warnings.push(format!("Line {}: device needs at least 3 tokens", line_number));
                    return device;
                }
                device.nodes.push(tokens[1].clone());
                device.nodes.push(tokens[2].clone());
                if tokens.len() >= 4 && !tokens[3].contains('=') {
                    device.model_or_value = tokens[3].clone();
                    for tok in &tokens[4..] { Self::parse_param(tok, &mut device.parameters); }
                } else {
                    for tok in &tokens[3..] { Self::parse_param(tok, &mut device.parameters); }
                }
            }
        }

        device
    }

    fn parse_param(token: &str, params: &mut HashMap<String, String>) {
        if let Some(eq_idx) = token.find('=') {
            if eq_idx > 0 {
                let key = token[..eq_idx].trim().to_string();
                let value = token[eq_idx + 1..].trim().to_string();
                params.insert(key, value);
            }
        }
    }
}

/// Pin names in SPICE node order for each symbol type.
pub fn pin_names_for_symbol(symbol_name: &str) -> Vec<&'static str> {
    match symbol_name {
        "nmos4" | "pmos4" => vec!["D", "G", "S", "B"],
        "npn" | "pnp" => vec!["C", "B", "E"],
        "diode" => vec!["A", "K"],
        "vcvs" | "vccs" | "ccvs" | "cccs" => vec!["NP", "NN", "CP", "CN"],
        _ => vec!["P", "N"],  // resistor, capacitor, inductor, vsource, isource
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inverter() {
        let spice = "* CMOS Inverter\nM1 out in vdd vdd pch W=20u L=1u\nM2 out in 0 0 nch W=10u L=1u\n";
        let mut parser = SpiceParser::new();
        let result = parser.parse(spice);
        assert_eq!(result.title, "CMOS Inverter");
        assert_eq!(result.devices.len(), 2);
        assert_eq!(result.devices[0].instance_name, "M1");
        assert_eq!(result.devices[0].nodes, vec!["out", "in", "vdd", "vdd"]);
        assert_eq!(result.devices[0].model_or_value, "pch");
        assert_eq!(result.devices[1].model_or_value, "nch");
        assert_eq!(SpiceParser::infer_mos_type(&result.devices[0]), "pmos4");
        assert_eq!(SpiceParser::infer_mos_type(&result.devices[1]), "nmos4");
    }

    #[test]
    fn test_parse_diff_pair() {
        let spice = "* Differential Pair\nM1 out1 inp tail 0 nch W=10u L=1u\nM2 out2 inm tail 0 nch W=10u L=1u\nM3 tail bias 0 0 nch W=20u L=2u\nR1 vdd out1 10k\nR2 vdd out2 10k\n";
        let mut parser = SpiceParser::new();
        let result = parser.parse(spice);
        assert_eq!(result.devices.len(), 5);
        assert_eq!(result.devices[3].device_type, 'R');
        assert_eq!(result.devices[3].model_or_value, "10k");
    }
}
