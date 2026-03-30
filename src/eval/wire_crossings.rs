use serde::Serialize;
use crate::model::{Schematic, Point};

#[derive(Debug, Serialize)]
pub struct WireCrossingReport {
    pub crossing_count: usize,
    pub crossings: Vec<CrossingInfo>,
}

#[derive(Debug, Serialize)]
pub struct CrossingInfo {
    pub position: Point,
    pub wire_a: usize,
    pub wire_b: usize,
}

pub fn check(schematic: &Schematic) -> WireCrossingReport {
    // Collect all segments: (wire_index, p1, p2)
    let mut segments: Vec<(usize, Point, Point)> = Vec::new();
    for (wi, wire) in schematic.wires.iter().enumerate() {
        for k in 0..wire.points.len().saturating_sub(1) {
            segments.push((wi, wire.points[k], wire.points[k + 1]));
        }
    }

    // Collect junction positions for exclusion
    let junctions: Vec<Point> = schematic.junctions.iter().map(|j| j.position).collect();

    let mut crossings = Vec::new();
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            // Skip segments from the same wire
            if segments[i].0 == segments[j].0 { continue; }

            if let Some(pt) = segment_intersection(
                &segments[i].1, &segments[i].2,
                &segments[j].1, &segments[j].2,
            ) {
                // Exclude shared endpoints (T-junctions at junctions)
                if is_endpoint(&pt, &segments[i].1, &segments[i].2)
                    && is_endpoint(&pt, &segments[j].1, &segments[j].2)
                {
                    continue;
                }
                // Exclude known junctions
                if junctions.iter().any(|jp| close(jp, &pt)) {
                    continue;
                }
                crossings.push(CrossingInfo {
                    position: pt,
                    wire_a: segments[i].0,
                    wire_b: segments[j].0,
                });
            }
        }
    }

    WireCrossingReport {
        crossing_count: crossings.len(),
        crossings,
    }
}

fn close(a: &Point, b: &Point) -> bool {
    (a.x - b.x).abs() < 1.0 && (a.y - b.y).abs() < 1.0
}

fn is_endpoint(pt: &Point, a: &Point, b: &Point) -> bool {
    close(pt, a) || close(pt, b)
}

/// Test if two line segments intersect, return the intersection point if they do.
/// Uses the standard cross-product method.
fn segment_intersection(p1: &Point, p2: &Point, p3: &Point, p4: &Point) -> Option<Point> {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-10 {
        return None; // Parallel or collinear
    }

    let t = ((p3.x - p1.x) * d2y - (p3.y - p1.y) * d2x) / denom;
    let u = ((p3.x - p1.x) * d1y - (p3.y - p1.y) * d1x) / denom;

    // Strict interior intersection (exclude endpoints to avoid double-counting)
    let eps = 0.001;
    if t > eps && t < 1.0 - eps && u > eps && u < 1.0 - eps {
        Some(Point::new(p1.x + t * d1x, p1.y + t * d1y))
    } else {
        None
    }
}
