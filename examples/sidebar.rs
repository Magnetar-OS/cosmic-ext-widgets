// SPDX-License-Identifier: MPL-2.0

//! The three sidebar widths, in a real window.
//!
//! `cargo run --example sidebar`
//!
//! The header button cycles expanded → rail → hidden. Watch the nav bar slide
//! between them, interrupt a slide mid-way by clicking again, and right-click
//! an entry in either width. Narrow the window past libcosmic's condensed
//! breakpoint and the sidebar hides itself; the button then shows and hides
//! it, as the stock toggle does on a narrow window.
//!
//! Note what the application does *not* do: no timer, no per-frame message,
//! no width in its own state. The only thing it hears from the animation is
//! [`Message::SidebarClosed`], and only so that the element can leave the tree.
//!
//! Shown-at-all stays libcosmic's own nav-bar state, `core.nav_bar_active()`:
//! libcosmic pads the main content by it, and flips it itself at the condensed
//! breakpoint. The application keeps only the shape it shows in — full or
//! rail — and [`App::sync_sidebar`] folds the two into the sidebar's mode.

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
    /// The width last chosen while the sidebar was showing, so a narrow window
    /// hiding it and a wide one bringing it back does not change its shape.
    rail: bool,
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

        let mut app = App {
            core,
            nav,
            sidebar: SidebarState::default(),
            rail: false,
            last_context: None,
        };
        app.sync_sidebar();

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
        // `Shrink`, and capped as libcosmic caps its own: a nav bar is `Fill`
        // wide by default and would otherwise claim the whole row.
        let mut expanded = widget::nav_bar(&self.nav, Message::NavSelect)
            .on_context(Message::NavContext)
            .into_container()
            .width(Length::Shrink)
            .height(Length::Fill);
        if !self.core.is_condensed() {
            expanded = expanded.max_width(280);
        }

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

    fn on_window_resize(&mut self, _id: cosmic::iced::window::Id, _width: f32, _height: f32) {
        // Crossing libcosmic's condensed breakpoint flips its nav-bar state
        // without a message of ours; this is where it tells us.
        self.sync_sidebar();
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::ToggleSidebar => self.toggle_sidebar(),
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

impl App {
    /// One button, three widths: each press takes the sidebar a step
    /// narrower, and from nothing back to full.
    ///
    /// On a condensed window the button only shows and hides, through
    /// libcosmic's condensed state, so that hiding the sidebar to read a page
    /// on a small window does not lose the wide window's choice.
    fn toggle_sidebar(&mut self) {
        if self.core.is_condensed() {
            self.core.nav_bar_toggle_condensed();
        } else {
            match self.sidebar.mode() {
                Mode::Expanded => self.rail = true,
                Mode::Rail => {
                    self.rail = false;
                    self.core.nav_bar_set_toggled(false);
                }
                Mode::Hidden => self.core.nav_bar_set_toggled(true),
            }
        }
        self.sync_sidebar();
    }

    /// Folds libcosmic's shown/hidden nav-bar state and the chosen width into
    /// the sidebar's mode. Idempotent, so it is called wherever either input
    /// can change.
    fn sync_sidebar(&mut self) {
        let mode = if !self.core.nav_bar_active() {
            Mode::Hidden
        } else if self.rail {
            Mode::Rail
        } else {
            Mode::Expanded
        };
        self.sidebar.set_mode(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App {
            core: Core::default(),
            nav: nav_bar::Model::default(),
            sidebar: SidebarState::default(),
            rail: false,
            last_context: None,
        };
        app.sync_sidebar();
        app
    }

    #[test]
    fn the_toggle_walks_all_three_widths_and_back() {
        let mut app = app();
        assert_eq!(app.sidebar.mode(), Mode::Expanded);

        app.toggle_sidebar();
        assert_eq!(app.sidebar.mode(), Mode::Rail);

        app.toggle_sidebar();
        assert_eq!(app.sidebar.mode(), Mode::Hidden);

        app.toggle_sidebar();
        assert_eq!(app.sidebar.mode(), Mode::Expanded);
    }

    #[test]
    fn libcosmic_knows_whether_the_sidebar_is_shown() {
        // libcosmic pads the main content's leading edge only when its own
        // nav-bar state says no nav bar is shown. Driving the sidebar alone
        // would leave that state `true` while hidden: content flush against
        // the window edge.
        let mut app = app();
        for _ in 0..6 {
            app.toggle_sidebar();
            assert_eq!(
                app.core.nav_bar_active(),
                app.sidebar.mode() != Mode::Hidden,
                "in {:?}",
                app.sidebar.mode()
            );
        }
    }

    #[test]
    fn a_hide_from_outside_keeps_the_chosen_width() {
        // What the condensed breakpoint does to libcosmic's state, seen
        // through the only public door: the sidebar follows it, and comes
        // back in the width it left in.
        let mut app = app();
        app.toggle_sidebar();
        assert_eq!(app.sidebar.mode(), Mode::Rail);

        app.core.nav_bar_set_toggled(false);
        app.sync_sidebar();
        assert_eq!(app.sidebar.mode(), Mode::Hidden);

        app.core.nav_bar_set_toggled(true);
        app.sync_sidebar();
        assert_eq!(app.sidebar.mode(), Mode::Rail);
    }
}
