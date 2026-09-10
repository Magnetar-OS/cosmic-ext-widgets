// SPDX-License-Identifier: MPL-2.0

//! A wrapper that animates its content between hidden and shown by sliding it
//! along one axis.
//!
//! # Accessibility
//!
//! Content inside a [`Reveal`] does not reach the accessibility tree, even
//! when fully open. `Widget::a11y_nodes` returns an empty tree unless a widget
//! overrides it, and the override cannot be written from outside libcosmic:
//! it returns `iced_accessibility::A11yTree`, and libcosmic keeps
//! `iced_accessibility` as a private dependency. Naming `pop-os/iced` as a
//! dependency here would build a second copy of that crate, whose `A11yTree`
//! is a different type and so does not satisfy the trait.
//!
//! One line upstream — `pub use iced_accessibility;` — closes it, after which
//! the override belongs here, forwarding to the child under the same
//! settled-open condition that `draw` and `update` already use.

use std::time::{Duration, Instant};

use cosmic::Element;
use cosmic::iced::advanced::widget::{Operation, Tree, tree};
use cosmic::iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
};
use cosmic::iced::animation::{Animation, Easing};
use cosmic::iced::{Event, Length, Point, Rectangle, Size, Vector, window};

/// The edge a [`Reveal`]'s content slides out past.
///
/// Name the edge the content leaves by, which for a panel in a row or column
/// is the outer one: a nav bar on the left of the window is [`Edge::Left`],
/// an inspector on the right is [`Edge::Right`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Edge {
    /// Width goes to zero; the content slides out to the left.
    #[default]
    Left,
    /// Width goes to zero; the content slides out to the right.
    Right,
    /// Height goes to zero; the content slides up.
    Top,
    /// Height goes to zero; the content slides down.
    Bottom,
}

impl Edge {
    /// The size the wrapper occupies at `progress` (0 = hidden, 1 = shown),
    /// and where the content sits inside it. The content keeps its natural
    /// size throughout and is clipped, so text does not reflow mid-slide; it
    /// is offset so the edge that disappears is the one named.
    fn reveal(self, full: Size, progress: f32) -> (Size, Point) {
        match self {
            Self::Left => {
                let width = (full.width * progress).round();
                (
                    Size::new(width, full.height),
                    Point::new(width - full.width, 0.0),
                )
            }
            Self::Right => {
                let width = (full.width * progress).round();
                (Size::new(width, full.height), Point::ORIGIN)
            }
            Self::Top => {
                let height = (full.height * progress).round();
                (
                    Size::new(full.width, height),
                    Point::new(0.0, height - full.height),
                )
            }
            Self::Bottom => {
                let height = (full.height * progress).round();
                (Size::new(full.width, height), Point::ORIGIN)
            }
        }
    }

    /// Whether the animated dimension is the width.
    const fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// Wraps `content` so that toggling [`Reveal::open`] slides it in and out.
///
/// The content must have an intrinsic size on the revealed axis — a fixed
/// width, or shrink-to-fit — because that size is what the wrapper animates
/// towards. A `Length::Fill` sidebar would claim the whole row.
pub fn reveal<'a, Message>(content: impl Into<Element<'a, Message>>) -> Reveal<'a, Message> {
    Reveal {
        content: content.into(),
        open: true,
        edge: Edge::Left,
        duration: Duration::from_millis(200),
        easing: Easing::EaseOutCubic,
        start_closed: false,
        on_closed: None,
    }
}

/// See [`reveal`].
#[must_use]
pub struct Reveal<'a, Message> {
    content: Element<'a, Message>,
    open: bool,
    edge: Edge,
    duration: Duration,
    easing: Easing,
    start_closed: bool,
    on_closed: Option<Message>,
}

impl<Message> Reveal<'_, Message> {
    /// Whether the content is shown. Changing this between views starts the
    /// slide; the widget animates from wherever it currently is.
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The edge the content slides out past. Default [`Edge::Left`].
    pub const fn edge(mut self, edge: Edge) -> Self {
        self.edge = edge;
        self
    }

    /// Length of the slide. Default 200ms — iced's `quick`.
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Default ease-out cubic: fast to leave, slow to arrive.
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Treat a freshly created widget as closed even when `open` is true, so
    /// it slides in on its first frames instead of appearing at full size.
    ///
    /// Off by default, so a sidebar that is open at launch is simply there.
    /// Turn it on for a widget re-entering the tree after being removed —
    /// that is the case where the eye expects the reverse of the slide it saw
    /// on the way out. Only the widget's first appearance is affected.
    pub const fn start_closed(mut self, start_closed: bool) -> Self {
        self.start_closed = start_closed;
        self
    }

    /// Published once the content has settled fully hidden — at the end of a
    /// closing slide, or on the first frame of a widget created closed.
    ///
    /// The point of it: the application can then drop the widget from the
    /// tree entirely. A zero-width widget still occupies whatever padding its
    /// container gives it, and libcosmic's nav-bar slot gives it some.
    pub fn on_closed(mut self, message: Message) -> Self {
        self.on_closed = Some(message);
        self
    }
}

