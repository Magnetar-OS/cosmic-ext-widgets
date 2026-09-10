# cosmic-ext-widgets

Widgets libcosmic does not have, built the way libcosmic builds its own.

| | libcosmic | this crate |
|---|---|---|
| Collapsing the nav bar | instant show/hide, at every call site | `reveal` — a width (or height) slide, interruptible, on any of the four edges |
| Collapsed nav bar | gone | `rail` / `nav_rail` — icons with tooltips, on the nav-bar surface |
| Both, in the nav-bar slot | — | `SidebarState` — expanded → rail → hidden, animated, dropped from the tree once hidden |

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

struct App { core: cosmic::Core, nav: nav_bar::Model, sidebar: SidebarState, /* … */ }

enum Message { ToggleSidebar, SidebarClosed, NavSelect(nav_bar::Id), /* … */ }

impl cosmic::Application for App {
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        // Draw the bar ourselves. This also hides libcosmic's own header
        // toggle — see below, it could not drive three widths anyway.
        None
    }

    fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Message>>> {
        let expanded = widget::nav_bar(&self.nav, Message::NavSelect);
        let rail = nav_rail(&self.nav, Message::NavSelect);
        self.sidebar
            .view(expanded, rail, Message::SidebarClosed)
            .map(|element| element.map(cosmic::Action::App))
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        // libcosmic's own toggle, wired to our three-way cycle.
        vec![widget::nav_bar_toggle()
            .active(self.sidebar.mode() != Mode::Hidden)
            .on_toggle(Message::ToggleSidebar)
            .into()]
    }

    fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::ToggleSidebar => self.sidebar.cycle(),
            Message::SidebarClosed => self.sidebar.closed(),
            // …
        }
        Task::none()
    }
}
```

`cycle` walks expanded → rail → hidden → expanded. `set_mode` goes straight to
one and is idempotent, so it is safe to call on every event that might have
changed the answer: if libcosmic's condensed breakpoint should hide the
sidebar on narrow windows, call it from `on_window_resize`.

### The toggle has to be yours

libcosmic draws its header toggle only when `nav_model()` returns `Some`, and
that toggle is binary — it flips `core.nav_bar_active()`, which the
application cannot intercept and which cannot express three widths. So a
three-width sidebar returns `None` from `nav_model()` and puts
`widget::nav_bar_toggle()` in `header_start()` itself, as above. It is
libcosmic's own widget, so the header still looks like every other COSMIC
application's.

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

- `a11y` (default) — give the rail's icon-only buttons an accessible name.
  Without it the label exists only in a tooltip, which reaches nothing but the
  eye. On by default because libcosmic's own default features include `a11y`,
  and because the setter this needs is itself behind that feature upstream.

## Known gap: `reveal` and the accessibility tree

Content inside a `reveal` does not appear in the accessibility tree, even when
fully open. `Widget::a11y_nodes` returns an empty tree unless a widget
overrides it, and the override cannot be written from outside libcosmic:
its return type is `iced_accessibility::A11yTree`, and libcosmic keeps
`iced_accessibility` as a private dependency. Depending on
`pop-os/iced` directly would give a *second* copy of the crate — a different
type of the same name, which does not satisfy the trait.

The fix is one line upstream, `pub use iced_accessibility;` in libcosmic's
`src/lib.rs`, after which `Reveal` can forward to its child under the same
condition `draw` and `update` already use. Until then a sidebar drawn through
`SidebarState` is invisible to a screen reader. The rail's buttons carry their
names either way, so the gap is `reveal`'s, not the rail's.

## One thing to know about the nav-bar slot

libcosmic pads whatever `nav_bar()` returns and only removes that padding
when it returns `None`. A sidebar that has slid down to zero width still costs
about 8 px until it is dropped. `SidebarState` drops it on the frame after the
slide finishes; that last 8 px is a step, not a slide. Hosting the sidebar
inside your own `view()` avoids it at the cost of re-deriving libcosmic's
window padding, which changes with maximisation.

## Licence

MPL-2.0, like libcosmic.
