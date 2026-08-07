use std::env;
use std::ffi::OsStr;
use std::io::{self, IsTerminal};
use std::sync::LazyLock;

#[allow(unreachable_code)]
static USE_COLOR: LazyLock<bool> = LazyLock::new(|| {
    #[cfg(test)]
    {
        return true;
    }
    let no_color = env::var_os("NO_COLOR");
    let is_terminal = io::stdout().is_terminal();
    is_color_enabled(no_color.as_deref(), is_terminal)
});

pub const RESET: &str = "\x1b[m";
pub const FAINT: &str = "\x1b[2m";
pub const YELLOW: &str = "\x1b[33m";

#[must_use]
pub fn maybe_color(color: &'static str) -> &'static str {
    if *USE_COLOR { color } else { "" }
}

fn is_color_enabled(no_color: Option<&OsStr>, stdout_is_terminal: bool) -> bool {
    stdout_is_terminal && no_color.is_none_or(OsStr::is_empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_requires_a_terminal_and_no_nonempty_no_color_value() {
        assert!(is_color_enabled(None, true));
        assert!(is_color_enabled(Some(OsStr::new("")), true));
        assert!(!is_color_enabled(Some(OsStr::new("1")), true));
        assert!(!is_color_enabled(None, false));
    }

    #[test]
    fn color_code_is_unchanged_when_color_is_enabled() {
        assert_eq!(maybe_color(YELLOW), YELLOW);
    }
}
