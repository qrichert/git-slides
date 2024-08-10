// git-slides — Navigate through Git commits like presentation slides.
// Copyright (C) 2024  Quentin Richert
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
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct Commit {
    pub hash: String,
    pub title: String,
}

#[must_use]
pub fn is_git_in_path() -> bool {
    Command::new("git")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

#[must_use]
pub fn find_git_directory() -> Option<PathBuf> {
    let mut current_dir = env::current_dir().ok()?;

    loop {
        let git_dir = current_dir.join(".git");
        if git_dir.is_dir() {
            return Some(git_dir);
        }
        if !current_dir.pop() {
            break;
        }
    }

    None
}

#[must_use]
pub fn current_commit_hash() -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("--quiet")
        .arg("HEAD^{commit}")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            return Some(hash);
        }
    }

    None
}

#[must_use]
pub fn current_branch() -> Option<String> {
    let output = Command::new("git")
        .arg("symbolic-ref")
        .arg("--short")
        .arg("--quiet")
        .arg("HEAD")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            return Some(branch);
        }
    }

    None
}

#[must_use]
pub fn ref_to_commit_hash(ref_: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("--quiet")
        .arg("--end-of-options")
        .arg(format!("{ref_}^{{commit}}"))
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            return Some(hash);
        }
    }

    None
}

#[cfg(not(tarpaulin_include))] // Does not ignore '(return) Vec::new()'.
#[must_use]
pub fn history_up_to_commit(commit: &str) -> Vec<Commit> {
    let output = Command::new("git")
        .arg("rev-list")
        .arg("--first-parent")
        .arg("--format=%H %s")
        .arg("--no-commit-header")
        .arg("--reverse")
        .arg(commit)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let commits: Vec<Commit> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let pieces = line.split_once(' ')?;
                    let hash = String::from(pieces.0);
                    let title = String::from(pieces.1);
                    Some(Commit { hash, title })
                })
                .collect();
            return commits;
        }
    }

    // Should never happen, because we always have at least one commit.
    Vec::new()
}

#[cfg(not(tarpaulin_include))] // Does not ignore 'return false'.
#[must_use]
pub fn checkout(commit: &str) -> bool {
    let status = Command::new("git")
        .arg("checkout")
        .arg(commit)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let Ok(status) = status else {
        return false;
    };

    status.success()
}

#[cfg(not(tarpaulin_include))] // Does not ignore 'return false'.
#[must_use]
pub fn is_working_directory_clean() -> bool {
    let output = Command::new("git")
        .arg("status")
        .arg("--untracked-files=no")
        .arg("--porcelain")
        .output();

    let Ok(output) = output else {
        return false;
    };

    String::from_utf8_lossy(&output.stdout).trim().is_empty()
}

#[cfg(not(tarpaulin_include))] // Does not ignore 'return false'.
pub fn stash() -> bool {
    let status = Command::new("git")
        .arg("stash")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let Ok(status) = status else {
        return false;
    };

    status.success()
}
