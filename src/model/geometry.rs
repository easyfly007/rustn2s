use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    /// Apply rotation (degrees) and optional mirroring around Y axis.
    pub fn transform(self, rotation: i32, mirrored: bool) -> Point {
        let mut x = self.x;
        let y = self.y;
        if mirrored {
            x = -x;
        }
        let rad = (rotation as f64) * std::f64::consts::PI / 180.0;
        let cos_r = rad.cos();
        let sin_r = rad.sin();
        Point {
            x: x * cos_r - y * sin_r,
            y: x * sin_r + y * cos_r,
        }
    }

    pub fn snap_to_grid(self, grid: f64) -> Point {
        if grid <= 0.0 {
            return self;
        }
        Point {
            x: (self.x / grid).round() * grid,
            y: (self.y / grid).round() * grid,
        }
    }
}

impl std::ops::Add for Point {
    type Output = Point;
    fn add(self, rhs: Point) -> Point {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            x,
            y,
            width: w,
            height: h,
        }
    }

    pub fn from_points(min: Point, max: Point) -> Self {
        Self {
            x: min.x,
            y: min.y,
            width: max.x - min.x,
            height: max.y - min.y,
        }
    }

    pub fn left(&self) -> f64 {
        self.x
    }
    pub fn top(&self) -> f64 {
        self.y
    }
    pub fn right(&self) -> f64 {
        self.x + self.width
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn distance_to_is_euclidean() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!(approx(a.distance_to(&b), 5.0));
    }

    #[test]
    fn transform_zero_rotation_is_identity() {
        let p = Point::new(7.0, -2.0);
        let q = p.transform(0, false);
        assert!(approx(p.x, q.x) && approx(p.y, q.y));
    }

    #[test]
    fn transform_mirror_flips_x() {
        let p = Point::new(5.0, 3.0).transform(0, true);
        assert!(approx(p.x, -5.0) && approx(p.y, 3.0));
    }

    #[test]
    fn transform_90_rotates_ccw() {
        // Standard math convention: (1,0) rotated +90° → (0,1)
        let p = Point::new(1.0, 0.0).transform(90, false);
        assert!(approx(p.x, 0.0) && approx(p.y, 1.0));
    }

    #[test]
    fn snap_to_grid_rounds_to_nearest() {
        let p = Point::new(13.0, -27.0).snap_to_grid(10.0);
        assert!(approx(p.x, 10.0) && approx(p.y, -30.0));
    }

    #[test]
    fn snap_to_grid_zero_or_negative_is_passthrough() {
        let p = Point::new(13.0, -27.0);
        let q = p.snap_to_grid(0.0);
        assert!(approx(p.x, q.x) && approx(p.y, q.y));
        let r = p.snap_to_grid(-5.0);
        assert!(approx(p.x, r.x) && approx(p.y, r.y));
    }

    #[test]
    fn point_addition() {
        let s = Point::new(1.0, 2.0) + Point::new(3.0, 5.0);
        assert!(approx(s.x, 4.0) && approx(s.y, 7.0));
    }

    #[test]
    fn rect_from_points_uses_min_max() {
        let r = Rect::from_points(Point::new(1.0, 2.0), Point::new(4.0, 7.0));
        assert!(approx(r.left(), 1.0));
        assert!(approx(r.top(), 2.0));
        assert!(approx(r.right(), 4.0));
        assert!(approx(r.bottom(), 7.0));
        assert!(approx(r.width, 3.0));
        assert!(approx(r.height, 5.0));
    }
}
