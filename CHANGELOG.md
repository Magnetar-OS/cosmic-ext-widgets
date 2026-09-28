# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

It is consumed by git tag, so each version below is a tag of that name.

## [Unreleased]

### Fixed

- `rail` and `nav_rail` stay as narrow as their widest item when a child is
  `Fill` wide. An entry with `divider_above`, or a full-width header or
  footer, used to make the rail claim the whole window row.
- A `SidebarState` created in `Mode::Hidden` now slides in the first time it
  is shown, instead of appearing at full width in one frame.
- Focus and scroll operations no longer reach a `Reveal`'s content while it
  is still sliding open. Focus could land on content whose keyboard input the
  widget was still holding back, so the first keys typed were lost.
- `nav_rail` shows the first whole character of an entry without an icon.
  A flag, emoji sequence or accented letter written with a combining mark
  used to be cut to its first code point.
- The README's integration and `examples/sidebar.rs` keep libcosmic's
  nav-bar state in step with the sidebar: shown-at-all lives in
  `core.nav_bar_active()` and the mode is derived from it. Following the old
  instructions, a hidden sidebar left the main content flush against the
  window edge and a condensed window padded it twice. The README's expanded
  nav bar is also `Shrink` wide and capped at 280 px, as libcosmic's own is;
  as written before, it claimed the whole row.

### Changed

- `Reveal::duration` and `Reveal::easing` take effect when changed on a live
  widget, from its next slide. They used to be read only when the widget was
  first created. A slide already running finishes with the timing it started
  with.
- The crate is marked `publish = false`: it builds on libcosmic, which is only
  published as a git repository, so crates.io cannot take it.

## [1.0.0] - 2026-09-10

### Added

- `reveal` / `Reveal`: animates its content's width or height between zero
  and its natural size, along any of the four edges (`Edge`). The slide is
  interruptible, clipped rather than reflowed, driven from the widget tree's
  own clock, and reports `on_closed` once the content has settled hidden.
- `rail`, `rail_item` and `nav_rail`: an icon-only nav bar on libcosmic's
  nav-bar surface, with tooltips, pinned `header`/`footer`, and `nav_rail`
  reading the same `nav_bar::Model` as the stock nav bar (icons, active,
  disabled, dividers, `on_context`).
- `SidebarState` and `Mode`: a sidebar with expanded, rail and hidden widths
  in libcosmic's nav-bar slot, animated between them and dropped from the
  tree once hidden.
- `a11y` feature (default): the rail's icon-only buttons carry their label as
  their accessible name.
- `examples/sidebar.rs`: a window that runs the three widths.

[Unreleased]: https://github.com/Magnetar-OS/cosmic-ext-widgets/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/Magnetar-OS/cosmic-ext-widgets/releases/tag/v1.0.0
