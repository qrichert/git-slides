// git-slides — Navigate through Git commits like presentation slides.
// Copyright (C) 2026  Quentin Richert
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

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
