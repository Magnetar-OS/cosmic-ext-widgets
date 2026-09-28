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
use unicode_segmentation::UnicodeSegmentation;

/// The rail surface around a column of [`rail_item`]s.
///
/// Same background, radius and scrollbar as libcosmic's nav bar, so that a
/// sidebar sliding between the two does not change colour on the way.
pub fn rail<'a, Message: 'static>(
    items: impl IntoIterator<Item = Element<'a, Message>>,
) -> Rail<'a, Message> {
    Rail {
        items: items.into_iter().collect(),
        header: None,
        footer: None,
    }
}

/// See [`rail`].
#[must_use]
pub struct Rail<'a, Message> {
    items: Vec<Element<'a, Message>>,
    header: Option<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
}

impl<'a, Message> Rail<'a, Message> {
    /// Pinned above the items, outside the scroll area.
    pub fn header(mut self, header: impl Into<Element<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Pinned below the items, outside the scroll area — where a rail
    /// conventionally keeps the things that are not pages, such as settings.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

impl<'a, Message: 'a> From<Rail<'a, Message>> for Element<'a, Message> {
    fn from(rail: Rail<'a, Message>) -> Self {
        let spacing = cosmic::theme::spacing();

        // Both columns are `Shrink` wide, set after their children because a
        // column adopts `Fill` from any child that has it. A rail is as wide
        // as its widest item; a full-width child such as a divider is then
        // stretched to that width instead of stretching the rail to the row.
        let items = widget::column::with_children(rail.items)
            .align_x(Alignment::Center)
            .spacing(spacing.space_xxs)
            .width(Length::Shrink)
            .apply(widget::scrollable)
            .class(cosmic::style::iced::Scrollable::Minimal)
            .height(Length::Fill);

        let mut column = widget::column::with_capacity(3)
            .align_x(Alignment::Center)
            .spacing(spacing.space_xxs)
            .height(Length::Fill);

        if let Some(header) = rail.header {
            column = column.push(header);
        }
        column = column.push(items);
        if let Some(footer) = rail.footer {
            column = column.push(footer);
        }

        widget::container(column.width(Length::Shrink))
            .padding(spacing.space_xxs)
            .height(Length::Fill)
            .class(cosmic::theme::Container::custom(nav_bar::nav_bar_style))
            .into()
    }
}

/// One entry in a [`rail`]: `content` as an icon button, `label` as a tooltip
/// to its right.
///
/// `content` is anything 16px-ish — a symbolic icon for a page, a colour
/// swatch for a calendar. It gets libcosmic's icon-button padding, so a 16px
/// icon makes the standard 32px button.
///
/// `on_press` may be `None` for an entry that is shown but not selectable;
/// the button then draws disabled, as libcosmic's nav bar draws an entry the
/// model has disabled.
///
/// The label is also the button's accessible name, because in a rail the
/// tooltip is the only other place it appears and a tooltip reaches nothing
/// but the eye.
pub fn rail_item<'a, Message: Clone + 'static>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<Cow<'a, str>>,
    selected: bool,
    on_press: impl Into<Option<Message>>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    let label = label.into();

    let button = widget::button::custom(content)
        .class(cosmic::theme::Button::Icon)
        .padding(spacing.space_xxs)
        .selected(selected)
        .on_press_maybe(on_press.into());

    #[cfg(feature = "a11y")]
    let button = button.name(label.clone());

    widget::tooltip(button, widget::text::body(label), tooltip::Position::Right).into()
}

/// A [`rail()`] over the same model a [`nav_bar()`](cosmic::widget::nav_bar())
/// takes, so an application can show either from one source of truth.
///
/// Entries without an icon show the first character of their text. Entries
/// the model has disabled are drawn but not pressable, and an entry the model
/// marks with a divider above gets one here too, so that grouping survives
/// the narrow width.
pub fn nav_rail<Message: Clone + 'static>(
    model: &nav_bar::Model,
    on_activate: fn(segmented_button::Entity) -> Message,
) -> NavRail<'_, Message> {
    NavRail {
        model,
        on_activate,
        on_context: None,
        header: None,
        footer: None,
    }
}

/// See [`nav_rail`].
#[must_use]
pub struct NavRail<'a, Message> {
    model: &'a nav_bar::Model,
    on_activate: fn(segmented_button::Entity) -> Message,
    on_context: Option<Box<dyn Fn(segmented_button::Entity) -> Message + 'a>>,
    header: Option<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
}

