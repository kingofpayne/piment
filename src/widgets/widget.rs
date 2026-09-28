use crate::{graphics::Graphics, input::Input, rect::Rect, uid::Uid};
use glam::Vec2;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use winit::window::CursorIcon;

/// Use this macro to implement [Widget::core] and [Widget::core_mut] methods to return the
/// [WidgetCore] member of the widget implementor.
#[macro_export]
macro_rules! impl_widget_core {
    () => {
        fn core(&self) -> &WidgetCore {
            &self.core
        }

        fn core_mut(&mut self) -> &mut WidgetCore {
            &mut self.core
        }
    };
}

/// The trait every widget must implement.
pub trait Widget {
    /// Sets the position of the widget.
    ///
    /// When the layout is changed, the widget shall recalculate their children position and update
    /// them by recursively calling this method.
    ///
    /// The implementation shall call [WidgetCore::layout] on the widget core member.The default
    /// implementation will call [WidgetCore::layout] and [Widget::layout] on children declared in
    /// [WidgetCore] member with the same rectangle area.
    fn layout(&mut self, graphics: &mut Graphics, rect: Rect) {
        let core = self.core_mut();
        core.layout(rect);
        for child in core.children.iter_mut() {
            child.borrow_mut().layout(graphics, rect);
        }
    }

    /// Recalculates the layout of this widget and its children, reusing the current widget
    /// position.
    ///
    /// This may be called manually for instance when the visibility of a child has changed.
    fn relayout(&mut self, graphics: &mut Graphics) {
        self.layout(graphics, self.core().rect);
    }

    /// Returns widget minimum allowed size.
    ///
    /// Default implementation has no minimum size.
    fn minimum_size(&mut self, _graphics: &mut Graphics) -> Vec2 {
        Vec2::ZERO
    }

    /// Update the widget from an event.
    ///
    /// Note: [Self::update] may be called multiple times before the widget being rendered. If any
    /// event may trigger graphics calculation, such as updating a texture, it is advised to delay
    /// such operation until [Self::render] is called; this may avoids heavy operations on
    /// intermediate widget states.
    fn update(&mut self, _graphics: &mut Graphics, _input: &Input) {}

    /// Paint the widget.
    ///
    /// The UX makes sure children widgets are rendered after their parent.
    ///
    /// This method takes a mutable reference to allow caching mechanisms to be implemented if
    /// needed.
    fn render(&mut self, _graphics: &mut Graphics) {}

    /// Returns reference to widget common properties.
    /// Use [impl_widget_core] macro to automatically implement this method.
    fn core(&self) -> &WidgetCore;

    /// Returns mutable reference to widget common properties.
    /// Use [impl_widget_core] macro to automatically implement this method.
    fn core_mut(&mut self) -> &mut WidgetCore;

    /// Returns widget unique identifier.
    fn uid(&self) -> Uid {
        self.core().uid
    }

    /// Registers in the list of targets of the given `signal`.
    ///
    /// Once registered, if the signal is emitted by the widget owning it, the method
    /// [Self::signal] will be called for every listeners.
    fn listen_to(&self, signal: &mut Signal) {
        signal.listeners.push(self.uid());
    }

    /// Called when a signal the widget is registered to has been emitted by a widget.
    /// When handling this signal, more signals can be emitted through the returned
    /// [Response].
    fn signal(&mut self, _graphics: &mut Graphics, _signal: Uid) {}

    /// Hides the widget.
    ///
    /// This [WidgetCore::visible] property to false.
    fn hide(&mut self) {
        self.core_mut().visible = false;
    }

    /// Shows the widget.
    ///
    /// This [WidgetCore::visible] property to true.
    fn show(&mut self) {
        self.core_mut().visible = true;
    }
}

