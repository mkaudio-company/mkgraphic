//! Rectangle type for 2D regions.

use super::point::{Point, Extent, Axis};

/// A rectangle defined by its left, top, right, and bottom edges.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    /// Creates a new rectangle from edge coordinates.
    #[inline]
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    /// Creates a rectangle from origin point and size.
    #[inline]
    pub fn from_origin_size(origin: Point, size: Extent) -> Self {
        Self {
            left: origin.x,
            top: origin.y,
            right: origin.x + size.x,
            bottom: origin.y + size.y,
        }
    }

    /// Creates a rectangle from two corner points.
    #[inline]
    pub fn from_points(p1: Point, p2: Point) -> Self {
        Self {
            left: p1.x.min(p2.x),
            top: p1.y.min(p2.y),
            right: p1.x.max(p2.x),
            bottom: p1.y.max(p2.y),
        }
    }

    /// Creates an empty rectangle at the origin.
    #[inline]
    pub const fn zero() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    /// Returns the width of the rectangle.
    #[inline]
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    /// Sets the width, keeping the left edge fixed.
    #[inline]
    pub fn set_width(&mut self, width: f32) {
        self.right = self.left + width;
    }

    /// Returns the height of the rectangle.
    #[inline]
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    /// Sets the height, keeping the top edge fixed.
    #[inline]
    pub fn set_height(&mut self, height: f32) {
        self.bottom = self.top + height;
    }

    /// Returns the size as an Extent.
    #[inline]
    pub fn size(&self) -> Extent {
        Extent::new(self.width(), self.height())
    }

    /// Sets the size, keeping the top-left corner fixed.
    #[inline]
    pub fn set_size(&mut self, size: Extent) {
        self.right = self.left + size.x;
        self.bottom = self.top + size.y;
    }

    /// Returns the top-left corner.
    #[inline]
    pub fn top_left(&self) -> Point {
        Point::new(self.left, self.top)
    }

    /// Returns the top-right corner.
    #[inline]
    pub fn top_right(&self) -> Point {
        Point::new(self.right, self.top)
    }

    /// Returns the bottom-left corner.
    #[inline]
    pub fn bottom_left(&self) -> Point {
        Point::new(self.left, self.bottom)
    }

    /// Returns the bottom-right corner.
    #[inline]
    pub fn bottom_right(&self) -> Point {
        Point::new(self.right, self.bottom)
    }

    /// Returns the center point.
    #[inline]
    pub fn center(&self) -> Point {
        Point::new(
            self.left + self.width() / 2.0,
            self.top + self.height() / 2.0,
        )
    }

    /// Returns true if the rectangle is empty (zero width or height).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.left == self.right || self.top == self.bottom
    }

    /// Returns true if the rectangle is valid (right >= left and bottom >= top).
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.left <= self.right && self.top <= self.bottom
    }

    /// Returns true if the point is inside the rectangle.
    #[inline]
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.left && p.x <= self.right && p.y >= self.top && p.y <= self.bottom
    }

    /// Returns true if this rectangle fully contains the other rectangle.
    #[inline]
    pub fn contains_rect(&self, other: &Rect) -> bool {
        other.left >= self.left
            && other.right <= self.right
            && other.top >= self.top
            && other.bottom <= self.bottom
    }

    /// Moves the rectangle by the given delta.
    #[inline]
    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right + dx,
            bottom: self.bottom + dy,
        }
    }

    /// Moves the rectangle to the given position.
    #[inline]
    pub fn move_to(self, x: f32, y: f32) -> Self {
        self.translate(x - self.left, y - self.top)
    }

    /// Insets the rectangle by the given amounts (shrinks it).
    #[inline]
    pub fn inset(self, x_inset: f32, y_inset: f32) -> Self {
        let mut r = Self {
            left: self.left + x_inset,
            top: self.top + y_inset,
            right: self.right - x_inset,
            bottom: self.bottom - y_inset,
        };

        if !r.is_valid() {
            r = Self::zero();
        }
        r
    }

    /// Expands the rectangle by the given amounts.
    #[inline]
    pub fn expand(self, x_expand: f32, y_expand: f32) -> Self {
        self.inset(-x_expand, -y_expand)
    }

    /// Returns the area of the rectangle.
    #[inline]
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Returns the extent along the given axis.
    #[inline]
    pub fn extent(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.width(),
            Axis::Y => self.height(),
        }
    }

    /// Returns the minimum value along the given axis.
    #[inline]
    pub fn min(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.left,
            Axis::Y => self.top,
        }
    }

    /// Returns the maximum value along the given axis.
    #[inline]
    pub fn max(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.right,
            Axis::Y => self.bottom,
        }
    }

    /// Returns a mutable reference to the minimum value along the given axis.
    #[inline]
    pub fn min_mut(&mut self, axis: Axis) -> &mut f32 {
        match axis {
            Axis::X => &mut self.left,
            Axis::Y => &mut self.top,
        }
    }

    /// Returns a mutable reference to the maximum value along the given axis.
    #[inline]
    pub fn max_mut(&mut self, axis: Axis) -> &mut f32 {
        match axis {
            Axis::X => &mut self.right,
            Axis::Y => &mut self.bottom,
        }
    }

    /// Returns the intersection with another rectangle.
    #[inline]
    pub fn intersection(&self, other: Rect) -> Option<Rect> {
        let r = Rect {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };

        if r.is_valid() && !r.is_empty() {
            Some(r)
        } else {
            None
        }
    }
}