/// What one frame asks of the shell.
///
/// Separating the decision from the shell that carries it out is what makes
/// the animation testable: [`State`] can be stepped with instants of the
/// test's choosing, with no renderer, no window and no clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Frame {
    /// Another frame is wanted: the slide is still running.
    redraw: bool,
    /// The wrapper's size has changed, or has just stopped changing.
    invalidate_layout: bool,
    /// The content has just settled fully hidden, exactly once per close.
    closed: bool,
}

struct State {
    anim: Animation<bool>,
    /// True while a transition is running, and on creation, so that the first
    /// settled frame after either is noticed exactly once.
    unsettled: bool,
}

impl State {
    fn new(open: bool, easing: Easing, duration: Duration) -> Self {
        Self {
            anim: Animation::new(open).easing(easing).duration(duration),
            unsettled: true,
        }
    }

    fn progress(&self, now: Instant) -> f32 {
        self.anim.interpolate(0.0_f32, 1.0, now)
    }

    fn settled_open(&self, now: Instant) -> bool {
        self.anim.value() && !self.anim.is_animating(now)
    }

    /// The application changed `open` since the last view. Returns whether it
    /// had in fact changed, and so whether a slide has just started.
    fn retarget(&mut self, open: bool, now: Instant) -> bool {
        if self.anim.value() == open {
            return false;
        }
        self.anim.go_mut(open, now);
        self.unsettled = true;
        true
    }

    /// A redraw arrived at `now`.
    fn redraw(&mut self, open: bool, now: Instant) -> Frame {
        if self.anim.is_animating(now) {
            return Frame {
                redraw: true,
                invalidate_layout: true,
                closed: false,
            };
        }
        if !self.unsettled {
            return Frame::default();
        }
        self.unsettled = false;
        Frame {
            redraw: false,
            invalidate_layout: true,
            closed: !open,
        }
    }
}

