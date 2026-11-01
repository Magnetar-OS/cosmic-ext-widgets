# cosmic-ext-widgets

Widgets libcosmic does not have, built the way libcosmic builds its own.

| | libcosmic | this crate |
|---|---|---|
| Collapsing the nav bar | instant show/hide, at every call site | `reveal` — a width (or height) slide, interruptible, on any of the four edges |
| Collapsed nav bar | gone | `rail` / `nav_rail` — icons with tooltips, on the nav-bar surface |
| Both, in the nav-bar slot | — | `SidebarState` — expanded → rail → hidden, animated, dropped from the tree once hidden |
| Icon button in the header bar | `button::icon`, cut off at Spacious density | `header_button` — the window controls' size at every density, with a tooltip and an accessible name |

Each is usable alone. `reveal` wraps any element; `nav_rail` takes the same
`nav_bar::Model` the stock nav bar does; `rail_item` takes arbitrary content
for sidebars that are not a page list.

```sh
cargo run --example sidebar
```

opens a window with all three widths and one button that walks between them.

## Using it

```toml
[dependencies]
cosmic-ext-widgets = { git = "https://github.com/Magnetar-OS/cosmic-ext-widgets" }
```

libcosmic is unpinned here, as in every COSMIC application. Your `Cargo.lock`
picks the revision and cargo unifies the two.

```rust
use cosmic_ext_widgets::{Mode, SidebarState, nav_rail};

struct App {
    core: cosmic::Core,
    nav: nav_bar::Model,
    sidebar: SidebarState,
    /// The width chosen while showing: full or rail. Shown-at-all is
    /// libcosmic's, in `core`.
    rail: bool,
    /* … */
}

enum Message { ToggleSidebar, SidebarClosed, NavSelect(nav_bar::Id), /* … */ }

impl cosmic::Application for App {
    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut app = App { core, sidebar: SidebarState::default(), rail: false, /* … */ };
        app.sync_sidebar();
        (app, Task::none())
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        // Draw the bar ourselves. This also hides libcosmic's own header
        // toggle — see below, it could not drive three widths anyway.
        None
    }

    fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Message>>> {
        // A nav bar is `Fill` wide on its own; shrink and cap it as
        // libcosmic's default nav bar does, or it claims the whole row.
        let mut expanded = widget::nav_bar(&self.nav, Message::NavSelect)
            .into_container()
            .width(Length::Shrink)
            .height(Length::Fill);
        if !self.core.is_condensed() {
            expanded = expanded.max_width(280);
        }
        let rail = nav_rail(&self.nav, Message::NavSelect);
        self.sidebar
            .view(expanded, rail, Message::SidebarClosed)
            .map(|element| element.map(cosmic::Action::App))
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        // libcosmic's own toggle, wired to our three widths.
        vec![widget::nav_bar_toggle()
            .active(self.sidebar.mode() != Mode::Hidden)
            .on_toggle(Message::ToggleSidebar)
            .into()]
    }

    fn on_window_resize(&mut self, _id: window::Id, _width: f32, _height: f32) {
        // Crossing the condensed breakpoint flips libcosmic's nav-bar state
        // without a message of ours; this is where it tells us.
        self.sync_sidebar();
    }

    fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::ToggleSidebar => {
                if self.core.is_condensed() {
                    // A narrow window only shows and hides.
                    self.core.nav_bar_toggle_condensed();
                } else {
                    // Each press a step narrower, and from nothing back to full.
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
            Message::SidebarClosed => self.sidebar.closed(),
            // …
        }
        Task::none()
    }
}

impl App {
    /// libcosmic's shown/hidden state and our width, folded into one mode.
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
```

`set_mode` goes straight to a mode and is idempotent, so `sync_sidebar` is
safe to call on every event that might have changed the answer.

### Keep libcosmic's nav-bar state in step

libcosmic decides the main content's leading padding from
`core.nav_bar_active()`, not from whether `nav_bar()` returned anything. So
shown-at-all has to stay in `core`: hide the sidebar through
`core.nav_bar_set_toggled` (or `nav_bar_toggle_condensed` on a condensed
window) and derive the mode from `core.nav_bar_active()`, as above. Drive
`SidebarState` alone and a hidden sidebar leaves the content flush against the
window edge, while a condensed window pads it twice. Keeping it in `core` also
brings libcosmic's responsive behaviour with it: the condensed breakpoint hides
the sidebar on a narrow window, and `on_window_resize` is where the app hears
about it. `Mode::next` / `cycle` walk the three widths without touching `core`,
so they suit a sidebar hosted outside the nav-bar slot.

### The toggle has to be yours

