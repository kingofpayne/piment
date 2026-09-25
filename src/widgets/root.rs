use crate::{graphics::Graphics, input::Input, rect::Rect, uid::Uid, widgets::SharedWidget};
use std::{collections::VecDeque, mem::take};
use winit::window::CursorIcon;

pub struct Root {
    pub widget: SharedWidget,
    focused: Uid,
    /// Current cursor icon. This member is used to remember the latest cursor asked to winit, to
    /// remove unecessary calls to cursor change for each frame.
    cursor: CursorIcon,
}

impl Root {
    pub fn new(widget: SharedWidget) -> Self {
        Self {
            widget,
            focused: Uid::new(),
            cursor: CursorIcon::Default,
        }
    }

    pub fn layout(&self, graphics: &mut Graphics, rect: Rect) {
        self.widget.borrow_mut().layout(graphics, rect);
    }

    /// Forwards the event to all widgets in the interface.
    ///
    /// Child widgets are updated before their parents. This method also performs mouse interaction
    /// hit test: only the top-most widget will receive mouse events with the `hit` flag set to
    /// true. For instance, when clicking on a menu, the menu will have the `hit` flag while the
    /// widget below won't.
    pub fn update(&mut self, graphics: &mut Graphics, input: &Input) {
        let mut hit_registered = false;
        let widgets = self.list_widgets();
        // Update from children up to parents
        for item in widgets.iter().rev() {
            // For mouse events, we set hit flag for top-most widget (the widget which is directly
            // under the cursor).
            let mut filtered_input = input.clone();
            if !hit_registered {
                let w = item.widget.borrow();
                if item.visible && w.core().rect.contains(input.mouse_pos) && w.core().catch_hit {
                    filtered_input.hit = true;
                    hit_registered = true;
                    let new_cursor = w.core().cursor;
                    if new_cursor != self.cursor {
                        self.cursor = new_cursor;
                        graphics.window.set_cursor(new_cursor);
                    }
                }
            }
            let mut widget = item.widget.borrow_mut();
            let uid = widget.core().uid;
            if self.focused == uid {
                filtered_input.focus = true;
            }
            widget.update(graphics, &filtered_input);
        }
        // When widgets update, their may raise flags and signals in their WidgetCore member. We
        // must parse the widgets and satisfy their requests.
        self.handle_requests(graphics, &widgets);
    }

    /// Walks all widgets to take into account their requests:
    /// - layout recalculation requests,
    /// - repaint requests,
    /// - signals transfer to other widgets,
    /// - focus requests.
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
        let mut repaint_request = false;
        while let Some(widget) = todo.pop_front() {
            let mut widget = widget.borrow_mut();
            let core = widget.core_mut();
            layout_request |= core.layout_request;
            repaint_request |= core.repaint_request;
            core.layout_request = false;
            core.repaint_request = false;
            // Handle focus requests
            if let Some(target) = core.focus_request.take() {
                self.focused = target;
            }
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
