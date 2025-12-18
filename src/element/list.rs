//! List and selection elements.

use std::any::Any;
use std::sync::RwLock;
use super::{Element, ViewLimits, ViewStretch};
use super::context::{BasicContext, Context};
use crate::support::point::Point;
use crate::support::rect::Rect;
use crate::support::color::Color;
use crate::support::theme::get_theme;
use crate::view::{MouseButton, MouseButtonKind, CursorTracking};

/// List selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    #[default]
    Single,
    Multiple,
    None,
}

/// Callback type for selection changes.
pub type SelectionCallback = Box<dyn Fn(usize) + Send + Sync>;
pub type MultiSelectionCallback = Box<dyn Fn(&[usize]) + Send + Sync>;

/// A list item.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub label: String,
    pub data: Option<String>,
}

impl ListItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            data: None,
        }
    }

    pub fn with_data(label: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            data: Some(data.into()),
        }
    }
}

/// A list element for displaying and selecting items.
pub struct List {
    items: RwLock<Vec<ListItem>>,
    selected: RwLock<Vec<usize>>,
    selection_mode: SelectionMode,
    hovered_index: RwLock<Option<usize>>,
    scroll_offset: RwLock<f32>,
    background_color: Color,
    item_color: Color,
    selected_color: Color,
    hover_color: Color,
    text_color: Color,
    selected_text_color: Color,
    item_height: f32,
    width: f32,
    height: f32,
    padding: f32,
    corner_radius: f32,
    enabled: bool,
    on_select: Option<SelectionCallback>,
    on_multi_select: Option<MultiSelectionCallback>,
}

