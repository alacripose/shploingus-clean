//! **The shell: declared windows, their arrangement, and the state the shell owns.**
//!
//! # Why this directory exists, and it is four rows deep
//!
//! **`UI-SPEC.md` §2**: *"Ownership is the load-bearing half: the window declares; the shell
//! owns `WindowState` … A window that draws its own chrome is a window the shell cannot
//! theme."*
//!
//! Five feature rows name a file in **this** directory as their witness. Where they stand,
//! measured on the build that mounted the shell:
//!
//! | feature | witness | state |
//! |---|---|---|
//! | `shell_arrangement` | `arrangement.rs` | **active** — the pane tree of §3 |
//! | `pinned_hud` | `hud.rs` | **active** — the pinned layer, and the layer choice |
//! | `notification_queue` | `notifications.rs` | unchecked — the file does not exist yet |
//! | `ticket_window` | `tickets.rs` | unchecked — the file does not exist yet |
//! | `change_view` | `changes.rs` | unchecked — the file does not exist yet |
//!
//! **A witness file existing is not a feature implemented**, and `features.rs` says so itself:
//! a row's `means` text is *"checked against nothing"*. So the rows read `active` when this
//! directory appears **and the thing each row names is actually there** — which is why each
//! module below states what it does and does not yet do.

pub mod arrangement;
pub mod console;
pub mod debug;
pub mod hud;
pub mod mount;

pub use mount::{BarEntry, Button, ContentViews, Mount, Status};