impl<'a, Message> NavRail<'a, Message> {
    /// Emitted when an entry is right-clicked, as
    /// [`nav_bar()`](cosmic::widget::nav_bar())'s `on_context` is.
    ///
    /// The message carries the entry; showing a menu for it is the
    /// application's, since the rail draws outside libcosmic's chrome.
    pub fn on_context(
        mut self,
        on_context: impl Fn(segmented_button::Entity) -> Message + 'a,
    ) -> Self {
        self.on_context = Some(Box::new(on_context));
        self
    }

    /// Pinned above the entries. See [`Rail::header`].
    pub fn header(mut self, header: impl Into<Element<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Pinned below the entries. See [`Rail::footer`].
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

impl<'a, Message: Clone + 'static> From<NavRail<'a, Message>> for Element<'a, Message> {
    fn from(nav_rail: NavRail<'a, Message>) -> Self {
        let NavRail {
            model,
            on_activate,
            on_context,
            header,
            footer,
        } = nav_rail;

        let mut items: Vec<Element<'a, Message>> = Vec::with_capacity(model.iter().count());

        for (nth, id) in model.iter().enumerate() {
            // A divider above the first entry would sit against the top edge,
            // which is what libcosmic's own vertical layout avoids.
            if nth > 0 && model.divider_above(id).unwrap_or(false) {
                items.push(widget::divider::horizontal::default().into());
            }

            let text = model.text(id).unwrap_or_default();
            let content: Element<'a, Message> = match model.icon(id) {
                Some(icon) => icon.clone().into(),
                None => widget::text::body(initial(text).to_owned()).into(),
            };

            let item = rail_item(
                content,
                text,
                model.is_active(id),
                model.is_enabled(id).then(|| on_activate(id)),
            );

            items.push(match &on_context {
                Some(on_context) => widget::mouse_area(item)
                    .on_right_press(on_context(id))
                    .into(),
                None => item,
            });
        }

        let mut rail = rail(items);
        if let Some(header) = header {
            rail = rail.header(header);
        }
        if let Some(footer) = footer {
            rail = rail.footer(footer);
        }
        rail.into()
    }
}

/// What an entry without an icon shows: the first character of its text as a
/// reader sees it. A flag, an emoji sequence or a letter with a combining
/// accent is several `char`s, and the first of them alone is a fragment.
fn initial(text: &str) -> &str {
    text.graphemes(true).next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_without_an_icon_shows_its_first_whole_character() {
        assert_eq!(initial("Inbox"), "I");
        assert_eq!(initial("Ημερολόγιο"), "Η");
        assert_eq!(initial(""), "");
        // A flag is two regional indicators.
        assert_eq!(initial("🇬🇷 Athens"), "🇬🇷");
        // e + combining acute accent.
        assert_eq!(initial("e\u{301}cole"), "e\u{301}");
        // woman + ZWJ + laptop.
        assert_eq!(initial("👩\u{200d}💻 Work"), "👩\u{200d}💻");
    }

    /// The entity itself is message enough to route `on_activate`.
    type Message = segmented_button::Entity;

    fn width(element: &Element<'_, Message>) -> Length {
        element.as_widget().size_hint().width
    }

    #[test]
    fn a_divider_does_not_widen_the_rail() {
        // A horizontal divider is `Fill` wide. Without a width of its own the
        // column adopts that, and the rail claims the whole row.
        let mut model = nav_bar::Model::default();
        model
            .insert()
            .text("Inbox")
            .icon(widget::icon::from_name("mail-unread-symbolic"))
            .activate();
        model
            .insert()
            .text("Archive")
            .icon(widget::icon::from_name("folder-symbolic"))
            .divider_above(true);

        let element: Element<'_, Message> = nav_rail(&model, std::convert::identity).into();
        assert_eq!(width(&element), Length::Shrink);
    }

    #[test]
    fn a_full_width_header_or_footer_does_not_widen_the_rail() {
        let item = || rail_item(widget::text::body("A"), "A", false, None::<Message>);

        let element: Element<'_, Message> = rail([item()])
            .header(widget::divider::horizontal::default())
            .into();
        assert_eq!(width(&element), Length::Shrink);

        let element: Element<'_, Message> = rail([item()])
            .footer(widget::divider::horizontal::default())
            .into();
        assert_eq!(width(&element), Length::Shrink);
    }
}