impl<Message: Clone> Widget<Message, cosmic::Theme, cosmic::Renderer> for Reveal<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(
            self.open && !self.start_closed,
            self.easing,
            self.duration,
        ))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        let content = self.content.as_widget().size();
        if self.edge.is_horizontal() {
            Size::new(Length::Shrink, content.height)
        } else {
            Size::new(content.width, Length::Shrink)
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &cosmic::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let progress = tree.state.downcast_ref::<State>().progress(Instant::now());
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let (size, origin) = self.edge.reveal(content.size(), progress);
        layout::Node::with_children(size, vec![content.move_to(origin)])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &cosmic::Renderer,
        operation: &mut dyn Operation,
    ) {
        // Focus and scroll operations have no business reaching content that
        // is on its way out.
        if !self.open {
            return;
        }
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout
                    .children()
                    .next()
                    .unwrap()
                    .with_virtual_offset(layout.virtual_offset()),
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &cosmic::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();

        // The application changed its mind since the last view.
        if state.retarget(self.open, now) {
            shell.request_redraw();
            shell.invalidate_layout();
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            let frame = state.redraw(self.open, now);
            if frame.redraw {
                shell.request_redraw();
            }
            if frame.invalidate_layout {
                shell.invalidate_layout();
            }
            if frame.closed
                && let Some(message) = &self.on_closed
            {
                shell.publish(message.clone());
            }
        }

        // Nothing reaches the content while it is moving or hidden: its layout
        // extends past the clip, and a click there belongs to the neighbour.
        if !state.settled_open(now) {
            return;
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout
                .children()
                .next()
                .unwrap()
                .with_virtual_offset(layout.virtual_offset()),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &cosmic::Renderer,
    ) -> mouse::Interaction {
        if !tree
            .state
            .downcast_ref::<State>()
            .settled_open(Instant::now())
        {
            return mouse::Interaction::None;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout
                .children()
                .next()
                .unwrap()
                .with_virtual_offset(layout.virtual_offset()),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        if state.progress(now) <= 0.0 {
            return;
        }

        let bounds = layout.bounds();
        let content_layout = layout
            .children()
            .next()
            .unwrap()
            .with_virtual_offset(layout.virtual_offset());
        let draw = |renderer: &mut cosmic::Renderer, viewport: &Rectangle| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                content_layout,
                cursor,
                viewport,
            );
        };

        if state.settled_open(now) {
            draw(renderer, viewport);
        } else {
            let Some(visible) = bounds.intersection(viewport) else {
                return;
            };
            renderer.with_layer(bounds, |renderer| draw(renderer, &visible));
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &cosmic::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, cosmic::Theme, cosmic::Renderer>> {
        if !tree
            .state
            .downcast_ref::<State>()
            .settled_open(Instant::now())
        {
            return None;
        }
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout
                .children()
                .next()
                .unwrap()
                .with_virtual_offset(layout.virtual_offset()),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Reveal<'a, Message>> for Element<'a, Message> {
    fn from(reveal: Reveal<'a, Message>) -> Self {
        Element::new(reveal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUICK: Duration = Duration::from_millis(200);

    fn state(open: bool) -> State {
        State::new(open, Easing::EaseOutCubic, QUICK)
    }

    #[test]
    fn left_slides_out_to_the_left() {
        let full = Size::new(300.0, 500.0);
        assert_eq!(Edge::Left.reveal(full, 1.0), (full, Point::ORIGIN));
        assert_eq!(
            Edge::Left.reveal(full, 0.0),
            (Size::new(0.0, 500.0), Point::new(-300.0, 0.0))
        );
        assert_eq!(
            Edge::Left.reveal(full, 0.25),
            (Size::new(75.0, 500.0), Point::new(-225.0, 0.0))
        );
    }

    #[test]
    fn right_keeps_the_content_at_the_leading_edge() {
        let full = Size::new(300.0, 500.0);
        assert_eq!(
            Edge::Right.reveal(full, 0.25),
            (Size::new(75.0, 500.0), Point::ORIGIN)
        );
    }

    #[test]
    fn top_slides_up_and_bottom_slides_down() {
        let full = Size::new(300.0, 500.0);
        assert_eq!(
            Edge::Top.reveal(full, 0.5),
            (Size::new(300.0, 250.0), Point::new(0.0, -250.0))
        );
        assert_eq!(
            Edge::Bottom.reveal(full, 0.5),
            (Size::new(300.0, 250.0), Point::ORIGIN)
        );
    }

    #[test]
    fn partial_widths_land_on_whole_pixels() {
        assert_eq!(
            Edge::Left.reveal(Size::new(333.0, 10.0), 0.3333),
            (Size::new(111.0, 10.0), Point::new(-222.0, 0.0))
        );
    }

    #[test]
    fn a_widget_created_closed_reports_it_once() {
        let mut state = state(false);
        let now = Instant::now();
        assert_eq!(
            state.redraw(false, now),
            Frame {
                redraw: false,
                invalidate_layout: true,
                closed: true
            }
        );
        // Every later frame is silent until something moves again.
        assert_eq!(state.redraw(false, now + QUICK), Frame::default());
    }

    #[test]
    fn a_widget_created_open_settles_without_reporting_closed() {
        let mut state = state(true);
        let now = Instant::now();
        assert_eq!(
            state.redraw(true, now),
            Frame {
                redraw: false,
                invalidate_layout: true,
                closed: false
            }
        );
    }

    #[test]
    fn closing_reports_once_at_the_end_of_the_slide() {
        let mut state = state(true);
        let now = Instant::now();
        assert_eq!(
            state.redraw(true, now),
            Frame {
                redraw: false,
                invalidate_layout: true,
                closed: false
            }
        );

        assert!(state.retarget(false, now), "the slide starts");
        assert!(!state.retarget(false, now), "and only starts once");

        // Mid-slide: keep the frames coming, say nothing.
        let mid = state.redraw(false, now + QUICK / 2);
        assert!(mid.redraw && mid.invalidate_layout);
        assert!(!mid.closed);

        // Settled.
        assert_eq!(
            state.redraw(false, now + QUICK * 2),
            Frame {
                redraw: false,
                invalidate_layout: true,
                closed: true
            }
        );
        assert_eq!(state.redraw(false, now + QUICK * 3), Frame::default());
    }

    #[test]
    fn reversing_mid_slide_never_reports_closed() {
        let mut state = state(true);
        let now = Instant::now();
        let _ = state.redraw(true, now);

        state.retarget(false, now);
        let _ = state.redraw(false, now + QUICK / 2);
        // The application changed its mind before the slide finished.
        state.retarget(true, now + QUICK / 2);

        let settled = state.redraw(true, now + QUICK * 3);
        assert!(!settled.closed, "it ended up open, so nothing closed");
        assert_eq!(state.redraw(true, now + QUICK * 4), Frame::default());
    }

    #[test]
    fn reopening_after_a_close_reports_closed_only_for_the_close() {
        let mut state = state(true);
        let now = Instant::now();
        let _ = state.redraw(true, now);

        state.retarget(false, now);
        assert!(state.redraw(false, now + QUICK * 2).closed);

        state.retarget(true, now + QUICK * 2);
        assert!(!state.redraw(true, now + QUICK * 4).closed);
    }

    #[test]
    fn progress_runs_from_zero_to_one_over_the_duration() {
        // The same comparisons the widget itself makes: `draw` skips a
        // wrapper whose progress is `<= 0.0`, and treats the settled-open
        // case as the one that needs no clipping layer.
        let mut state = state(false);
        let now = Instant::now();
        assert!(state.progress(now) <= 0.0);
        assert!(!state.settled_open(now));

        state.retarget(true, now);
        assert!(state.progress(now + QUICK / 2) > 0.0);
        assert!(state.progress(now + QUICK * 2) >= 1.0);
        assert!(state.settled_open(now + QUICK * 2));
    }
}
