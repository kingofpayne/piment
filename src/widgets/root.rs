use crate::{graphics::Graphics, input::Input, rect::Rect, uid::Uid, widgets::SharedWidget};
use std::{
    collections::VecDeque,
    mem::take,
    time::{Duration, Instant},
};
use winit::window::CursorIcon;

pub struct Root {
    pub widget: SharedWidget,
    focused: Uid,
    /// Widget which had the focus during the previous update, to raise the focus change flags of
    /// [Input].
    update_focused: Uid,
    /// Current cursor icon. This member is used to remember the latest cursor asked to winit, to
    /// remove unnecessary calls to cursor change for each frame.
    cursor: CursorIcon,
    /// Set when a widget has requested the application to exit. Never cleared.
    pub exit_request: bool,
    /// Latest received input without its events, given to the updates notifying focus changes.
    idle_input: Input,
    /// Start time of the application, to calculate [Input::time].
    start: Instant,
    /// [Input::time] of the latest update, to calculate [Input::time_delta].
    last_update: Duration,
    /// Set when a widget has requested an animation frame. Cleared by the application once the
    /// next animation update is scheduled.
    pub animation_request: bool,
}

impl Root {
    pub fn new(widget: SharedWidget) -> Self {
        let focused = Uid::new();
        Self {
            widget,
            focused,
            update_focused: focused,
            cursor: CursorIcon::Default,
            exit_request: false,
            idle_input: Input::new(),
            start: Instant::now(),
            last_update: Duration::ZERO,
            animation_request: false,
        }
    }

    pub fn layout(&self, graphics: &mut Graphics, rect: Rect) {
        self.widget.borrow_mut().layout(graphics, rect);
    }

    /// Forwards the event to all widgets in the interface.
    ///
    /// Child widgets are updated before their parents. This method also performs mouse interaction
    /// hit test: only the top-most widget under the mouse pointer will receive the `hit` flag set
    /// to true. For instance, when clicking on a menu, the menu will have the `hit` flag while the
    /// widget below won't.
    ///
    /// If the focus has changed, widgets are then updated again without any event, to notify the
    /// focus change.
    pub fn update(&mut self, graphics: &mut Graphics, input: &Input) {
        let mut input = input.clone();
        input.time = self.start.elapsed();
        input.time_delta = input.time - self.last_update;
        self.last_update = input.time;
        self.idle_input = input.without_events();
        self.update_widgets(graphics, &input);
        self.notify_focus_changes(graphics);
    }

    /// Updates all widgets with `input`, then handles their requests.
    fn update_widgets(&mut self, graphics: &mut Graphics, input: &Input) {
        let mut hit_widget: Option<SharedWidget> = None;
        let widgets = self.list_widgets();
        let focus_changed = self.focused != self.update_focused;
        // Update from children up to parents
        for item in widgets.iter().rev() {
            // We set hit flag for top-most widget (the widget which is directly under the cursor).
            let mut filtered_input = input.clone();
            if hit_widget.is_none() {
                let w = item.widget.borrow();
                if item.visible && w.core().rect.contains(input.mouse_pos) && w.core().catch_hit {
                    filtered_input.hit = true;
                    hit_widget = Some(item.widget.clone());
                }
            }
            let mut widget = item.widget.borrow_mut();
            let uid = widget.core().uid;
            filtered_input.focus = self.focused == uid;
            filtered_input.focus_gained = focus_changed && self.focused == uid;
            filtered_input.focus_lost = focus_changed && self.update_focused == uid;
            widget.update(graphics, &filtered_input);
        }
        self.update_focused = self.focused;
        // The cursor is read after the update, as widgets may change it depending on the pointer
        // position.
        if let Some(widget) = hit_widget {
            let new_cursor = widget.borrow().core().cursor;
            if new_cursor != self.cursor {
                self.cursor = new_cursor;
                graphics.window.set_cursor(new_cursor);
            }
        }
        // When widgets update, their may raise flags and signals in their WidgetCore member. We
        // must parse the widgets and satisfy their requests.
        self.handle_requests(graphics, &widgets);
    }

    /// Updates all widgets without any event while the focus changes, so that widgets receive
    /// [Input::focus_gained] and [Input::focus_lost] before being rendered.
    ///
    /// If widgets keep moving the focus on each focus change, this is not detected and loops
    /// forever.
    fn notify_focus_changes(&mut self, graphics: &mut Graphics) {
        while self.focused != self.update_focused {
            let input = self.idle_input.clone();
            self.update_widgets(graphics, &input);
        }
    }