/// Returns true if two rectangles intersect.
pub fn intersects(a: &Rect, b: &Rect) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/// Returns the intersection of two rectangles.
pub fn intersection(a: &Rect, b: &Rect) -> Option<Rect> {
    let r = Rect {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    };

    if r.is_valid() && !r.is_empty() {
        Some(r)
    } else {
        None
    }
}

/// Returns the union (bounding box) of two rectangles.
pub fn union(a: &Rect, b: &Rect) -> Rect {
    Rect {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

/// Centers a rectangle within an enclosing rectangle.
pub fn center(r: Rect, encl: &Rect) -> Rect {
    let dx = (encl.width() - r.width()) / 2.0;
    let dy = (encl.height() - r.height()) / 2.0;
    Rect {
        left: encl.left + dx,
        top: encl.top + dy,
        right: encl.left + dx + r.width(),
        bottom: encl.top + dy + r.height(),
    }
}

/// Horizontally centers a rectangle within an enclosing rectangle.
pub fn center_h(r: Rect, encl: &Rect) -> Rect {
    let dx = (encl.width() - r.width()) / 2.0;
    Rect {
        left: encl.left + dx,
        top: r.top,
        right: encl.left + dx + r.width(),
        bottom: r.bottom,
    }
}

/// Vertically centers a rectangle within an enclosing rectangle.
pub fn center_v(r: Rect, encl: &Rect) -> Rect {
    let dy = (encl.height() - r.height()) / 2.0;
    Rect {
        left: r.left,
        top: encl.top + dy,
        right: r.right,
        bottom: encl.top + dy + r.height(),
    }
}

/// Aligns a rectangle within an enclosing rectangle.
///
/// `x_align` and `y_align` should be between 0.0 and 1.0.
pub fn align(r: Rect, encl: &Rect, x_align: f32, y_align: f32) -> Rect {
    let dx = (encl.width() - r.width()) * x_align;
    let dy = (encl.height() - r.height()) * y_align;
    Rect {
        left: encl.left + dx,
        top: encl.top + dy,
        right: encl.left + dx + r.width(),
        bottom: encl.top + dy + r.height(),
    }
}

/// Clips a rectangle to fit within an enclosing rectangle.
pub fn clip(r: Rect, encl: &Rect) -> Rect {
    Rect {
        left: r.left.max(encl.left).min(encl.right),
        top: r.top.max(encl.top).min(encl.bottom),
        right: r.right.max(encl.left).min(encl.right),
        bottom: r.bottom.max(encl.top).min(encl.bottom),
    }
}

/// Creates a rectangle given an axis and coordinates.
pub fn make_rect(
    axis: Axis,
    this_axis_min: f32,
    other_axis_min: f32,
    this_axis_max: f32,
    other_axis_max: f32,
) -> Rect {
    match axis {
        Axis::X => Rect::new(this_axis_min, other_axis_min, this_axis_max, other_axis_max),
        Axis::Y => Rect::new(other_axis_min, this_axis_min, other_axis_max, this_axis_max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_basic() {
        let r = Rect::new(10.0, 20.0, 110.0, 70.0);
        assert_eq!(r.width(), 100.0);
        assert_eq!(r.height(), 50.0);
        assert_eq!(r.area(), 5000.0);
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Point::new(50.0, 50.0)));
        assert!(r.contains(Point::new(0.0, 0.0)));
        assert!(r.contains(Point::new(100.0, 100.0)));
        assert!(!r.contains(Point::new(-1.0, 50.0)));
        assert!(!r.contains(Point::new(101.0, 50.0)));
    }

    #[test]
    fn test_intersection() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 150.0, 150.0);
        let c = intersection(&a, &b).unwrap();
        assert_eq!(c, Rect::new(50.0, 50.0, 100.0, 100.0));
    }

    #[test]
    fn test_no_intersection() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(200.0, 200.0, 300.0, 300.0);
        assert!(intersection(&a, &b).is_none());
    }
}
