# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-10-26

### Breaking changes

- `LineBox::on_submit` is now raised when the enter key is pressed or when the focus is lost, and
  only if the text has changed since the edition started.
- The `color`, `event`, `quad`, `rect`, `uid` and `vertex` modules are now private: `Color`,
  `CustomEvent`, `Quad`, `DQuad`, `Rect`, `DRect`, `IRect`, `Uid` and `Vertex` are re-exported at
  the crate root.
- `font::Atlas` moved to `piment::Atlas`. `Atlas::rebuild` has been removed: `Atlas::insert` now
  packs the glyph immediately and returns an error if it does not fit in the atlas.
- `Font::from_system` and `Font::from_bytes` no longer take a list of font sizes.
- `Font::build_texture` renamed to `Font::update_texture`.
- `Font::ascent`, `Font::descent`, `Font::text_width` and `Font::layout` now take `&mut self`, as
  missing glyphs may be loaded on demand.
- `Painter::new`, `Painter::prepare_render` and `Painter::render` are no longer public.
- `widgets::Root` is no longer public.
- `Graphics::font` has been replaced by `Painter::fonts`, which stores fonts by name. The system
  font is registered as `"main"`, which the piment widgets require.
- `Painter` text drawing methods take the font to be used from an identified in `FontStyle`.
- `FontStyle` is no longer `Copy`, and `Painter::text` and `Painter::text_layout` take it by
  reference.
- `Painter::segment_no_cap` replaced by `Painter::line`.

### Added

- `WidgetCore::request_exit` to let a widget ask the application to close.
- `WidgetCore::release_focus` to leave no widget focused.
- `Input::focus_gained` and `Input::focus_lost`. When the focus changes, widgets are updated again
  with an input holding only these focus events.
- Support functions to modify Grid layout after creation.
- Animation support: `WidgetCore::request_animation_frame`, `Input::time` and `Input::time_delta`.
- `Painter::layout_text` builds a `TextLayout` from a `FontStyle`.
- Improved line drawing: now with round caps.
- `LineBox`: pressing escape restores initial value.

### Changed

- Dynamic loading of missing bitmap font characters and sizes.
- `LineBox` selection and focus behavior changes.
- Text selection of `LineBox` is only displayed while focused.
- `Numeric` text edit behavior minor change.

### Fixed

- `Grid::insert` now requests a layout update.

## [0.2.0] - 2026-10-02

### Breaking changes

- `App::new` now takes the window title as argument: `App::new("My app")`.
- `WidgetCore::new` no longer takes a debug name argument, and the `WidgetCore::name` field has
  been removed.

### Added

- `Grid` layout widget, with configurable row and column spacing (`set_row_spacing`,
  `set_column_spacing`).
- `Numeric` widget: a button displaying a numeric value, which can be changed by dragging the
  mouse, clicking on the decrement and increment arrows, or typing a new value. Supports optional
  minimum and maximum bounds, step and drag speed.
- `CollapsiblePanel` widget: a panel with a title header which shows or hides its child when the
  header is clicked.
- Signed distance field fonts (`font_sdf::FontSdf`), which can be drawn at any size with outline
  and shadow, using `Painter::text_sdf` and `Painter::text_layout_sdf`.
- `WidgetCore::focused` to know whether a widget has the keyboard focus.
- The window minimum size is now set from the root widget minimum size, and the window opens at
  that size unless maximized.
- `widget_gallery` and `font_sdf` examples.
- Example screenshot in the README.

### Changed

- Button text is now vertically centered including the button shadow, so that its baseline matches
  the baseline of a label placed beside it.
- Widgets may now change the mouse cursor icon during their update depending on the pointer
  position.

### Fixed

- Sub-menus visibility now follows the visibility of their parent menu.
- Crash when a clipping rectangle extends outside of the window.

## [0.1.0] - 2026-09-28

Initial release.

### Added

- `App` to run a widget tree in a winit window rendered with wgpu, with options to start the
  window maximized, an `ImageCache` shared between widgets and threads, and an event loop proxy to
  send custom events from other threads.
- Widgets: `Button`, `CheckBox`, `Label`, `Layout`, `LineBox`, `RootMenu` and `MenuItem`,
  `MultiPage`, `Panel` and `ScrollBar`.
- Font rendering with shadows.
- Built-in shaders embedded in the binary.
- `hello_button` example.

[Unreleased]: https://github.com/kingofpayne/piment/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kingofpayne/piment/releases/tag/v0.1.0
