//! Global toast notification system.
//!
//! Provides a DI-like interface for showing toast notifications from anywhere in the codebase
//! without requiring direct coupling to UI systems. Toasts are queued globally and drained
//! by the UI system each frame.

use std::cell::RefCell;

/// Type of toast notification, determines styling and icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // variants are used via macro expansion
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

/// A pending toast notification waiting to be displayed.
#[derive(Debug, Clone)]
pub struct PendingToast {
    pub toast_type: ToastType,
    pub title: String,
    pub message: String,
    pub duration: Option<f64>, // None = use default (5s)
}

thread_local! {
    /// Global queue of pending toast notifications.
    /// This is thread-local to avoid synchronization overhead in a single-threaded game loop.
    static TOAST_QUEUE: RefCell<Vec<PendingToast>> = RefCell::new(Vec::new());
}

/// Show a toast notification.
///
/// This function can be called from anywhere in the codebase without requiring
/// access to UI systems or message queues.
///
/// # Arguments
/// * `toast_type` - Type of toast (Success, Error, Warning, Info)
/// * `title` - Toast title text
/// * `message` - Toast message text
/// * `duration` - Optional custom duration in seconds (None = use default 5s)
///
/// # Example
/// ```
/// use crate::ui::toast::{show_toast, ToastType};
///
/// show_toast(ToastType::Success, "Quest Complete", "You have slain the Ember Drake!", None);
/// show_toast(ToastType::Error, "Connection Lost", "Retrying...", Some(10.0));
/// ```
pub fn show_toast(
    toast_type: ToastType,
    title: impl Into<String>,
    message: impl Into<String>,
    duration: Option<f64>,
) {
    let title_str = title.into();
    let message_str = message.into();
    println!("[show_toast] Adding toast: {:?} - {} | {}", toast_type, title_str, message_str);
    
    TOAST_QUEUE.with(|queue| {
        queue.borrow_mut().push(PendingToast {
            toast_type,
            title: title_str,
            message: message_str,
            duration,
        });
        println!("[show_toast] Queue now has {} toasts", queue.borrow().len());
    });
}

/// Drain all pending toast notifications.
///
/// This should be called once per frame by the UI system to retrieve and display
/// pending toasts. After calling this, the global queue is empty.
///
/// # Returns
/// Vector of pending toasts in the order they were queued.
pub fn drain_pending_toasts() -> Vec<PendingToast> {
    TOAST_QUEUE.with(|queue| queue.borrow_mut().drain(..).collect())
}

/// Convenience macro for showing toast notifications.
///
/// # Usage
/// ```
/// // Basic usage with default duration (5s)
/// toast!(Success, "Quest Complete", "You have slain the Ember Drake!");
///
/// // With custom duration
/// toast!(Error, "Connection Lost", "Retrying in 10 seconds...", 10.0);
/// ```
#[macro_export]
macro_rules! toast {
    ($type:ident, $title:expr, $msg:expr) => {
        $crate::ui::toast::show_toast($crate::ui::toast::ToastType::$type, $title, $msg, None)
    };
    ($type:ident, $title:expr, $msg:expr, $duration:expr) => {
        $crate::ui::toast::show_toast(
            $crate::ui::toast::ToastType::$type,
            $title,
            $msg,
            Some($duration),
        )
    };
}
