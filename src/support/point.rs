//! Point and extent types for 2D coordinates.

use std::ops::{Add, Sub, Mul, Div, Neg, Index, IndexMut};

/// Represents an axis in 2D space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
}

impl Axis {
    /// Returns the other axis.
    #[inline]
    pub fn other(self) -> Self {
        match self {
            Axis::X => Axis::Y,
            Axis::Y => Axis::X,
        }
    }
}

/// A 2D point with x and y coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Creates a new point.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a point at the origin (0, 0).
    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Moves the point by the given delta.
    #[inline]
    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    /// Moves the point to the given coordinates.
    #[inline]
    pub fn move_to(self, x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the distance to another point.
    #[inline]
    pub fn distance_to(self, other: Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Returns the squared distance to another point (faster than distance_to).
    #[inline]
    pub fn distance_squared_to(self, other: Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

impl Index<Axis> for Point {
    type Output = f32;

    #[inline]
    fn index(&self, axis: Axis) -> &Self::Output {
        match axis {
            Axis::X => &self.x,
            Axis::Y => &self.y,
        }
    }
}

impl IndexMut<Axis> for Point {
    #[inline]
    fn index_mut(&mut self, axis: Axis) -> &mut Self::Output {
        match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
        }
    }
}

impl Add for Point {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f32> for Point {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl Mul<Point> for f32 {
    type Output = Point;

    #[inline]
    fn mul(self, point: Point) -> Point {
        Point {
            x: self * point.x,
            y: self * point.y,
        }
    }
}

impl Div<f32> for Point {
    type Output = Self;

    #[inline]
    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl Neg for Point {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl From<(f32, f32)> for Point {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl From<Point> for (f32, f32) {
    #[inline]
    fn from(point: Point) -> Self {
        (point.x, point.y)
    }
}

/// A 2D extent (size) with width and height.
///
/// This is similar to Point but semantically represents a size rather than a position.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Extent {
    pub x: f32,
    pub y: f32,
}

impl Extent {
    /// Creates a new extent.
    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { x: width, y: height }
    }

    /// Creates a zero extent.
    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Returns the width.
    #[inline]
    pub fn width(&self) -> f32 {
        self.x
    }

    /// Returns the height.
    #[inline]
    pub fn height(&self) -> f32 {
        self.y
    }

    /// Returns the area.
    #[inline]
    pub fn area(&self) -> f32 {
        self.x * self.y
    }
}

impl Index<Axis> for Extent {
    type Output = f32;

    #[inline]
    fn index(&self, axis: Axis) -> &Self::Output {
        match axis {
            Axis::X => &self.x,
            Axis::Y => &self.y,
        }
    }
}

impl IndexMut<Axis> for Extent {
    #[inline]
    fn index_mut(&mut self, axis: Axis) -> &mut Self::Output {
        match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
        }
    }
}

impl From<Point> for Extent {
    #[inline]
    fn from(point: Point) -> Self {
        Self { x: point.x, y: point.y }
    }
}

impl From<Extent> for Point {
    #[inline]
    fn from(extent: Extent) -> Self {
        Self { x: extent.x, y: extent.y }
    }
}

impl From<(f32, f32)> for Extent {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl Add for Extent {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Extent {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f32> for Extent {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl Div<f32> for Extent {
    type Output = Self;

    #[inline]
    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_operations() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 4.0);

        assert_eq!(p1 + p2, Point::new(4.0, 6.0));
        assert_eq!(p2 - p1, Point::new(2.0, 2.0));
        assert_eq!(p1 * 2.0, Point::new(2.0, 4.0));
    }

    #[test]
    fn test_point_index() {
        let mut p = Point::new(1.0, 2.0);
        assert_eq!(p[Axis::X], 1.0);
        assert_eq!(p[Axis::Y], 2.0);

        p[Axis::X] = 5.0;
        assert_eq!(p.x, 5.0);
    }

    #[test]
    fn test_extent() {
        let e = Extent::new(100.0, 50.0);
        assert_eq!(e.width(), 100.0);
        assert_eq!(e.height(), 50.0);
        assert_eq!(e.area(), 5000.0);
    }
}
