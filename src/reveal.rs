// SPDX-License-Identifier: MPL-2.0

//! A wrapper that animates its content between hidden and shown by sliding it
//! along one axis.

use std::time::{Duration, Instant};

use cosmic::Element;
use cosmic::iced::advanced::widget::{Operation, Tree, tree};
use cosmic::iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
};
use cosmic::iced::animation::{Animation, Easing};
use cosmic::iced::{Event, Length, Point, Rectangle, Size, Vector, window};

/// The axis a [`Reveal`] slides along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Axis {
    /// Width goes to zero; the content slides out to the left.
    #[default]
    Horizontal,
    /// Height goes to zero; the content slides up.
    Vertical,
}

impl Axis {
    /// The size the wrapper occupies at `progress` (0 = hidden, 1 = shown),
    /// and where the content sits inside it. The content keeps its natural
    /// size throughout and is clipped, so text does not reflow mid-slide; it
    /// is offset so the edge that disappears is the outer one.
    fn reveal(self, full: Size, progress: f32) -> (Size, Point) {
        match self {
            Self::Horizontal => {
                let width = (full.width * progress).round();
                (
                    Size::new(width, full.height),
                    Point::new(width - full.width, 0.0),
                )
            }
            Self::Vertical => {
                let height = (full.height * progress).round();
                (
                    Size::new(full.width, height),
                    Point::new(0.0, height - full.height),
                )
            }
        }
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
        axis: Axis::Horizontal,
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
    axis: Axis,
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

    pub const fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
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

struct State {
    anim: Animation<bool>,
    /// True while a transition is running, and on creation, so that the first
    /// settled frame after either is noticed exactly once.
    unsettled: bool,
}

impl State {
    fn progress(&self, now: Instant) -> f32 {
        self.anim.interpolate(0.0_f32, 1.0, now)
    }

    fn settled_open(&self, now: Instant) -> bool {
        self.anim.value() && !self.anim.is_animating(now)
    }
}

impl<Message: Clone> Widget<Message, cosmic::Theme, cosmic::Renderer> for Reveal<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            anim: Animation::new(self.open && !self.start_closed)
                .easing(self.easing)
                .duration(self.duration),
            unsettled: true,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        let content = self.content.as_widget().size();
        match self.axis {
            Axis::Horizontal => Size::new(Length::Shrink, content.height),
            Axis::Vertical => Size::new(content.width, Length::Shrink),
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
        let (size, origin) = self.axis.reveal(content.size(), progress);
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
        if state.anim.value() != self.open {
            state.anim.go_mut(self.open, now);
            state.unsettled = true;
            shell.request_redraw();
            shell.invalidate_layout();
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            if state.anim.is_animating(now) {
                shell.request_redraw();
                shell.invalidate_layout();
            } else if state.unsettled {
                state.unsettled = false;
                shell.invalidate_layout();
                if !self.open
                    && let Some(message) = &self.on_closed
                {
                    shell.publish(message.clone());
                }
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

    #[test]
    fn horizontal_slides_out_to_the_left() {
        let full = Size::new(300.0, 500.0);
        assert_eq!(Axis::Horizontal.reveal(full, 1.0), (full, Point::ORIGIN));
        assert_eq!(
            Axis::Horizontal.reveal(full, 0.0),
            (Size::new(0.0, 500.0), Point::new(-300.0, 0.0))
        );
        assert_eq!(
            Axis::Horizontal.reveal(full, 0.25),
            (Size::new(75.0, 500.0), Point::new(-225.0, 0.0))
        );
    }

    #[test]
    fn vertical_slides_up() {
        let full = Size::new(300.0, 500.0);
        assert_eq!(
            Axis::Vertical.reveal(full, 0.5),
            (Size::new(300.0, 250.0), Point::new(0.0, -250.0))
        );
    }

    #[test]
    fn partial_widths_land_on_whole_pixels() {
        assert_eq!(
            Axis::Horizontal.reveal(Size::new(333.0, 10.0), 0.3333),
            (Size::new(111.0, 10.0), Point::new(-222.0, 0.0))
        );
    }
}
