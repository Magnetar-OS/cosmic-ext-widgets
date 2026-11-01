// SPDX-License-Identifier: MPL-2.0

//! An icon button for the header bar.
//!
//! libcosmic's header bar gives its content a fixed 32px row, and its own
//! window controls are 16px icons with a hard-coded 8px padding to fill that
//! row exactly. `button::icon` pads by the theme's `space_xxs` instead, which
//! follows the interface density: 8px at Comfortable, but 12px at Spacious,
//! where the button is 40px tall and the header row cuts off the top of it.
//! [`header_button`] sizes the button the way the window controls are sized,
//! so it fits at every density and lines up with them.

use std::borrow::Cow;

use cosmic::widget::{self, icon, tooltip};
use cosmic::{Apply, Element};

/// The icon size and padding of libcosmic's window controls: 16 + 2 × 8 is
/// the header bar's 32px row.
const ICON_SIZE: u16 = 16;
const PADDING: u16 = 8;

/// An icon button for `header_start`, `header_center` or `header_end`, with
/// `label` as its tooltip below it.
///
/// `on_press` may be `None` for an action that is unavailable; the button
/// then draws disabled.
///
/// The label is also the button's accessible name: a glyph on its own tells a
/// screen reader nothing, and the tooltip reaches nothing but the eye.
pub fn header_button<'a, Message>(
    icon: impl Into<icon::Handle>,
    label: impl Into<Cow<'a, str>>,
    on_press: impl Into<Option<Message>>,
) -> HeaderButton<'a, Message> {
    HeaderButton {
        icon: icon.into(),
        label: label.into(),
        on_press: on_press.into(),
        focused: true,
    }
}

/// See [`header_button`].
#[must_use]
pub struct HeaderButton<'a, Message> {
    icon: icon::Handle,
    label: Cow<'a, str>,
    on_press: Option<Message>,
    focused: bool,
}

impl<Message> HeaderButton<'_, Message> {
    /// Whether the window is focused. The header bar dims its window controls
    /// in an unfocused window, and a button told so here dims with them.
    /// Defaults to `true`.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<'a, Message: Clone + 'static> From<HeaderButton<'a, Message>> for Element<'a, Message> {
    fn from(button: HeaderButton<'a, Message>) -> Self {
        let pressable = button
            .icon
            .apply(widget::button::icon)
            .icon_size(ICON_SIZE)
            .padding(PADDING)
            .class(cosmic::theme::Button::HeaderBar)
            .selected(button.focused)
            .on_press_maybe(button.on_press);

        #[cfg(feature = "a11y")]
        let pressable = pressable.name(button.label.clone());

        widget::tooltip(
            pressable,
            widget::text::body(button.label),
            tooltip::Position::Bottom,
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use cosmic::iced::advanced::layout::Limits;
    use cosmic::iced::advanced::widget::Tree;
    use cosmic::iced::{Font, Pixels, Size};

    use super::*;

    /// The global theme cannot be switched to Spacious from outside
    /// libcosmic, so this pins the size the button must have at every
    /// density rather than demonstrating the overflow.
    #[test]
    fn fills_the_header_row_exactly() {
        let mut element: Element<'_, ()> =
            header_button(icon::from_name("document-new-symbolic"), "New", ()).into();
        let mut tree = Tree::new(&element);
        let renderer: cosmic::Renderer =
            iced_tiny_skia::Renderer::new(Font::default(), Pixels(14.0));
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &Limits::new(Size::ZERO, Size::new(200.0, 200.0)),
        );
        assert_eq!(node.size(), Size::new(32.0, 32.0));
    }
}
