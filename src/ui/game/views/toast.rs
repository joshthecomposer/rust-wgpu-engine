//! Toast notification view - handles toast display logic and animations.
//! Works with the GameRoot Slint component, which is owned by GameRootView.

use std::cell::Cell;
use std::rc::Rc;

use crate::ui::toast::ToastType;

use super::game_root::GameRoot;

/// Animation state for a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToastState {
    Entering = 0, // slide in from right (0-300ms)
    Visible = 1,  // fully shown (300ms - duration)
    Exiting = 2,  // fade out (duration - duration+200ms)
}

/// Internal data for an active toast notification.
#[derive(Debug, Clone)]
struct ToastData {
    id: u32,
    toast_type: ToastType,
    title: String,
    message: String,
    created_at: f64,
    duration: f64,
    state: ToastState,
}

// Slint expects tuples for anonymous struct models in ALPHABETICAL order
// Fields: {id, message, state, title, toast-type} -> alphabetically sorted
type SlintToast = (i32, slint::SharedString, i32, slint::SharedString, i32);

fn to_slint_toast(toast: &ToastData) -> SlintToast {
    (
        toast.id as i32,
        toast.message.clone().into(), // message comes before state alphabetically
        toast.state as i32,
        toast.title.clone().into(), // title comes before toast-type alphabetically
        toast.toast_type as i32,
    )
}

/// Manages toast notifications for the GameRoot component.
pub struct ToastView {
    toasts: Vec<ToastData>,
    next_id: u32,
    dismiss_pending: Rc<Cell<Option<u32>>>,
    default_duration: f64,
    max_toasts: usize,
    enter_duration: f64,                    // 0.3s
    exit_duration: f64,                     // 0.2s
    model: Rc<slint::VecModel<SlintToast>>, // persistent model to avoid recreating every frame
    needs_sync: bool,                       // track if we need to sync to Slint
}

impl ToastView {
    /// Create a new ToastView and wire up callbacks to the GameRoot component.
    pub fn new(game_root: &GameRoot) -> Self {
        let dismiss_pending = Rc::new(Cell::new(None));

        // wire up close callback
        {
            let dismiss = dismiss_pending.clone();
            game_root.on_toast_close_clicked(move |id| {
                dismiss.set(Some(id as u32));
            });
        }

        let model = Rc::new(slint::VecModel::default());
        game_root.set_toasts(model.clone().into());

        Self {
            toasts: Vec::new(),
            next_id: 0,
            dismiss_pending,
            default_duration: 5.0,
            max_toasts: 5,
            enter_duration: 0.3,
            exit_duration: 0.2,
            model,
            needs_sync: false,
        }
    }

    /// Add a new toast notification.
    pub fn add_toast(
        &mut self,
        toast_type: ToastType,
        title: String,
        message: String,
        duration: Option<f64>,
        created_at: f64,
    ) {
        // enforce max toasts limit - remove oldest if at limit
        if self.toasts.len() >= self.max_toasts {
            self.toasts.remove(0);
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        self.toasts.push(ToastData {
            id,
            toast_type,
            title,
            message,
            created_at,
            duration: duration.unwrap_or(self.default_duration),
            state: ToastState::Entering,
        });
        self.needs_sync = true;
    }

    /// Update toast states and sync to Slint.
    pub fn update(&mut self, _game_root: &GameRoot, elapsed_time: f64) {
        let mut state_changed = false;

        // handle manual dismiss
        if let Some(id) = self.dismiss_pending.replace(None) {
            if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                toast.state = ToastState::Exiting;
                state_changed = true;
            }
        }

        // update toast states based on elapsed time
        for toast in &mut self.toasts {
            let age = elapsed_time - toast.created_at;

            match toast.state {
                ToastState::Entering if age >= self.enter_duration => {
                    toast.state = ToastState::Visible;
                    state_changed = true;
                }
                ToastState::Visible if age >= toast.duration => {
                    toast.state = ToastState::Exiting;
                    state_changed = true;
                }
                _ => {}
            }
        }

        // remove toasts that have finished exiting
        let old_len = self.toasts.len();
        self.toasts.retain(|toast| {
            let age = elapsed_time - toast.created_at;
            if toast.state == ToastState::Exiting {
                age < toast.duration + self.exit_duration
            } else {
                true
            }
        });
        if self.toasts.len() != old_len {
            state_changed = true;
        }

        // only sync to Slint if something changed
        if self.needs_sync || state_changed {
            let slint_toasts: Vec<SlintToast> = self.toasts.iter().map(to_slint_toast).collect();
            self.model.set_vec(slint_toasts);
            self.needs_sync = false;
        }
    }
}
