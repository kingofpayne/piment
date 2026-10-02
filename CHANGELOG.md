# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking changes

- `LineBox::on_submit` is now raised when the enter key is pressed or when the focus is lost, and
  only if the text has changed since the edition started.

### Added

- `WidgetCore::request_exit` to let a widget ask the application to close.
- `WidgetCore::release_focus` to leave no widget focused.
- `Input::focus_gained` and `Input::focus_lost`. When the focus changes, widgets are updated again
  with an input holding only these focus events.
- Pressing the escape key in a `LineBox` restores the text it had when the edition started. The
  enter and escape keys, as well as clicking outside of the widget, release the focus.
- A `LineBox` selects its whole text when it gains the focus. When the focus is gained by a
  click, the text is selected on release, unless a range has been selected by dragging.

### Changed

- The text selection of a `LineBox` is only displayed while it has the focus.
- `Numeric` follows its text box behavior: the edition ends when the text box loses the focus, and
  the typed value is committed only if the text has changed.

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