    /// Walks all widgets to take into account their requests:
    /// - layout recalculation requests,
    /// - repaint requests,
    /// - animation frame requests,
    /// - signals transfer to other widgets,
    /// - focus requests,
    /// - exit requests.
    ///
    /// When handling signals, listening widgets are updated and may perform even more requests.
    /// This method handles this and loops until no more widgets emit signals.
    ///
    /// If there is a cyclic loop in signals emission, this is not detected and may loop forever.
    ///
    /// The flat tree of widgets is passed as `widgets` parameter to avoid recalculating it.
    fn handle_requests(&mut self, graphics: &mut Graphics, widgets: &[FlatTreeItem]) {
        let mut todo: VecDeque<SharedWidget> =
            widgets.iter().map(|item| item.widget.clone()).collect();
        let mut layout_request = false;
        let mut _repaint_request = false;
        let previous_focused = self.focused;
        let mut focus_requested = false;
        let mut focus_release = false;
        while let Some(widget) = todo.pop_front() {
            let mut widget = widget.borrow_mut();
            let core = widget.core_mut();
            layout_request |= core.layout_request;
            _repaint_request |= core.repaint_request;
            self.animation_request |= core.animation_request;
            self.exit_request |= core.exit_request;
            core.layout_request = false;
            core.repaint_request = false;
            core.animation_request = false;
            core.exit_request = false;
            // Handle focus requests
            if let Some(target) = core.focus_request.take() {
                self.focused = target;
                focus_requested = true;
            }
            focus_release |= take(&mut core.focus_release);
            // Handle signals
            // Clear list from core using take.
            let signals = take(&mut core.signals);
            // Release borrow because updated widget may need to borrow their children.
            drop(widget);
            for (signal, target) in signals {
                // Look-up for the target item.
                if let Some(item) = widgets.iter().find(|w| w.widget.borrow().uid() == target) {
                    // Update the widget. During the update, the widget may perform more requests,
                    // so we copy that widget in the todo list.
                    item.widget.borrow_mut().signal(graphics, signal);
                    todo.push_back(item.widget.clone());
                }
            }
        }
        // A release must not cancel a request made by another widget, for instance the one which
        // has just been clicked, whatever their order in the tree.
        if focus_release && !focus_requested {
            // A new random UID matches no widget.
            self.focused = Uid::new();
        }
        // Update the focus flags once no widget is borrowed anymore, as the widget requesting the
        // focus may be the previous or the new focus holder.
        if self.focused != previous_focused {
            for item in widgets {
                let mut widget = item.widget.borrow_mut();
                let core = widget.core_mut();
                core.focused = core.uid == self.focused;
            }
        }
        // Handle layout requests
        if layout_request {
            let mut widget = self.widget.borrow_mut();
            let rect = widget.core().rect;
            widget.layout(graphics, rect);
        }
    }

    /// Handles a signal received from winit custom event proxy.
    pub fn signal(&mut self, graphics: &mut Graphics, signal: Uid, listener: Uid) {
        let widgets: Vec<_> = self.list_widgets();
        let mut signals = VecDeque::new();
        signals.push_front((signal, listener));
        while let Some((signal, listener)) = signals.pop_front() {
            if let Some(widget) = widgets.iter().find(|w| w.widget.borrow().uid() == listener) {
                let mut widget = widget.widget.borrow_mut();
                widget.signal(graphics, signal);
            }
        }
        self.handle_requests(graphics, &widgets);
        self.notify_focus_changes(graphics);
    }

    /// Renders the visible widgets of the interface.
    ///
    /// Widgets with the [crate::ux::widgets::WidgetCore::above] flag set are rendered first. This
    /// flag is used for windows, menus, popups, etc.Then parent widgets are rendered before their
    /// children. This particular ordering is computed in [Self::list_widgets].
    pub fn render(&mut self, graphics: &mut Graphics) {
        self.list_widgets()
            .iter()
            .filter(|item| item.visible && item.widget.borrow().core().rect != Rect::ZERO)
            .for_each(|item| item.widget.borrow_mut().render(graphics));
    }

    /// Build the list of widgets to be updated/rendered, in order.
    fn list_widgets(&self) -> Vec<FlatTreeItem> {
        let mut result = Vec::new();
        let mut todo_list = VecDeque::new();

        todo_list.push_back(FlatTreeItem {
            widget: self.widget.clone(),
            visible: self.widget.borrow().core().visible,
        });

        while let Some(item) = todo_list.pop_front() {
            result.push(item.clone());
            for child in item.widget.borrow().core().children.iter() {
                let item = FlatTreeItem {
                    widget: child.clone(),
                    visible: item.visible && child.borrow().core().visible,
                };
                if child.borrow().core().above {
                    todo_list.push_back(item);
                } else {
                    todo_list.push_front(item);
                }
            }
        }
        result
    }
}

#[derive(Clone)]
struct FlatTreeItem {
    widget: SharedWidget,
    /// Whether the widget is visible or not. This is different for widget own visible flag,
    /// because it takes into account the visibility of the parents.
    visible: bool,
}
