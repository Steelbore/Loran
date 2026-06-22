// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! User configuration loaded from `$XDG_CONFIG_HOME/loran/config.toml`.
//!
//! Only the `[update]` section exists today. Catalog auto-update is
//! **opt-in and off by default** — Loran never reaches the network on a
//! read verb unless the user explicitly enables it here (or via the
//! `LORAN_AUTO_UPDATE` env var), keeping the no-surprise-network posture
//! the workspace is built around.
//!
//! ```toml
//! [update]
//! auto_update = true
//! auto_update_interval = "30d"
//! ```
//!
//! Environment overrides (highest precedence, useful for scripting and
//! hermetic tests):
//!
//! - `LORAN_AUTO_UPDATE` — `1`/`true`/`yes`/`on` enable, anything else
//!   disables.
//! - `LORAN_AUTO_UPDATE_INTERVAL` — duration string (e.g. `7d`, `12h`).

use std::time::Duration;

use serde::Deserialize;

/// Default auto-update interval when enabled without an explicit value.
///
/// 30 days mirrors the tldr-pages client default cache age — long
/// enough that the network hit is rare, short enough that a daily-driven
/// catalog never drifts far.
const DEFAULT_AUTO_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Resolved Loran configuration.
#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub update: UpdateConfig,
}

/// Resolved `[update]` settings.
#[derive(Debug, Clone)]
pub(crate) struct UpdateConfig {
    /// Whether read verbs may refresh a stale catalog over the network.
    pub auto_update: bool,
    /// How old the cached catalog must be before an auto-refresh fires.
    pub interval: Duration,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_update: false,
            interval: DEFAULT_AUTO_UPDATE_INTERVAL,
        }
    }
}

/// On-disk form. Every field optional so a partial file is valid.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    update: RawUpdate,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUpdate {
    auto_update: Option<bool>,
    auto_update_interval: Option<String>,
}

impl Config {
    /// Load `config.toml`, then apply environment overrides.
    ///
    /// Infallible by design: a missing file yields defaults, and a
    /// malformed file is logged at `warn` and falls back to defaults
    /// rather than bricking every read verb over a stray typo.
    #[must_use]
    pub(crate) fn load() -> Self {
        let raw = read_raw_config();
        let env_auto = env_bool("LORAN_AUTO_UPDATE");
        let env_interval = std::env::var_os("LORAN_AUTO_UPDATE_INTERVAL")
            .and_then(|v| v.to_str().and_then(parse_interval));
        Self::resolve(&raw, env_auto, env_interval)
    }

    /// Pure resolution: file values overlaid by environment overrides.
    /// Kept free of `std::env` reads so it is unit-testable without
    /// touching process state.
    fn resolve(raw: &RawConfig, env_auto: Option<bool>, env_interval: Option<Duration>) -> Self {
        let auto_update = env_auto.or(raw.update.auto_update).unwrap_or(false);
        let interval = env_interval
            .or_else(|| {
                raw.update
                    .auto_update_interval
                    .as_deref()
                    .and_then(parse_interval)
            })
            .unwrap_or(DEFAULT_AUTO_UPDATE_INTERVAL);
        Self {
            update: UpdateConfig {
                auto_update,
                interval,
            },
        }
    }
}

/// Read and parse `$XDG_CONFIG_HOME/loran/config.toml`, tolerating
/// absence and malformed content (logged, not fatal).
fn read_raw_config() -> RawConfig {
    let Some(dir) = loran_core::config_home() else {
        return RawConfig::default();
    };
    let path = dir.join("loran").join("config.toml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return RawConfig::default(),
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "could not read config; using defaults");
            return RawConfig::default();
        }
    };
    match toml::from_str(&text) {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "config is not valid TOML; using defaults");
            RawConfig::default()
        }
    }
}

/// Parse a truthy/falsey environment variable. Returns `None` when the
/// variable is unset so callers can fall back to the file value.
fn env_bool(key: &str) -> Option<bool> {
    let raw = std::env::var(key).ok()?;
    Some(matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    ))
}

/// Parse a small human duration: an integer with an optional unit
/// suffix (`s`, `m`, `h`, `d`, `w`). A bare integer is seconds. Returns
/// `None` for unparseable or zero-length input.
fn parse_interval(raw: &str) -> Option<Duration> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let (digits, unit_secs) = match s.as_bytes().last()? {
        b's' => (&s[..s.len() - 1], 1u64),
        b'm' => (&s[..s.len() - 1], 60),
        b'h' => (&s[..s.len() - 1], 60 * 60),
        b'd' => (&s[..s.len() - 1], 24 * 60 * 60),
        b'w' => (&s[..s.len() - 1], 7 * 24 * 60 * 60),
        b'0'..=b'9' => (s, 1),
        _ => return None,
    };
    let value: u64 = digits.trim().parse().ok()?;
    value.checked_mul(unit_secs).map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::{Config, RawConfig, RawUpdate, parse_interval};
    use std::time::Duration;

    #[test]
    fn parse_interval_handles_units_and_bare_seconds() {
        assert_eq!(parse_interval("30d"), Some(Duration::from_secs(2_592_000)));
        assert_eq!(parse_interval("12h"), Some(Duration::from_secs(43_200)));
        assert_eq!(parse_interval("90"), Some(Duration::from_secs(90)));
        assert_eq!(parse_interval("2w"), Some(Duration::from_secs(1_209_600)));
        assert_eq!(parse_interval(""), None);
        assert_eq!(parse_interval("nonsense"), None);
    }

    #[test]
    fn defaults_to_disabled_auto_update() {
        let cfg = Config::resolve(&RawConfig::default(), None, None);
        assert!(!cfg.update.auto_update);
        assert_eq!(cfg.update.interval, Duration::from_secs(2_592_000));
    }

    #[test]
    fn file_values_are_honoured() {
        let raw = RawConfig {
            update: RawUpdate {
                auto_update: Some(true),
                auto_update_interval: Some("7d".to_owned()),
            },
        };
        let cfg = Config::resolve(&raw, None, None);
        assert!(cfg.update.auto_update);
        assert_eq!(cfg.update.interval, Duration::from_secs(604_800));
    }

    #[test]
    fn env_overrides_win_over_file() {
        let raw = RawConfig {
            update: RawUpdate {
                auto_update: Some(true),
                auto_update_interval: Some("7d".to_owned()),
            },
        };
        let cfg = Config::resolve(&raw, Some(false), Some(Duration::from_secs(60)));
        assert!(!cfg.update.auto_update, "env disable overrides file enable");
        assert_eq!(cfg.update.interval, Duration::from_secs(60));
    }
}
