# cosmic-ext-widgets

Widgets libcosmic does not have, built the way libcosmic builds its own.

| | libcosmic | this crate |
|---|---|---|
| Collapsing the nav bar | instant show/hide, at every call site | `reveal` — a width (or height) slide, interruptible |
| Collapsed nav bar | gone | `rail` / `nav_rail` — icons with tooltips, on the nav-bar surface |
| Both, in the nav-bar slot | — | `SidebarState` — expanded → rail → hidden, animated, dropped from the tree once hidden |

Each is usable alone. `reveal` wraps any element; `nav_rail` takes the same
`nav_bar::Model` the stock nav bar does; `rail_item` takes arbitrary content
for sidebars that are not a page list.

## Using it

```toml
[dependencies]
cosmic-ext-widgets = { git = "https://github.com/entro314-labs/cosmic-ext-widgets" }
```

libcosmic is unpinned here, as in every COSMIC application. Your `Cargo.lock`
picks the revision and cargo unifies the two.

```rust
use cosmic_ext_widgets::{Mode, SidebarState, nav_rail};

struct App { core: cosmic::Core, nav: nav_bar::Model, sidebar: SidebarState, /* … */ }

enum Message { ToggleSidebar, SidebarClosed, NavSelect(nav_bar::Id), /* … */ }

impl cosmic::Application for App {
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        // Keep libcosmic's chrome and its header toggle, but draw the bar ourselves.
        None
    }

    fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Message>>> {
        let expanded = widget::nav_bar(&self.nav, Message::NavSelect);
        let rail = nav_rail(&self.nav, Message::NavSelect);
        self.sidebar
            .view(expanded, rail, Message::SidebarClosed)
            .map(|element| element.map(cosmic::Action::App))
    }

    fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::ToggleSidebar => {
                let next = match self.sidebar.mode() {
                    Mode::Expanded => Mode::Rail,
                    Mode::Rail => Mode::Hidden,
                    Mode::Hidden => Mode::Expanded,
                };
                self.sidebar.set_mode(next);
            }
            Message::SidebarClosed => self.sidebar.closed(),
            // …
        }
        Task::none()
    }
}
```

`set_mode` is idempotent. If libcosmic's condensed breakpoint should hide the
sidebar on narrow windows, call it from `on_window_resize` with a mode derived
from `core.nav_bar_active()`.

## How the animation runs

The clock is in the widget tree, driven from `RedrawRequested`, exactly as
libcosmic's `toggler` and `cards` do it. The application holds no timer and
sends no per-frame message. Interpolation is iced's `Animation` (lilt), which
libcosmic already ships: reversing a half-finished slide continues from where
it is. Default 200 ms, ease-out cubic; `reveal` exposes both.

## One thing to know about the nav-bar slot

libcosmic pads whatever `nav_bar()` returns and only removes that padding
when it returns `None`. A sidebar that has slid down to zero width still costs
about 8 px until it is dropped. `SidebarState` drops it on the frame after the
slide finishes; that last 8 px is a step, not a slide. Hosting the sidebar
inside your own `view()` avoids it at the cost of re-deriving libcosmic's
window padding, which changes with maximisation.

## Licence

MPL-2.0, like libcosmic.
