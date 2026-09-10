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

use std::time::Duration;

use cosmic::Element;
use cosmic::iced::animation::Easing;
use cosmic::widget;

use crate::reveal::{Edge, reveal};

/// How wide the sidebar is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// The full sidebar.
    #[default]
    Expanded,
    /// Icons only — see [`rail()`](crate::rail()).
    Rail,
    /// Nothing, once the closing slide has finished.
    Hidden,
}

impl Mode {
    /// The next mode in the cycle a nav-bar toggle walks: expanded, then
    /// rail, then hidden, then back to expanded.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Expanded => Self::Rail,
            Self::Rail => Self::Hidden,
            Self::Hidden => Self::Expanded,
        }
    }
}

/// Where the sidebar is, and whether its element should still be in the tree.
///
/// Not [`Eq`]: the easing function it carries is not.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarState {
    mode: Mode,
    edge: Edge,
    duration: Duration,
    easing: Easing,
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
    /// A sidebar starting in `mode`, sliding out past the left edge in
    /// [`reveal`]'s default 200ms ease-out cubic.
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            edge: Edge::Left,
            duration: Duration::from_millis(200),
            easing: Easing::EaseOutCubic,
            present: mode != Mode::Hidden,
            reappearing: false,
        }
    }

    /// The edge the sidebar slides out past. Default [`Edge::Left`].
    ///
    /// This also decides which way round the two widths sit, so that the
    /// sidebar stays against its own edge as one gives way to the other.
    #[must_use]
    pub const fn edge(mut self, edge: Edge) -> Self {
        self.edge = edge;
        self
    }

    /// Length of the slide. See [`Reveal::duration`](crate::Reveal::duration).
    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// How the slide is interpolated. See
    /// [`Reveal::easing`](crate::Reveal::easing).
    #[must_use]
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// The mode the sidebar is in, or on its way to.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Move to `mode`. Idempotent, so it is safe to call on every event that
    /// might have changed the answer, such as a window resize.
    pub const fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if !matches!(mode, Mode::Hidden) {
            self.present = true;
        }
    }

    /// Move to [`Mode::next`] — what a nav-bar toggle wants.
    pub const fn cycle(&mut self) {
        self.set_mode(self.mode.next());
    }

    /// Handle the message given to [`view`](Self::view) as `on_closed`.
    ///
    /// Fires whenever either width settles closed, including when the
    /// expanded sidebar has finished giving way to the rail, so it only acts
    /// when the whole sidebar is meant to be gone.
    pub const fn closed(&mut self) {
        if matches!(self.mode, Mode::Hidden) {
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

        let slide = |content: Element<'a, Message>, open: bool, on_closed: Message| {
            Element::from(
                reveal(content)
                    .open(open)
                    .edge(self.edge)
                    .duration(self.duration)
                    .easing(self.easing)
                    .start_closed(self.reappearing)
                    .on_closed(on_closed),
            )
        };

        let expanded = slide(
            expanded.into(),
            self.mode == Mode::Expanded,
            on_closed.clone(),
        );
        let rail = slide(rail.into(), self.mode == Mode::Rail, on_closed);

        // The wider view leads on the side the sidebar is anchored to, so
        // that whichever width is showing stays against that edge.
        Some(match self.edge {
            Edge::Left => widget::row::with_children(vec![expanded, rail]).into(),
            Edge::Right => widget::row::with_children(vec![rail, expanded]).into(),
            Edge::Top => widget::column::with_children(vec![expanded, rail]).into(),
            Edge::Bottom => widget::column::with_children(vec![rail, expanded]).into(),
        })
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

    #[test]
    fn cycling_walks_all_three_and_comes_back() {
        let mut state = SidebarState::default();
        assert_eq!(state.mode(), Mode::Expanded);
        state.cycle();
        assert_eq!(state.mode(), Mode::Rail);
        state.cycle();
        assert_eq!(state.mode(), Mode::Hidden);
        state.cycle();
        assert_eq!(state.mode(), Mode::Expanded);
    }

    #[test]
    fn cycling_to_hidden_keeps_the_element_until_the_slide_reports_in() {
        let mut state = SidebarState::new(Mode::Rail);
        state.cycle();
        assert_eq!(state.mode(), Mode::Hidden);
        assert!(state.present);
        state.closed();
        assert!(!state.present);
    }

    #[test]
    fn the_animation_settings_are_carried_not_dropped() {
        let state = SidebarState::default()
            .edge(Edge::Right)
            .duration(Duration::from_millis(80))
            .easing(Easing::Linear);
        assert_eq!(state.edge, Edge::Right);
        assert_eq!(state.duration, Duration::from_millis(80));
        assert_eq!(state.easing, Easing::Linear);
    }
}
