use crate::uid::Uid;

/// Custom event type that can be sent from threads and received by winit main loop handler, using
/// winit event proxies.
#[derive(Debug)]
pub enum CustomEvent {
    /// Signal sent to widgets.
    /// The first element is the signal UID and the second element is the listener UID.
    Signal((Uid, Uid)),
}
