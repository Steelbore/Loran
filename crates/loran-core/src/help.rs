// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran help <tool>` — live `--help` capture engine.
//!
//! Per Spec §4.2 the engine:
//!
//! 1. Resolves the target binary via `$PATH` (`which` crate). User-supplied
//!    names that look like paths (contain `/` or `\`) are rejected outright
//!    so that `loran help ../etc/passwd` can never succeed (Spec §4.2 step 1).
//! 2. Spawns the resolved binary with `argv = [tool, flag]` — no shell, no
//!    interpolation. Subprocess environment carries `PAGER` / `MANPAGER`
//!    selected by the §4.2.1 cascade, with `LESS` cleared only when the
//!    Steelbore default chain (`bat -pp`, `moor`, or `cat`) fires.
//! 3. Enforces a 5-second wall-clock timeout via `wait-timeout`; SIGKILL on
//!    overrun.
//! 4. Retries `--help` → `-h` → `help` (sub-command) on non-zero exit /
//!    empty output, preferring the first non-empty result.
//! 5. Records the resolved pager command and the cascade step that won
//!    in [`HelpResult`] so the JSON envelope can surface them as
//!    `data.body.pager_command` / `data.body.pager_source`.

use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

use jiff::Timestamp;
use serde::Serialize;
use thiserror::Error;
use wait_timeout::ChildExt;

/// Default wall-clock timeout for the subprocess.
pub const HELP_TIMEOUT: Duration = Duration::from_secs(5);

/// Which flag form ultimately produced the captured output.
#[derive(Copy, Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HelpFlag {
    /// `<tool> --help`
    Help,
    /// `<tool> -h`
    H,
    /// `<tool> help`
    HelpSub,
}

impl HelpFlag {
    fn argv_token(self) -> &'static str {
        match self {
            Self::Help => "--help",
            Self::H => "-h",
            Self::HelpSub => "help",
        }
    }
}

/// Which step of the §4.2.1 cascade resolved the pager.
#[derive(Copy, Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PagerSource {
    /// `--pager <cmd>` provided on the CLI invocation.
    Flag,
    /// `$MANPAGER` environment variable.
    ManpagerEnv,
    /// `$PAGER` environment variable.
    PagerEnv,
    /// `bat -pp` (Steelbore default; `bat` was found on `$PATH`).
    Bat,
    /// `moor` (Steelbore-blessed pure-Rust pager; `moor` was found on `$PATH`).
    Moor,
    /// `cat` — final fallback.
    Cat,
}

/// Caller-supplied options for [`capture_help`].
#[derive(Debug, Default, Clone)]
pub struct HelpOpts {
    /// `--pager <cmd>` override. `Some(s)` with `s.is_empty()` is the
    /// "disable pagination" sentinel (cat-equivalent passthrough).
    pub pager: Option<String>,
    /// Set to `true` when the CLI saw the `--pager=loran` sentinel.
    /// Causes [`resolve_pager`] to skip the `MANPAGER` / `PAGER`
    /// environment steps and start the cascade at `bat -pp`.
    pub skip_user_env_pager: bool,
    /// Optional wall-clock timeout override. Defaults to [`HELP_TIMEOUT`].
    pub timeout: Option<Duration>,
}

/// Captured output of a successful `<tool> --help` invocation.
#[derive(Debug, Clone, Serialize)]
pub struct HelpResult {
    /// The raw captured text — stdout if non-empty, otherwise stderr.
    pub captured_text: String,
    /// Which flag form produced this output.
    pub flag_used: HelpFlag,
    /// When the capture occurred. ISO 8601 UTC with `Z` suffix.
    pub captured_at: Timestamp,
    /// Exit code of the subprocess.
    pub exit_code: i32,
    /// The resolved pager command (for `data.body.pager_command`).
    pub pager_command: String,
    /// Cascade step that won (for `data.body.pager_source`).
    pub pager_source: PagerSource,
}

#[derive(Debug, Error)]
pub enum HelpError {
    /// The tool name did not resolve to an executable on `$PATH`.
    #[error("binary not found on $PATH: `{0}`")]
    BinaryNotFound(String),
    /// The user-supplied name looked like a path (contained `/` or `\`).
    #[error("`{0}` looks like a path; only `$PATH` lookups are accepted")]
    PathLikeName(String),
    /// The subprocess exceeded its wall-clock timeout.
    #[error("`{tool} {flag}` exceeded the {seconds}s timeout")]
    Timeout {
        tool: String,
        flag: &'static str,
        seconds: u64,
    },
    /// Subprocess spawn / wait failed for some other reason.
    #[error("subprocess error invoking `{tool}`: {source}")]
    SpawnFailed {
        tool: String,
        #[source]
        source: io::Error,
    },
    /// Every flag variant (`--help`, `-h`, `help`) ran but none produced
    /// non-empty output with a zero exit.
    #[error("`{0}` produced no usable output for any of --help / -h / help")]
    AllFlagsFailed(String),
}