/// Common properties shared by all widgets.
pub struct WidgetCore {
    /// Unique identifier for widget identification.
    /// Used for instance for focus attribution.
    pub uid: Uid,
    /// Position of the widget.
    pub rect: Rect,
    /// Children.
    pub children: Vec<SharedWidget>,
    /// When false, the widget and its children are not rendered.
    pub visible: bool,
    /// When true, widget is above others.
    pub above: bool,
    /// Whether the mouse pointer is hover the widget or not.
    pub hover: bool,
    /// Whenever the widget catches mouse interaction (true) or let widgets underneath catch hit
    /// (false).
    pub catch_hit: bool,
    /// Cursor to be displayed when mouse is over the widget.
    pub cursor: CursorIcon,
    /// When this flag is set to true, the UI will recalculate the layout of the widgets.
    /// This is cleared by the UI after update.
    pub layout_request: bool,
    /// When this flag is set to true, the UI will repaint as soon as possible.
    pub repaint_request: bool,
    /// Emitted signals
    /// First tuple item is the signal, second tuple is the target widget.
    pub signals: VecDeque<(Uid, Uid)>,
    /// When set, widget request focus to be transferred to the given widget UID.
    /// Target can be self.
    pub focus_request: Option<Uid>,
}

impl WidgetCore {
    /// Returns a new `WidgetCore` with no children.
    pub fn new() -> Self {
        Self {
            uid: Uid::new(),
            rect: Default::default(),
            children: Default::default(),
            visible: true,
            above: Default::default(),
            hover: Default::default(),
            catch_hit: true,
            cursor: CursorIcon::Default,
            layout_request: false,
            repaint_request: false,
            signals: VecDeque::new(),
            focus_request: None,
        }
    }

    /// Defines the children.
    pub fn with_children(self, children: Vec<SharedWidget>) -> Self {
        Self { children, ..self }
    }

    /// Builds a `WidgetCore` with `above` flag set.
    pub fn with_above(self) -> Self {
        Self {
            above: true,
            ..self
        }
    }

    /// Builds a `WidgetCore` with given visibility.
    pub fn with_visible(self, visible: bool) -> Self {
        Self { visible, ..self }
    }

    /// Builds a `WidgetCore` without `catch_hit` flag set.
    pub fn with_no_catch_hit(self) -> Self {
        Self {
            catch_hit: false,
            ..self
        }
    }

    /// Builds a `WidgetCore` with given cursor icon.
    pub fn with_cursor(self, cursor: CursorIcon) -> Self {
        Self { cursor, ..self }
    }

    /// Sets the position of the widget.
    pub fn layout(&mut self, rect: Rect) {
        self.rect = rect
    }

    /// Updates core state from an input event.
    pub fn update(&mut self, input: &Input) {
        if input.mouse_moved() {
            self.hover = self.rect.contains(input.mouse_pos)
        }
    }

    /// Requests the UI for a layout update.
    pub fn request_layout(&mut self) {
        self.layout_request = true;
    }

    /// Requests the UI a repaint.
    pub fn request_repaint(&mut self) {
        self.repaint_request = true;
    }

    /// Emit a signal that will be transmitted to the registered signal listeners.
    pub fn emit(&mut self, signal: &Signal) {
        self.signals
            .extend(signal.listeners.iter().map(|l| (signal.uid, *l)));
    }

    /// Request the focus to itself.
    pub fn request_focus(&mut self) {
        self.focus_request = Some(self.uid);
    }

    /// Request the focus to be given to the specified widget.
    pub fn give_focus(&mut self, target: Uid) {
        self.focus_request = Some(target);
    }
}

pub type SharedWidget = Rc<RefCell<dyn Widget>>;

pub type Shared<T> = Rc<RefCell<T>>;

pub trait Share: Sized {
    fn shared(self) -> Shared<Self> {
        Rc::new(RefCell::new(self))
    }
}

impl<T> Share for T {}

#[derive(Default)]
pub struct Signal {
    pub uid: Uid,
    pub listeners: Vec<Uid>,
}

impl Signal {
    pub fn register(&mut self, widget: &dyn Widget) {
        self.listeners.push(widget.uid())
    }
}
