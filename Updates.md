## Critical / Functional Issues

- [x] **Token via GITHUB_TOKEN env var only** — Replaced with `keyring` crate (secret-service). Token is loaded from the system keyring in `init()` via a `Task::perform` + `spawn_blocking`. On first run, the popup shows a text-input setup UI to enter and save the token. `dotenv` removed from `main.rs` and `Cargo.toml`. Files changed: `src/app.rs`, `src/main.rs`, `Cargo.toml`.
- [x] **No periodic refresh subscription** — Added `cosmic::iced::time::every(Duration::from_secs(300))` in `subscription()` mapped to a new `Message::Tick`. The `Tick` handler triggers `RefreshNotifications` when a client is connected and not already loading. File changed: `src/app.rs`.
- [x] **Config struct is a stub** — Replaced `demo: String` with `show_all: bool` (default `false`) and `refresh_interval_secs: u64` (default `300`). Added `config_handler: Option<CosmicConfigHandler>` to `AppModel` so changes are persisted via `write_entry`. `ToggleShowAll` now saves immediately. `subscription()` uses `self.config.refresh_interval_secs` for the tick interval. Files changed: `src/config.rs`, `src/app.rs`.
- [x] **Hardcoded popup size** — Replaced fixed `1000×1000` with `max_width(500.0)` and `max_height(620.0)`, allowing the popup to shrink when there are fewer notifications. File changed: `src/app.rs`.
- [x] **single-instance feature in Cargo.toml** — Removed. Panel applets are managed by the compositor and must not enforce single-instance themselves. File changed: `Cargo.toml`.

## Minor / Polish Issues

- [x] **desktop entry missing X-CosmicApplet** — Added `X-CosmicApplet=true` to `resources/app.desktop` so the applet appears in the COSMIC panel applet picker. File changed: `resources/app.desktop`.
- [x] **dotenv dependency** — Removed `dotenv` from `Cargo.toml` and `src/main.rs`.
- [x] **Emoji in UI (🎉)** — Replaced with `widget::icon::from_name("emblem-ok-symbolic").size(32)` in the empty-state column. Removed emoji from `i18n/en/git_work.ftl`. Files changed: `src/app.rs`, `i18n/en/git_work.ftl`.
- [x] **Hardcoded `height(Length::Fixed(400.0))`** — Changed to `Length::Fill`; the popup's `max_height(620.0)` caps it. File changed: `src/app.rs`.
- [x] **No i18n usage** — All visible strings now use `fl!()` macros. Fixed `fl!` macro in `src/i18n.rs` to accept `$($args:tt)*` for key=value variable arguments. Added missing keys `reason-ci-activity`, `token-setup-title`, `token-setup-instructions`, `token-save` to `i18n/en/git_work.ftl`. Added `use crate::fl;` to `src/app.rs` and `src/github.rs`. `format_reason` and `format_time_ago` fully i18n-ized. Files changed: `src/i18n.rs`, `src/app.rs`, `src/github.rs`, `i18n/en/git_work.ftl`.
- [x] **view_window error branch** — GITHUB_TOKEN error replaced by a proper token setup UI (text input + Save button). General API errors still display inline. File changed: `src/app.rs`.