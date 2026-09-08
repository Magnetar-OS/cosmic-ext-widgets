// SPDX-License-Identifier: MPL-2.0

//! A sidebar with three widths — expanded, rail, hidden — and animated
//! transitions between them, inside libcosmic's nav-bar slot.
//!
//! The slot wants an `Option<Element>` and pads whatever it is given, so a
//! sidebar that has slid down to nothing still costs the padding. That is
//! why this is a small state machine rather than just two [`reveal`]s in a
//! row: it keeps the element in the tree exactly as long as something is
//! visible or moving, and drops it once the closing slide has finished.
//!
//! ```ignore
//! // In the application:
//! sidebar: SidebarState,
//!
//! // On the nav-bar toggle, and in `on_window_resize` if libcosmic's
//! // condensed breakpoint should hide it:
//! self.sidebar.set_mode(mode);
//!
//! // One message, so the state learns the closing slide is over:
//! Message::SidebarClosed => self.sidebar.closed(),
//!
//! fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Self::Message>>> {
//!     self.sidebar
//!         .view(self.expanded_sidebar(), self.rail_sidebar(), Message::SidebarClosed)
//!         .map(|element| element.map(cosmic::Action::App))
//! }
//! ```

use cosmic::Element;
use cosmic::widget;

use crate::reveal::reveal;

/// How wide the sidebar is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// The full sidebar.
    #[default]
    Expanded,
    /// Icons only — see [`rail`](crate::rail).
    Rail,
    /// Nothing, once the closing slide has finished.
    Hidden,
}

/// Where the sidebar is, and whether its element should still be in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarState {
    mode: Mode,
    /// The element is in the tree: something is visible or still moving.
    present: bool,
    /// The element has been dropped from the tree at least once, so its next
    /// appearance is a re-appearance and should slide in.
    reappearing: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new(Mode::default())
    }
}

impl SidebarState {
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            present: mode != Mode::Hidden,
            reappearing: false,
        }
    }

    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Move to `mode`. Idempotent, so it is safe to call on every event that
    /// might have changed the answer, such as a window resize.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode != Mode::Hidden {
            self.present = true;
        }
    }

    /// Handle the message given to [`view`](Self::view) as `on_closed`.
    ///
    /// Fires whenever either width settles closed, including when the
    /// expanded sidebar has finished giving way to the rail, so it only acts
    /// when the whole sidebar is meant to be gone.
    pub fn closed(&mut self) {
        if self.mode == Mode::Hidden {
            self.present = false;
            self.reappearing = true;
        }
    }

    /// The element for libcosmic's nav-bar slot, or `None` once hidden.
    ///
    /// Both `expanded` and `rail` are built every view; only the one the
    /// mode wants has any width. They sit side by side, so switching between
    /// them shrinks one as the other grows.
    #[must_use]
    pub fn view<'a, Message: Clone + 'static>(
        &self,
        expanded: impl Into<Element<'a, Message>>,
        rail: impl Into<Element<'a, Message>>,
        on_closed: Message,
    ) -> Option<Element<'a, Message>> {
        if !self.present {
            return None;
        }

        let row = widget::row::with_capacity(2)
            .push(
                reveal(expanded)
                    .open(self.mode == Mode::Expanded)
                    .start_closed(self.reappearing)
                    .on_closed(on_closed.clone()),
            )
            .push(
                reveal(rail)
                    .open(self.mode == Mode::Rail)
                    .start_closed(self.reappearing)
                    .on_closed(on_closed),
            );

        Some(row.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_present_unless_hidden() {
        assert!(SidebarState::default().present);
        assert!(SidebarState::new(Mode::Rail).present);
        assert!(!SidebarState::new(Mode::Hidden).present);
    }

    #[test]
    fn stays_in_the_tree_until_the_closing_slide_reports_in() {
        let mut state = SidebarState::default();
        state.set_mode(Mode::Hidden);
        assert!(state.present, "still sliding out");
        state.closed();
        assert!(!state.present);
        assert!(state.reappearing);
    }

    #[test]
    fn expanded_giving_way_to_rail_is_not_a_close() {
        let mut state = SidebarState::default();
        state.set_mode(Mode::Rail);
        // The expanded reveal settles closed and reports it.
        state.closed();
        assert!(state.present);
        assert!(!state.reappearing);
    }

    #[test]
    fn reopening_before_the_slide_finishes_keeps_the_element() {
        let mut state = SidebarState::default();
        state.set_mode(Mode::Hidden);
        state.set_mode(Mode::Expanded);
        assert!(state.present);
        // A late report from a slide that reversed changes nothing.
        state.closed();
        assert!(state.present);
    }

    #[test]
    fn showing_again_re_enters_the_tree() {
        let mut state = SidebarState::default();
        state.set_mode(Mode::Hidden);
        state.closed();
        state.set_mode(Mode::Rail);
        assert!(state.present);
        assert_eq!(state.mode(), Mode::Rail);
    }
}
