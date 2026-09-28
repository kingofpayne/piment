use crate::{
    graphics::Graphics,
    impl_widget_core,
    rect::Rect,
    widgets::{SharedWidget, Widget, WidgetCore},
};
use glam::{Vec2, vec2};

/// A layout placing widgets in cells arranged in rows and columns.
///
/// Each column is as wide as the largest minimum width of its widgets, and each row is as high as
/// the largest minimum height of its widgets.
pub struct Grid {
    /// Widget common properties.
    core: WidgetCore,
    /// Cells indexed by row, then column. All rows have the same number of columns.
    cells: Vec<Vec<Option<Cell>>>,
}

/// Properties of a grid cell.
struct Cell {
    /// Widget placed in the cell.
    widget: SharedWidget,
}

impl Grid {
    /// Creates a new empty grid.
    pub fn new() -> Self {
        Self {
            core: WidgetCore::new(),
            cells: Vec::new(),
        }
    }

    /// Returns the number of rows.
    pub fn rows(&self) -> usize {
        self.cells.len()
    }

    /// Returns the number of columns.
    pub fn columns(&self) -> usize {
        self.cells.first().map_or(0, Vec::len)
    }

    /// Places a widget in the cell at the given `row` and `column`, growing the grid if needed.
    ///
    /// Returns the widget previously placed in that cell, if any.
    pub fn insert(
        &mut self,
        row: usize,
        column: usize,
        widget: SharedWidget,
    ) -> Option<SharedWidget> {
        let columns = self.columns().max(column + 1);
        if row >= self.cells.len() {
            self.cells.resize_with(row + 1, Vec::new);
        }
        for cells in self.cells.iter_mut() {
            cells.resize_with(columns, || None);
        }
        let previous = self.cells[row][column]
            .replace(Cell { widget })
            .map(|cell| cell.widget);
        self.core.children = self
            .cells
            .iter()
            .flatten()
            .flatten()
            .map(|cell| cell.widget.clone())
            .collect();
        previous
    }

    /// Returns the minimum width of each column and the minimum height of each row.
    fn minimum_widths_heights(&self, graphics: &mut Graphics) -> (Vec<f32>, Vec<f32>) {
        let mut widths = vec![0.0f32; self.columns()];
        let mut heights = vec![0.0f32; self.rows()];
        for (row, cells) in self.cells.iter().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                if let Some(cell) = cell {
                    let size = cell.widget.borrow_mut().minimum_size(graphics);
                    widths[column] = widths[column].max(size.x);
                    heights[row] = heights[row].max(size.y);
                }
            }
        }
        (widths, heights)
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Grid {
    impl_widget_core!();

    fn layout(&mut self, graphics: &mut Graphics, rect: Rect) {
        self.core.layout(rect);
        let (widths, heights) = self.minimum_widths_heights(graphics);
        let mut y = rect.y1;
        for (cells, height) in self.cells.iter().zip(heights) {
            let mut x = rect.x1;
            for (cell, width) in cells.iter().zip(&widths) {
                if let Some(cell) = cell {
                    cell.widget
                        .borrow_mut()
                        .layout(graphics, Rect::new(x, y, x + width, y + height));
                }
                x += width;
            }
            y += height;
        }
    }

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        let (widths, heights) = self.minimum_widths_heights(graphics);
        vec2(widths.iter().sum(), heights.iter().sum())
    }
}
