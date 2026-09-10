// SPDX-License-Identifier: MPL-2.0

//! The three sidebar widths, in a real window.
//!
//! `cargo run --example sidebar`
//!
//! The header button cycles expanded → rail → hidden. Watch the nav bar slide
//! between them, interrupt a slide mid-way by clicking again, and right-click
//! an entry in either width.
//!
//! Note what the application does *not* do: no timer, no per-frame message,
//! no width in its own state. The only thing it hears from the animation is
//! [`Message::SidebarClosed`], and only so that the element can leave the tree.

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Size};
use cosmic::prelude::*;
use cosmic::widget::{self, nav_bar};

use cosmic_ext_widgets::{Mode, SidebarState, nav_rail};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default().size(Size::new(900.0, 600.0));
    cosmic::app::run::<App>(settings, ())?;
    Ok(())
}

#[derive(Clone, Debug)]
enum Message {
    /// The one control: walk to the next width.
    ToggleSidebar,
    /// The sidebar has finished sliding out and can leave the widget tree.
    SidebarClosed,
    NavSelect(nav_bar::Id),
    NavContext(nav_bar::Id),
    Settings,
}

struct App {
    core: Core,
    nav: nav_bar::Model,
    sidebar: SidebarState,
    /// Only so the page can show that right-click reached us.
    last_context: Option<String>,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "org.magnetar.CosmicExtWidgetsSidebar";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, (): Self::Flags) -> (Self, Task<Self::Message>) {
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text("Inbox")
            .icon(widget::icon::from_name("mail-unread-symbolic"))
            .activate();
        nav.insert()
            .text("Drafts")
            .icon(widget::icon::from_name("document-edit-symbolic"));
        nav.insert()
            .text("Sent")
            .icon(widget::icon::from_name("mail-send-symbolic"));

        // A group below a divider: the rail draws it too, so the grouping
        // survives the narrow width.
        nav.insert()
            .text("Archive")
            .icon(widget::icon::from_name("folder-symbolic"))
            .divider_above(true);

        // Disabled in the model: drawn in both widths, pressable in neither.
        let mut junk = None;
        nav.insert()
            .text("Junk (disabled)")
            .icon(widget::icon::from_name("user-trash-symbolic"))
            .with_id(|id| junk = Some(id));
        if let Some(junk) = junk {
            nav.enable(junk, false);
        }

        let app = App {
            core,
            nav,
            sidebar: SidebarState::default(),
            last_context: None,
        };

        (app, Task::none())
    }

    /// `None`, so libcosmic pads nothing and draws no nav bar of its own —
    /// [`Self::nav_bar`] draws it instead. It also means libcosmic's header
    /// toggle is not shown, which is what we want: that toggle is binary and
    /// this sidebar has three widths, so the example brings its own.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        None
    }

    fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Self::Message>>> {
        let expanded = widget::nav_bar(&self.nav, Message::NavSelect)
            .on_context(Message::NavContext)
            .into_container()
            .width(Length::Shrink)
            .height(Length::Fill);

        let rail = nav_rail(&self.nav, Message::NavSelect)
            .on_context(Message::NavContext)
            .footer(
                widget::button::icon(widget::icon::from_name("preferences-system-symbolic"))
                    .on_press(Message::Settings),
            );

        self.sidebar
            .view(expanded, rail, Message::SidebarClosed)
            .map(|element| element.map(cosmic::Action::App))
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            widget::nav_bar_toggle()
                .active(self.sidebar.mode() != Mode::Hidden)
                .on_toggle(Message::ToggleSidebar)
                .into(),
        ]
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::ToggleSidebar => self.sidebar.cycle(),
            Message::SidebarClosed => self.sidebar.closed(),
            Message::NavSelect(id) => {
                self.nav.activate(id);
                self.last_context = None;
            }
            Message::NavContext(id) => {
                self.last_context = self.nav.text(id).map(ToOwned::to_owned);
            }
            Message::Settings => self.last_context = Some("the rail footer".into()),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let mode = match self.sidebar.mode() {
            Mode::Expanded => "Expanded",
            Mode::Rail => "Rail",
            Mode::Hidden => "Hidden",
        };

        let mut column = widget::column::with_capacity(4)
            .spacing(cosmic::theme::spacing().space_s)
            .align_x(Alignment::Center)
            .push(widget::text::title2(
                self.nav.text(self.nav.active()).unwrap_or("Nothing"),
            ))
            .push(widget::text::body(format!("Sidebar: {mode}")))
            .push(widget::text::caption(
                "The header button cycles expanded → rail → hidden.",
            ));

        if let Some(entry) = &self.last_context {
            column = column.push(widget::text::caption(format!("Right-clicked: {entry}")));
        }

        widget::container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    }
}
