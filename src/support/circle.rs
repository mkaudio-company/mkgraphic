//! Circle type for 2D circular regions.

use super::point::Point;
use super::rect::Rect;

/// A circle defined by its center and radius.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Circle {
    pub center: Point,
    pub radius: f32,
}

impl Circle {
    /// Creates a new circle.
    #[inline]
    pub const fn new(center: Point, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Creates a circle from center coordinates and radius.
    #[inline]
    pub const fn from_coords(cx: f32, cy: f32, radius: f32) -> Self {
        Self {
            center: Point::new(cx, cy),
            radius,
        }
    }

    /// Returns the bounding rectangle of the circle.
    #[inline]
    pub fn bounds(&self) -> Rect {
        Rect {
            left: self.center.x - self.radius,
            top: self.center.y - self.radius,
            right: self.center.x + self.radius,
            bottom: self.center.y + self.radius,
        }
    }

    /// Returns the diameter of the circle.
    #[inline]
    pub fn diameter(&self) -> f32 {
        self.radius * 2.0
    }

    /// Returns the circumference of the circle.
    #[inline]
    pub fn circumference(&self) -> f32 {
        2.0 * std::f32::consts::PI * self.radius
    }

    /// Returns the area of the circle.
    #[inline]
    pub fn area(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius
    }

    /// Returns true if the point is inside the circle.
    #[inline]
    pub fn contains(&self, p: Point) -> bool {
        self.center.distance_squared_to(p) <= self.radius * self.radius
    }

    /// Returns true if this circle intersects with another circle.
    #[inline]
    pub fn intersects(&self, other: &Circle) -> bool {
        let max_dist = self.radius + other.radius;
        self.center.distance_squared_to(other.center) <= max_dist * max_dist
    }

    /// Moves the circle by the given delta.
    #[inline]
    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self {
            center: self.center.translate(dx, dy),
            radius: self.radius,
        }
    }

    /// Scales the circle by the given factor.
    #[inline]
    pub fn scale(self, factor: f32) -> Self {
        Self {
            center: self.center,
            radius: self.radius * factor,
        }
    }
}

/// Creates a circle that inscribes the given rectangle.
pub fn inscribed_circle(rect: &Rect) -> Circle {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) / 2.0;
    Circle::new(center, radius)
}

/// Creates a circle that circumscribes the given rectangle.
pub fn circumscribed_circle(rect: &Rect) -> Circle {
    let center = rect.center();
    let radius = center.distance_to(rect.top_left());
    Circle::new(center, radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_basic() {
        let c = Circle::from_coords(0.0, 0.0, 10.0);
        assert_eq!(c.diameter(), 20.0);
        assert!((c.circumference() - 62.83).abs() < 0.1);
        assert!((c.area() - 314.16).abs() < 0.1);
    }

    #[test]
    fn test_circle_contains() {
        let c = Circle::from_coords(0.0, 0.0, 10.0);
        assert!(c.contains(Point::new(0.0, 0.0)));
        assert!(c.contains(Point::new(5.0, 5.0)));
        assert!(!c.contains(Point::new(10.0, 10.0)));
    }

    #[test]
    fn test_circle_bounds() {
        let c = Circle::from_coords(10.0, 20.0, 5.0);
        let b = c.bounds();
        assert_eq!(b.left, 5.0);
        assert_eq!(b.top, 15.0);
        assert_eq!(b.right, 15.0);
        assert_eq!(b.bottom, 25.0);
    }
}