/// Capture `<tool> --help` (with retry / pager cascade / timeout) and
/// return a typed [`HelpResult`].
///
/// `tool` is the user-supplied tool name. Pass `--pager` flag content
/// via `opts.pager` and the `--pager=loran` sentinel via
/// `opts.skip_user_env_pager = true`.
///
/// Reads `$MANPAGER` / `$PAGER` / `$PATH` from the calling process's
/// environment when the cascade reaches those steps. For test isolation
/// see [`resolve_pager`], which is the pure resolution helper.
pub fn capture_help(tool: &str, opts: &HelpOpts) -> Result<HelpResult, HelpError> {
    if tool.contains('/') || tool.contains('\\') {
        return Err(HelpError::PathLikeName(tool.to_owned()));
    }

    let resolved_binary =
        which::which(tool).map_err(|_| HelpError::BinaryNotFound(tool.to_owned()))?;

    let (pager_command, pager_source) = resolve_pager(opts, &|key| std::env::var(key).ok());
    let clear_less = matches!(
        pager_source,
        PagerSource::Bat | PagerSource::Moor | PagerSource::Cat
    );

    let timeout = opts.timeout.unwrap_or(HELP_TIMEOUT);
    let captured_at = Timestamp::now();

    let mut last_error: Option<HelpError> = None;
    for flag in [HelpFlag::Help, HelpFlag::H, HelpFlag::HelpSub] {
        match run_one(
            &resolved_binary,
            tool,
            flag,
            &pager_command,
            clear_less,
            timeout,
        ) {
            Ok((stdout, stderr, exit_code)) => {
                let captured_text = if stdout.trim().is_empty() {
                    stderr
                } else {
                    stdout
                };
                if !captured_text.trim().is_empty() {
                    return Ok(HelpResult {
                        captured_text,
                        flag_used: flag,
                        captured_at,
                        exit_code,
                        pager_command,
                        pager_source,
                    });
                }
                // Non-empty contract violated; try the next flag.
            }
            Err(HelpError::Timeout { .. }) => {
                return Err(HelpError::Timeout {
                    tool: tool.to_owned(),
                    flag: flag.argv_token(),
                    seconds: timeout.as_secs(),
                });
            }
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| HelpError::AllFlagsFailed(tool.to_owned())))
}

/// Spawn `<binary> <flag>` with the configured pager env, wait up to
/// `timeout`, capture stdout / stderr / exit code.
fn run_one(
    binary: &std::path::Path,
    tool_label: &str,
    flag: HelpFlag,
    pager_command: &str,
    clear_less: bool,
    timeout: Duration,
) -> Result<(String, String, i32), HelpError> {
    let mut cmd = Command::new(binary);
    cmd.arg(flag.argv_token())
        .env("PAGER", pager_command)
        .env("MANPAGER", pager_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    if clear_less {
        cmd.env("LESS", "");
    }

    let mut child = cmd.spawn().map_err(|source| HelpError::SpawnFailed {
        tool: tool_label.to_owned(),
        source,
    })?;

    let Some(status) = child
        .wait_timeout(timeout)
        .map_err(|source| HelpError::SpawnFailed {
            tool: tool_label.to_owned(),
            source,
        })?
    else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(HelpError::Timeout {
            tool: tool_label.to_owned(),
            flag: flag.argv_token(),
            seconds: timeout.as_secs(),
        });
    };

    let stdout = read_pipe(child.stdout.as_mut());
    let stderr = read_pipe(child.stderr.as_mut());
    let exit_code = status.code().unwrap_or(-1);

    if exit_code != 0 && stdout.trim().is_empty() && stderr.trim().is_empty() {
        return Err(HelpError::SpawnFailed {
            tool: tool_label.to_owned(),
            source: io::Error::other(format!(
                "process exited with code {exit_code} and no output"
            )),
        });
    }

    Ok((stdout, stderr, exit_code))
}

fn read_pipe<R: io::Read>(reader: Option<&mut R>) -> String {
    let Some(r) = reader else {
        return String::new();
    };
    let mut buf = String::new();
    let _ = r.read_to_string(&mut buf);
    buf
}

