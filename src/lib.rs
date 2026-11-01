// SPDX-License-Identifier: MPL-2.0

//! Widgets libcosmic does not have, built the way libcosmic builds its own.
//!
//! Five things, each usable on its own:
//!
//! - [`reveal()`] — a wrapper that animates its content's width (or height)
//!   between zero and its natural size, the way a sidebar slides in and out.
//!   libcosmic shows and hides its nav bar instantly, at every call site.
//! - [`rail()`] and [`nav_rail()`] — an icon-only nav bar. libcosmic's collapsed
//!   nav bar is gone, not narrow.
//! - [`SidebarState`] — the bookkeeping that puts the two together inside
//!   libcosmic's nav-bar slot: a sidebar that is expanded, a rail, or hidden,
//!   with animated transitions between all three.
//! - [`header_button()`] — an icon button sized to the header bar's row at
//!   every interface density. libcosmic's `button::icon` grows with the
//!   density and is cut off by the header at Spacious.
//! - [`search_field()`] — libcosmic's search field with a clear button that
//!   is there only when the field has text.
//!
//! Animation follows libcosmic's own widgets (`toggler`, `cards`): the clock
//! lives in the widget tree, driven from `RedrawRequested`, so the application
//! sends no per-frame messages and holds no timers. The interpolation is
//! iced's lilt-backed [`Animation`](cosmic::iced::animation::Animation),
//! which libcosmic already ships and which is interruptible — reversing a
//! half-finished slide continues from where it is.
//!
//! # Features
//!
//! - `a11y` (default) — give the rail's icon-only buttons an accessible name,
//!   which is otherwise only in a tooltip, and have a [`Reveal`] hand on its
//!   content's accessibility nodes once it has settled open. Forwards to
//!   `libcosmic/a11y`, and is on by default because libcosmic's own defaults
//!   include it.

pub mod header;
pub mod rail;
pub mod reveal;
pub mod search;
pub mod sidebar;

pub use header::{HeaderButton, header_button};
pub use rail::{NavRail, Rail, nav_rail, rail, rail_item};
pub use reveal::{Edge, Reveal, reveal};
pub use search::search_field;
pub use sidebar::{Mode, SidebarState};
