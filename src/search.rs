// SPDX-License-Identifier: MPL-2.0

//! A search field whose clear button is there only when there is something
//! to clear.
//!
//! libcosmic's `search_input(..).on_clear(..)` attaches its clear button
//! unconditionally, so an empty field carries a control that can only do
//! nothing, competing with the placeholder. The button is also a backspace
//! glyph in a 32px box: on a pill that tall the button's own ground fills the
//! end cap and the icon sits over the curve.

use std::borrow::Cow;

use cosmic::Apply;
use cosmic::widget::{self, TextInput};

/// libcosmic's `search_input`, with a clear button that appears once
/// `value` is not empty and sends `on_clear` when pressed.
///
/// The button is a close mark in a small box held off the field's right
/// edge: the same affordance as `on_clear`'s, clear of the pill's curve, and
/// reading as "clear this" rather than "delete a character".
///
/// Everything else — `id`, `on_input`, `on_submit`, `width` — is chained on
/// the returned field as on any `TextInput`. Don't also call `on_clear` on
/// it: that would replace this button with libcosmic's.
pub fn search_field<'a, Message: Clone + 'static>(
    placeholder: impl Into<Cow<'a, str>>,
    value: &'a str,
    on_clear: Message,
) -> TextInput<'a, Message> {
    let field = widget::search_input(placeholder, value);
    if value.is_empty() {
        return field;
    }

    let spacing = cosmic::theme::spacing();
    field.trailing_icon(
        widget::icon::from_name("window-close-symbolic")
            .size(16)
            .apply(widget::button::custom)
            .class(cosmic::theme::Button::Icon)
            .on_press(on_clear)
            .padding(spacing.space_xxxs)
            .apply(widget::container)
            .padding([0, spacing.space_xxs, 0, 0])
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use cosmic::Element;
    use cosmic::iced::advanced::widget::Tree;

    use super::*;

    /// The field's children are its icons: the magnifier, and the clear
    /// button when there is one.
    fn icons(value: &str) -> usize {
        let element: Element<'_, ()> = search_field("Search", value, ()).into();
        Tree::new(&element).children.len()
    }

    #[test]
    fn the_clear_button_comes_and_goes_with_the_text() {
        assert_eq!(icons(""), 1);
        assert_eq!(icons("ada"), 2);
    }
}
