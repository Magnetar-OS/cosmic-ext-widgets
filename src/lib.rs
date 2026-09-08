// SPDX-License-Identifier: MPL-2.0

//! Widgets libcosmic does not have, built the way libcosmic builds its own.
//!
//! Three things, each usable on its own:
//!
//! - [`reveal`] — a wrapper that animates its content's width (or height)
//!   between zero and its natural size, the way a sidebar slides in and out.
//!   libcosmic shows and hides its nav bar instantly, at every call site.
//! - [`rail`] and [`nav_rail`] — an icon-only nav bar. libcosmic's collapsed
//!   nav bar is gone, not narrow.
//! - [`SidebarState`] — the bookkeeping that puts the two together inside
//!   libcosmic's nav-bar slot: a sidebar that is expanded, a rail, or hidden,
//!   with animated transitions between all three.
//!
//! Animation follows libcosmic's own widgets (`toggler`, `cards`): the clock
//! lives in the widget tree, driven from `RedrawRequested`, so the application
//! sends no per-frame messages and holds no timers. The interpolation is
//! iced's lilt-backed [`Animation`](cosmic::iced::animation::Animation),
//! which libcosmic already ships and which is interruptible — reversing a
//! half-finished slide continues from where it is.

pub mod rail;
pub mod reveal;
pub mod sidebar;

pub use rail::{nav_rail, rail, rail_item};
pub use reveal::{Axis, Reveal, reveal};
pub use sidebar::{Mode, SidebarState};
