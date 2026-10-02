use crate::{
    graphics::Graphics,
    impl_widget_core,
    rect::Rect,
    widgets::{SharedWidget, Widget, WidgetCore},
};
use glam::{Vec2, vec2};

/// A layout placing widgets in cells arranged in rows and columns.
pub struct Grid {
    /// Widget common properties.
    core: WidgetCore,
    /// Cells indexed by row, then column. All rows have the same number of columns.
    cells: Vec<Vec<Option<Cell>>>,
    /// Horizontal spacing between two consecutive columns.
    column_spacing: f32,
    /// Vertical spacing between two consecutive rows.
    row_spacing: f32,
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
            column_spacing: 4.0,
            row_spacing: 4.0,
        }
    }

    /// Returns the horizontal spacing between two consecutive columns.
    pub fn column_spacing(&self) -> f32 {
        self.column_spacing
    }

    /// Sets the horizontal spacing between two consecutive columns.
    pub fn set_column_spacing(&mut self, spacing: f32) {
        if spacing != self.column_spacing {
            self.column_spacing = spacing;
            self.core.request_layout();
        }
    }

    /// Returns the vertical spacing between two consecutive rows.
    pub fn row_spacing(&self) -> f32 {
        self.row_spacing
    }

    /// Sets the vertical spacing between two consecutive rows.
    pub fn set_row_spacing(&mut self, spacing: f32) {
        if spacing != self.row_spacing {
            self.row_spacing = spacing;
            self.core.request_layout();
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
    /// If the cell already holds a widget, it is replaced.
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
        self.refresh_children();
        previous
    }

    /// Removes the widget placed in the cell at the given `row` and `column`, leaving the cell
    /// empty. The grid size is unchanged.
    ///
    /// Returns the removed widget, or `None` if the cell is empty or outside of the grid.
    pub fn remove(&mut self, row: usize, column: usize) -> Option<SharedWidget> {
        let previous = self
            .cells
            .get_mut(row)?
            .get_mut(column)?
            .take()
            .map(|cell| cell.widget);
        self.refresh_children();
        previous
    }

    /// Removes the given `row` and its widgets. Following rows are shifted up.
    ///
    /// # Panics
    ///
    /// Panics if `row` is out of bounds.
    pub fn remove_row(&mut self, row: usize) {
        self.cells.remove(row);
        self.refresh_children();
    }

    /// Removes the given `column` and its widgets. Following columns are shifted left.
    ///
    /// # Panics
    ///
    /// Panics if `column` is out of bounds.
    pub fn remove_column(&mut self, column: usize) {
        assert!(column < self.columns(), "column index out of bounds");
        for cells in self.cells.iter_mut() {
            cells.remove(column);
        }
        if self.columns() == 0 {
            self.cells.clear();
        }
        self.refresh_children();
    }

    /// Removes all the widgets, leaving a grid with no rows and no columns.
    pub fn clear(&mut self) {
        self.resize(0, 0);
    }

    /// Sets the number of rows and columns. Widgets placed outside of the new bounds are removed,
    /// and new cells are empty. If `rows` or `columns` is zero, the grid becomes empty.
    pub fn resize(&mut self, rows: usize, columns: usize) {
        if rows == 0 || columns == 0 {
            self.cells.clear();
        } else {
            self.cells.resize_with(rows, Vec::new);
            for cells in self.cells.iter_mut() {
                cells.resize_with(columns, || None);
            }
        }
        self.refresh_children();
    }

    /// Rebuilds the children list from the cells and requests a layout update.
    fn refresh_children(&mut self) {
        self.core.children = self
            .cells
            .iter()
            .flatten()
            .flatten()
            .map(|cell| cell.widget.clone())
            .collect();
        self.core.request_layout();
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
                x += width + self.column_spacing;
            }
            y += height + self.row_spacing;
        }
    }

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        let (widths, heights) = self.minimum_widths_heights(graphics);
        let column_spacings = self.columns().saturating_sub(1) as f32 * self.column_spacing;
        let row_spacings = self.rows().saturating_sub(1) as f32 * self.row_spacing;
        vec2(
            widths.iter().sum::<f32>() + column_spacings,
            heights.iter().sum::<f32>() + row_spacings,
        )
    }
}
