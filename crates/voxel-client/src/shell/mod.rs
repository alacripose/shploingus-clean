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
//! | `notification_queue` | `notifications.rs` | **active** — the api-layer queue the laya gate feeds |
//! | `ticket_window` | `tickets.rs` | unchecked — the file does not exist yet |
//! | `change_view` | `changes.rs` | **active** — the vector change view, its projection, and the outlines it draws |
//!
//! **A witness file existing is not a feature implemented**, and `features.rs` says so itself:
//! a row's `means` text is *"checked against nothing"*. So the rows read `active` when this
//! directory appears **and the thing each row names is actually there** — which is why each
//! module below states what it does and does not yet do.

pub mod api;
pub mod arrangement;
// The cell raster: the package designer shell's 8 × 16 surface, and the Android 4 schema every
// shell paint is retinted to. Declared here so the draw boundary and its tests share one path.
pub mod cells;
pub mod changes;
pub mod console;
pub mod debug;
pub mod desktop;
pub mod drop;
pub mod hud;
// The map window: one owner for its state, its rects, its verbs and its drawing. `mount` steers it
// — the pointer routes to it and the layer asks it for its content — and keeps no copy of any of it.
pub mod map_window;
pub mod mount;
pub mod notifications;
pub mod pane;
pub mod placeholder;
pub mod splitter;
pub mod taskbar;
pub mod theme;
pub mod titlebar;
pub mod webview;
pub mod window;
pub mod xml;

pub use api::ApiView;
pub use mount::{BarEntry, Button, ContentViews, Mount, Status, WheelDelta};