/// Pure pager-cascade resolution. Read `env(key)` for environment
/// variables instead of calling `std::env::var` directly so tests can
/// drive every branch without mutating process state.
///
/// Returns `(pager_command, pager_source)`. The command is what's
/// written to `PAGER` and `MANPAGER` in the subprocess environment.
pub fn resolve_pager<F: Fn(&str) -> Option<String>>(
    opts: &HelpOpts,
    env: &F,
) -> (String, PagerSource) {
    // Step 1: explicit --pager flag.
    if let Some(p) = opts.pager.as_deref() {
        if p.is_empty() {
            // `--pager=""` disables pagination — equivalent to cat.
            return ("cat".to_owned(), PagerSource::Cat);
        }
        return (p.to_owned(), PagerSource::Flag);
    }

    // Steps 2-3: user environment (unless suppressed by --pager=loran).
    if !opts.skip_user_env_pager {
        if let Some(m) = env("MANPAGER").filter(|v| !v.is_empty()) {
            return (m, PagerSource::ManpagerEnv);
        }
        if let Some(p) = env("PAGER").filter(|v| !v.is_empty()) {
            return (p, PagerSource::PagerEnv);
        }
    }

    // Step 4: bat -pp if bat is on $PATH.
    if which::which("bat").is_ok() {
        return ("bat -pp".to_owned(), PagerSource::Bat);
    }
    // Step 5: moor if moor is on $PATH.
    if which::which("moor").is_ok() {
        return ("moor".to_owned(), PagerSource::Moor);
    }
    // Step 6: cat.
    ("cat".to_owned(), PagerSource::Cat)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{HelpOpts, PagerSource, resolve_pager};

    fn make_env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |key| map.get(key).cloned()
    }

    // ─── Pager-cascade resolution ─────────────────────────────────

    #[test]
    fn flag_override_wins_over_environment() {
        let opts = HelpOpts {
            pager: Some("less -R".to_owned()),
            ..Default::default()
        };
        let env = make_env(&[("PAGER", "more"), ("MANPAGER", "less")]);
        assert_eq!(
            resolve_pager(&opts, &env),
            ("less -R".to_owned(), PagerSource::Flag)
        );
    }

    #[test]
    fn flag_empty_string_disables_pagination_as_cat() {
        let opts = HelpOpts {
            pager: Some(String::new()),
            ..Default::default()
        };
        let env = make_env(&[("PAGER", "less")]);
        assert_eq!(
            resolve_pager(&opts, &env),
            ("cat".to_owned(), PagerSource::Cat)
        );
    }

    #[test]
    fn manpager_wins_over_pager() {
        let env = make_env(&[("MANPAGER", "less -RFX"), ("PAGER", "more")]);
        assert_eq!(
            resolve_pager(&HelpOpts::default(), &env),
            ("less -RFX".to_owned(), PagerSource::ManpagerEnv)
        );
    }

    #[test]
    fn pager_used_when_manpager_absent() {
        let env = make_env(&[("PAGER", "less")]);
        assert_eq!(
            resolve_pager(&HelpOpts::default(), &env),
            ("less".to_owned(), PagerSource::PagerEnv)
        );
    }

    #[test]
    fn empty_environment_variables_do_not_count_as_set() {
        let env = make_env(&[("PAGER", ""), ("MANPAGER", "")]);
        // Falls through to bat/moor/cat depending on the host.
        let (_, source) = resolve_pager(&HelpOpts::default(), &env);
        assert!(
            matches!(
                source,
                PagerSource::Bat | PagerSource::Moor | PagerSource::Cat
            ),
            "got {source:?}"
        );
    }

    #[test]
    fn skip_user_env_pager_bypasses_environment() {
        let opts = HelpOpts {
            skip_user_env_pager: true,
            ..Default::default()
        };
        let env = make_env(&[("MANPAGER", "less"), ("PAGER", "more")]);
        let (_, source) = resolve_pager(&opts, &env);
        assert!(
            matches!(
                source,
                PagerSource::Bat | PagerSource::Moor | PagerSource::Cat
            ),
            "must bypass user env when --pager=loran sentinel set; got {source:?}"
        );
    }

    #[test]
    fn cascade_falls_through_to_default_chain_when_environment_empty() {
        let env = make_env(&[]);
        let (cmd, source) = resolve_pager(&HelpOpts::default(), &env);
        // We don't pin the source because it depends on whether bat /
        // moor exist on the test host, but cmd must be one of the three
        // documented defaults.
        assert!(
            ["bat -pp", "moor", "cat"].contains(&cmd.as_str()),
            "unexpected default-chain command: {cmd}"
        );
        assert!(
            matches!(
                source,
                PagerSource::Bat | PagerSource::Moor | PagerSource::Cat
            ),
            "source {source:?} must be one of the default-chain variants"
        );
    }

    // ─── Subprocess path-traversal rejection ───────────────────────

    #[test]
    fn path_traversal_rejected() {
        let err = super::capture_help("../etc/passwd", &HelpOpts::default()).unwrap_err();
        assert!(
            matches!(err, super::HelpError::PathLikeName(_)),
            "got {err:?}"
        );

        let err = super::capture_help("/bin/ls", &HelpOpts::default()).unwrap_err();
        assert!(
            matches!(err, super::HelpError::PathLikeName(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn unresolvable_tool_returns_binary_not_found() {
        let err = super::capture_help("definitely-not-a-real-binary-zzz", &HelpOpts::default())
            .unwrap_err();
        assert!(
            matches!(err, super::HelpError::BinaryNotFound(_)),
            "got {err:?}"
        );
    }
}