impl List {
    /// Creates a new list.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            items: RwLock::new(Vec::new()),
            selected: RwLock::new(Vec::new()),
            selection_mode: SelectionMode::Single,
            hovered_index: RwLock::new(None),
            scroll_offset: RwLock::new(0.0),
            background_color: theme.input_box_color,
            item_color: Color::new(0.0, 0.0, 0.0, 0.0),
            selected_color: theme.selection_hilite_color,
            hover_color: theme.frame_hilite_color.with_alpha(0.3),
            text_color: theme.label_font_color,
            selected_text_color: theme.label_font_color,
            item_height: 28.0,
            width: 200.0,
            height: 200.0,
            padding: 4.0,
            corner_radius: 4.0,
            enabled: true,
            on_select: None,
            on_multi_select: None,
        }
    }

    /// Sets the items.
    pub fn items(self, items: Vec<ListItem>) -> Self {
        *self.items.write().unwrap() = items;
        self
    }

    /// Sets items from strings.
    pub fn items_from_strings(self, items: Vec<&str>) -> Self {
        *self.items.write().unwrap() = items.into_iter().map(ListItem::new).collect();
        self
    }

    /// Sets the selection mode.
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Sets the dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the item height.
    pub fn item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }

    /// Sets the background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Sets the selected color.
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = color;
        self
    }

    /// Sets the selection callback (single selection mode).
    pub fn on_select<F: Fn(usize) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Sets the multi-selection callback.
    pub fn on_multi_select<F: Fn(&[usize]) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_multi_select = Some(Box::new(callback));
        self
    }

    /// Returns the selected indices.
    pub fn get_selected(&self) -> Vec<usize> {
        self.selected.read().unwrap().clone()
    }

    /// Sets the selected index (single selection).
    pub fn set_selected(&self, index: usize) {
        let mut selected = self.selected.write().unwrap();
        selected.clear();
        let items = self.items.read().unwrap();
        if index < items.len() {
            selected.push(index);
        }
    }

    /// Clears selection.
    pub fn clear_selection(&self) {
        self.selected.write().unwrap().clear();
    }

    /// Adds an item.
    pub fn add_item(&self, item: ListItem) {
        self.items.write().unwrap().push(item);
    }

    /// Removes an item.
    pub fn remove_item(&self, index: usize) {
        let mut items = self.items.write().unwrap();
        if index < items.len() {
            items.remove(index);
            // Update selection
            let mut selected = self.selected.write().unwrap();
            selected.retain(|&i| i != index);
            for i in selected.iter_mut() {
                if *i > index {
                    *i -= 1;
                }
            }
        }
    }

    fn total_content_height(&self) -> f32 {
        let items = self.items.read().unwrap();
        items.len() as f32 * self.item_height + self.padding * 2.0
    }

    fn item_bounds(&self, ctx: &Context, index: usize) -> Rect {
        let scroll = *self.scroll_offset.read().unwrap();
        let y = ctx.bounds.top + self.padding + index as f32 * self.item_height - scroll;

        Rect::new(
            ctx.bounds.left + self.padding,
            y,
            ctx.bounds.right - self.padding,
            y + self.item_height,
        )
    }

    fn draw_background(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(self.background_color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);
    }

    fn draw_items(&self, ctx: &Context) {
        let items = self.items.read().unwrap();
        let selected = self.selected.read().unwrap();
        let hovered = *self.hovered_index.read().unwrap();
        let theme = get_theme();

        for (i, item) in items.iter().enumerate() {
            let bounds = self.item_bounds(ctx, i);

            // Skip if outside visible area
            if bounds.bottom < ctx.bounds.top || bounds.top > ctx.bounds.bottom {
                continue;
            }

            let is_selected = selected.contains(&i);
            let is_hovered = hovered == Some(i) && self.enabled;

            let mut canvas = ctx.canvas.borrow_mut();

            // Background
            if is_selected {
                canvas.fill_style(self.selected_color);
                canvas.fill_round_rect(bounds, 3.0);
            } else if is_hovered {
                canvas.fill_style(self.hover_color);
                canvas.fill_round_rect(bounds, 3.0);
            }

            // Text
            let text_color = if !self.enabled {
                self.text_color.with_alpha(0.5)
            } else if is_selected {
                self.selected_text_color
            } else {
                self.text_color
            };

            canvas.fill_style(text_color);
            canvas.font_size(theme.label_font_size);

            let x = bounds.left + 8.0;
            let y = bounds.center().y + theme.label_font_size * 0.35;
            canvas.fill_text(&item.label, Point::new(x, y));
        }
    }

    fn draw_scrollbar(&self, ctx: &Context) {
        let total_height = self.total_content_height();
        let visible_height = ctx.bounds.height();

        if total_height <= visible_height {
            return;
        }

        let theme = get_theme();
        let scroll = *self.scroll_offset.read().unwrap();

        let scrollbar_height = (visible_height / total_height * visible_height).max(20.0);
        let scrollbar_y = scroll / (total_height - visible_height) * (visible_height - scrollbar_height);

        let scrollbar_rect = Rect::new(
            ctx.bounds.right - 8.0,
            ctx.bounds.top + scrollbar_y,
            ctx.bounds.right - 2.0,
            ctx.bounds.top + scrollbar_y + scrollbar_height,
        );

        let mut canvas = ctx.canvas.borrow_mut();
        canvas.fill_style(theme.scrollbar_color);
        canvas.fill_round_rect(scrollbar_rect, 3.0);
    }
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for List {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 1.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_background(ctx);
        self.draw_items(ctx);
        self.draw_scrollbar(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, _leaf: bool, _control: bool) -> Option<&dyn Element> {
        if ctx.bounds.contains(p) && self.enabled {
            Some(self)
        } else {
            None
        }
    }

    fn wants_control(&self) -> bool {
        self.enabled
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != MouseButtonKind::Left {
            return false;
        }

        if !btn.down {
            return true;
        }

        if self.selection_mode == SelectionMode::None {
            return true;
        }

        // Find clicked item
        let items = self.items.read().unwrap();
        for i in 0..items.len() {
            let bounds = self.item_bounds(ctx, i);
            if bounds.contains(btn.pos) && bounds.top >= ctx.bounds.top && bounds.bottom <= ctx.bounds.bottom {
                drop(items);

                let mut selected = self.selected.write().unwrap();

                match self.selection_mode {
                    SelectionMode::Single => {
                        selected.clear();
                        selected.push(i);
                        drop(selected);
                        if let Some(ref callback) = self.on_select {
                            callback(i);
                        }
                    }
                    SelectionMode::Multiple => {
                        if let Some(pos) = selected.iter().position(|&x| x == i) {
                            selected.remove(pos);
                        } else {
                            selected.push(i);
                        }
                        let selection = selected.clone();
                        drop(selected);
                        if let Some(ref callback) = self.on_multi_select {
                            callback(&selection);
                        }
                    }
                    SelectionMode::None => {}
                }

                return true;
            }
        }

        true
    }

    fn scroll(&mut self, ctx: &Context, dir: Point, _p: Point) -> bool {
        if !self.enabled {
            return false;
        }

        let total_height = self.total_content_height();
        let visible_height = ctx.bounds.height();

        if total_height <= visible_height {
            return false;
        }

        let mut scroll = self.scroll_offset.write().unwrap();
        *scroll = (*scroll - dir.y * 20.0).clamp(0.0, total_height - visible_height);

        true
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        match status {
            CursorTracking::Leaving => {
                *self.hovered_index.write().unwrap() = None;
            }
            _ => {
                let items = self.items.read().unwrap();
                let mut hovered = self.hovered_index.write().unwrap();
                *hovered = None;

                for i in 0..items.len() {
                    let bounds = self.item_bounds(ctx, i);
                    if bounds.contains(p) && bounds.top >= ctx.bounds.top && bounds.bottom <= ctx.bounds.bottom {
                        *hovered = Some(i);
                        break;
                    }
                }
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A dropdown/combo box element.
pub struct Dropdown {
    items: Vec<String>,
    selected: RwLock<Option<usize>>,
    expanded: RwLock<bool>,
    hovered_index: RwLock<Option<usize>>,
    background_color: Color,
    hover_color: Color,
    text_color: Color,
    arrow_color: Color,
    width: f32,
    height: f32,
    dropdown_height: f32,
    corner_radius: f32,
    enabled: bool,
    placeholder: String,
    on_select: Option<SelectionCallback>,
}

impl Dropdown {
    /// Creates a new dropdown.
    pub fn new() -> Self {
        let theme = get_theme();
        Self {
            items: Vec::new(),
            selected: RwLock::new(None),
            expanded: RwLock::new(false),
            hovered_index: RwLock::new(None),
            background_color: theme.default_button_color,
            hover_color: theme.frame_hilite_color,
            text_color: theme.label_font_color,
            arrow_color: theme.label_font_color,
            width: 150.0,
            height: 28.0,
            dropdown_height: 150.0,
            corner_radius: 4.0,
            enabled: true,
            placeholder: String::from("Select..."),
            on_select: None,
        }
    }

    /// Sets the items.
    pub fn items(mut self, items: Vec<&str>) -> Self {
        self.items = items.into_iter().map(String::from).collect();
        self
    }

    /// Sets the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the selection callback.
    pub fn on_select<F: Fn(usize) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Returns the selected index.
    pub fn get_selected(&self) -> Option<usize> {
        *self.selected.read().unwrap()
    }

    /// Returns the selected item text.
    pub fn get_selected_text(&self) -> Option<String> {
        let selected = *self.selected.read().unwrap();
        selected.and_then(|i| self.items.get(i).cloned())
    }

    /// Sets the selected index.
    pub fn set_selected(&self, index: Option<usize>) {
        *self.selected.write().unwrap() = index;
    }

    fn item_height(&self) -> f32 {
        self.height
    }

    fn dropdown_bounds(&self, ctx: &Context) -> Rect {
        let item_count = self.items.len().min(5);
        let height = (item_count as f32 * self.item_height()).min(self.dropdown_height);

        Rect::new(
            ctx.bounds.left,
            ctx.bounds.bottom + 2.0,
            ctx.bounds.right,
            ctx.bounds.bottom + 2.0 + height,
        )
    }

    fn draw_button(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let theme = get_theme();
        let expanded = *self.expanded.read().unwrap();

        let color = if expanded {
            self.background_color.level(1.2)
        } else {
            self.background_color
        };

        canvas.fill_style(color);
        canvas.fill_round_rect(ctx.bounds, self.corner_radius);

        // Text
        let selected = *self.selected.read().unwrap();
        let text = selected
            .and_then(|i| self.items.get(i))
            .unwrap_or(&self.placeholder);

        let text_color = if selected.is_none() {
            self.text_color.with_alpha(0.6)
        } else {
            self.text_color
        };

        canvas.fill_style(text_color);
        canvas.font_size(theme.label_font_size);

        let x = ctx.bounds.left + 10.0;
        let y = ctx.bounds.center().y + theme.label_font_size * 0.35;
        canvas.fill_text(text, Point::new(x, y));

        // Arrow
        canvas.fill_style(self.arrow_color);
        let arrow = if expanded { "▲" } else { "▼" };
        let arrow_x = ctx.bounds.right - 20.0;
        canvas.fill_text(arrow, Point::new(arrow_x, y));
    }

    fn draw_dropdown(&self, ctx: &Context) {
        if !*self.expanded.read().unwrap() {
            return;
        }

        let dropdown_rect = self.dropdown_bounds(ctx);
        let theme = get_theme();
        let selected = *self.selected.read().unwrap();
        let hovered = *self.hovered_index.read().unwrap();

        let mut canvas = ctx.canvas.borrow_mut();

        // Shadow
        let shadow_rect = dropdown_rect.translate(2.0, 2.0);
        canvas.fill_style(Color::new(0.0, 0.0, 0.0, 0.3));
        canvas.fill_round_rect(shadow_rect, self.corner_radius);

        // Background
        canvas.fill_style(self.background_color.level(1.1));
        canvas.fill_round_rect(dropdown_rect, self.corner_radius);

        // Items
        for (i, item) in self.items.iter().enumerate() {
            let item_rect = Rect::new(
                dropdown_rect.left,
                dropdown_rect.top + i as f32 * self.item_height(),
                dropdown_rect.right,
                dropdown_rect.top + (i + 1) as f32 * self.item_height(),
            );

            if item_rect.bottom > dropdown_rect.bottom {
                break;
            }

            let is_selected = selected == Some(i);
            let is_hovered = hovered == Some(i);

            if is_selected {
                canvas.fill_style(self.hover_color);
                canvas.fill_rect(item_rect);
            } else if is_hovered {
                canvas.fill_style(self.hover_color.with_alpha(0.5));
                canvas.fill_rect(item_rect);
            }

            canvas.fill_style(self.text_color);
            canvas.font_size(theme.label_font_size);

            let x = item_rect.left + 10.0;
            let y = item_rect.center().y + theme.label_font_size * 0.35;
            canvas.fill_text(item, Point::new(x, y));
        }
    }
}

impl Default for Dropdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Element for Dropdown {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        ViewLimits::fixed(self.width, self.height)
    }

    fn stretch(&self) -> ViewStretch {
        ViewStretch::new(1.0, 0.0)
    }

    fn draw(&self, ctx: &Context) {
        self.draw_button(ctx);
        self.draw_dropdown(ctx);
    }

    fn hit_test(&self, ctx: &Context, p: Point, _leaf: bool, _control: bool) -> Option<&dyn Element> {
        if !self.enabled {
            return None;
        }

        if ctx.bounds.contains(p) {
            return Some(self);
        }

        if *self.expanded.read().unwrap() {
            let dropdown_rect = self.dropdown_bounds(ctx);
            if dropdown_rect.contains(p) {
                return Some(self);
            }
        }

        None
    }

    fn wants_control(&self) -> bool {
        self.enabled
    }

    fn handle_click(&self, ctx: &Context, btn: MouseButton) -> bool {
        if !self.enabled || btn.button != MouseButtonKind::Left {
            return false;
        }

        if !btn.down {
            return true;
        }

        let expanded = *self.expanded.read().unwrap();

        if ctx.bounds.contains(btn.pos) {
            // Toggle dropdown
            *self.expanded.write().unwrap() = !expanded;
            return true;
        }

        if expanded {
            let dropdown_rect = self.dropdown_bounds(ctx);
            if dropdown_rect.contains(btn.pos) {
                // Find clicked item
                let rel_y = btn.pos.y - dropdown_rect.top;
                let index = (rel_y / self.item_height()) as usize;

                if index < self.items.len() {
                    *self.selected.write().unwrap() = Some(index);
                    *self.expanded.write().unwrap() = false;
                    if let Some(ref callback) = self.on_select {
                        callback(index);
                    }
                }
                return true;
            }

            // Click outside closes dropdown
            *self.expanded.write().unwrap() = false;
        }

        true
    }

    fn cursor(&mut self, ctx: &Context, p: Point, status: CursorTracking) -> bool {
        if !self.enabled {
            return false;
        }

        let expanded = *self.expanded.read().unwrap();

        match status {
            CursorTracking::Leaving => {
                *self.hovered_index.write().unwrap() = None;
            }
            _ if expanded => {
                let dropdown_rect = self.dropdown_bounds(ctx);
                if dropdown_rect.contains(p) {
                    let rel_y = p.y - dropdown_rect.top;
                    let index = (rel_y / self.item_height()) as usize;
                    *self.hovered_index.write().unwrap() = if index < self.items.len() {
                        Some(index)
                    } else {
                        None
                    };
                } else {
                    *self.hovered_index.write().unwrap() = None;
                }
            }
            _ => {
                *self.hovered_index.write().unwrap() = None;
            }
        }

        true
    }

    fn enable(&mut self, state: bool) {
        self.enabled = state;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a list.
pub fn list() -> List {
    List::new()
}

/// Creates a dropdown.
pub fn dropdown() -> Dropdown {
    Dropdown::new()
}
