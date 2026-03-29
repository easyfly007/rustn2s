use serde::{Serialize, Deserialize};
use super::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PowerType {
    GND,
    VDD,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub instance_name: String,
    pub symbol_name: String,
    pub position: Point,
    pub rotation: i32,
    pub mirrored: bool,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wire {
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub position: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSymbol {
    pub power_type: PowerType,
    pub net_name: String,
    pub position: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Junction {
    pub position: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schematic {
    pub title: String,
    pub components: Vec<Component>,
    pub wires: Vec<Wire>,
    pub labels: Vec<Label>,
    pub power_symbols: Vec<PowerSymbol>,
    pub junctions: Vec<Junction>,
}

impl Schematic {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.into(),
            components: Vec::new(),
            wires: Vec::new(),
            labels: Vec::new(),
            power_symbols: Vec::new(),
            junctions: Vec::new(),
        }
    }
}
