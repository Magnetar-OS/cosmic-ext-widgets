// SPDX-License-Identifier: MPL-2.0

//! An icon-only nav bar.
//!
//! libcosmic's nav bar has two states, open and gone. A rail is the third:
//! one icon per entry, its label in a tooltip, on the same surface as the
//! full nav bar so the two read as one thing at two widths.

use std::borrow::Cow;

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, nav_bar, segmented_button, tooltip};
use cosmic::{Apply, Element};

/// The rail surface around a column of [`rail_item`]s.
///
/// Same background, radius and scrollbar as libcosmic's nav bar, so that a
/// sidebar sliding between the two does not change colour on the way.
pub fn rail<'a, Message: 'static>(
    items: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let column = widget::column::with_children(items)
        .align_x(Alignment::Center)
        .spacing(spacing.space_xxs);

    widget::scrollable(widget::container(column).padding(spacing.space_xxs))
        .class(cosmic::style::iced::Scrollable::Minimal)
        .height(Length::Fill)
        .apply(widget::container)
        .height(Length::Fill)
        .class(cosmic::theme::Container::custom(nav_bar::nav_bar_style))
        .into()
}

/// One entry in a [`rail`]: `content` as an icon button, `label` as a tooltip
/// to its right.
///
/// `content` is anything 16px-ish — a symbolic icon for a page, a colour
/// swatch for a calendar. It gets libcosmic's icon-button padding, so a 16px
/// icon makes the standard 32px button.
pub fn rail_item<'a, Message: Clone + 'static>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<Cow<'a, str>> + 'a,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let button = widget::button::custom(content)
        .class(cosmic::theme::Button::Icon)
        .padding(spacing.space_xxs)
        .selected(selected)
        .on_press(on_press);

    widget::tooltip(button, widget::text::body(label), tooltip::Position::Right).into()
}

/// A [`rail`] over the same model a [`nav_bar`](cosmic::widget::nav_bar)
/// takes, so an application can show either from one source of truth.
///
/// Entries without an icon show the first character of their text.
pub fn nav_rail<'a, Message: Clone + 'static>(
    model: &'a nav_bar::Model,
    on_activate: fn(segmented_button::Entity) -> Message,
) -> Element<'a, Message> {
    rail(model.iter().map(|id| {
        let text = model.text(id).unwrap_or_default();
        let content: Element<'a, Message> = match model.icon(id) {
            Some(icon) => icon.clone().into(),
            None => {
                widget::text::body(text.chars().next().map(String::from).unwrap_or_default()).into()
            }
        };
        rail_item(content, text, model.is_active(id), on_activate(id))
    }))
}