libcosmic draws its header toggle only when `nav_model()` returns `Some`, and
that toggle is binary: it flips `core`'s nav-bar state, which cannot express
three widths. So a three-width sidebar returns `None` from `nav_model()` and
puts `widget::nav_bar_toggle()` in `header_start()` itself, as above, driving
`core` for the hidden step. It is libcosmic's own widget, so the header still
looks like every other COSMIC application's.

The example, `examples/sidebar.rs`, is this integration in full, and its tests
check that `core` and the sidebar agree at every press.

### The rail follows the model

`nav_rail` reads the same entry state the expanded bar does, so the two widths
agree: an entry the model has disabled is drawn but not pressable, and an
entry marked `divider_above` gets its divider here too, so grouping survives
the narrow width. `on_context` reports right-clicks the way `nav_bar`'s does.

`header` and `footer` pin content outside the scroll area, which is where a
rail conventionally keeps the things that are not pages.

### Other edges

`reveal` and `SidebarState` take an `Edge` — the edge the content slides out
past. `Edge::Left` (the default) is a nav bar; `Edge::Right` is an inspector
panel; `Edge::Top` and `Edge::Bottom` are drawers. `SidebarState` also uses it
to decide which way round the two widths sit, so that whichever one is showing
stays against its own edge.

### Header-bar buttons

```rust
use cosmic_ext_widgets::header_button;

fn header_start(&self) -> Vec<Element<'_, Message>> {
    vec![
        header_button(icon::from_name("document-new-symbolic"), fl!("new"), Message::New)
            // The test libcosmic's header bar uses for its window controls.
            .focused(
                self.core
                    .focus_chain()
                    .iter()
                    .any(|id| Some(*id) == self.core.main_window_id()),
            )
            .into(),
    ]
}
```

The header bar lays its content out in a fixed 32px row. `button::icon` pads
its icon by the theme's `space_xxs`, which is 12px at Spacious density, so
the button is 40px tall and the header cuts off its top. `header_button` uses
the 16px icon and 8px padding of libcosmic's own window controls, which fill
the row exactly at every density. `focused` dims it with the window controls
when the window loses focus; it defaults to `true`.

## How the animation runs

The clock is in the widget tree, driven from `RedrawRequested`, as libcosmic's
own animated widgets do it — the application holds no timer and sends no
per-frame message. Interpolation is iced's `Animation` (lilt), which libcosmic
already ships: reversing a half-finished slide continues from where it is.
Default 200 ms, ease-out cubic; both `reveal` and `SidebarState` expose both.

What a frame does about the slide — ask for another, invalidate the layout,
report that the content has settled hidden — is decided in one place that
takes the instant as an argument and touches neither the clock nor the shell,
so the behaviour that matters is unit-tested rather than watched.

## Features

- `a11y` (default) — give the rail's icon-only buttons an accessible name, and
  have a `reveal` hand on its content's accessibility nodes. Without it a rail
  label exists only in a tooltip, which reaches nothing but the eye, and what
  is inside a `reveal` is missing from the accessibility tree. On by default
  because libcosmic's own default features include `a11y`, and because what
  this needs is itself behind that feature upstream.

## `reveal` and the accessibility tree

A `reveal` that has settled open returns its content's accessibility nodes
unchanged, adding none of its own, so a sidebar drawn through `SidebarState`
is in the tree like any other content. While it is closed or sliding it
returns none: the same condition under which it withholds input, focus and
overlays, so a screen reader is not offered a control that cannot be used yet.

`Widget::a11y_nodes` returns `iced_accessibility::A11yTree`, a type libcosmic
uses but does not re-export. This crate depends on `iced_accessibility` by
libcosmic's own git URL, as it does on libcosmic, so cargo resolves the one
copy libcosmic is built against and the consumer's `Cargo.lock` gains no
package. A consumer that pins libcosmic to a `rev` or `branch` in its manifest
gives it a different source, and would have to pin this the same way.

What reaches a screen reader is libcosmic's to decide. At libcosmic `5a8bd94`
its window shell does not collect `a11y_nodes` from the widget tree at all,
for its own widgets or anyone's; when it does, a `reveal`'s content is there
to be collected.

## One thing to know about the nav-bar slot

libcosmic pads whatever `nav_bar()` returns and only removes that padding
when it returns `None`. A sidebar that has slid down to zero width still costs
about 8 px until it is dropped. `SidebarState` drops it on the frame after the
slide finishes; that last 8 px is a step, not a slide. Hosting the sidebar
inside your own `view()` avoids it at the cost of re-deriving libcosmic's
window padding, which changes with maximisation.

## Licence

MPL-2.0, like libcosmic.
