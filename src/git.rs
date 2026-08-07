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
    let mut output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-bare-repository")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    strip_git_line_ending(&mut output.stdout);
    if output.stdout != b"false" {
        return None;
    }

    let mut output = Command::new("git")
        .arg("rev-parse")
        .arg("--absolute-git-dir")
        .output()
        .ok()?;

    #[cfg(not(tarpaulin_include))] // Cannot fail after Git identified a non-bare repository.
    {
        if !output.status.success() {
            return None;
        }
    }

    strip_git_line_ending(&mut output.stdout);
    #[cfg(not(tarpaulin_include))] // Git always prints the absolute directory on success.
    {
        if output.stdout.is_empty() {
            return None;
        }
    }

    #[cfg(unix)]
    let git_dir = {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt as _;

        OsString::from_vec(output.stdout)
    };

    #[cfg(not(unix))]
    let git_dir = String::from_utf8(output.stdout).ok()?;

    Some(PathBuf::from(git_dir))
}

/// Remove line ending from `git rev-parse` output.
fn strip_git_line_ending(output: &mut Vec<u8>) {
    if output.last() == Some(&b'\n') {
        _ = output.pop();
    }

    #[cfg(windows)]
    if output.last() == Some(&b'\r') {
        _ = output.pop();
    }
}

#[must_use]
pub fn current_commit_hash() -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("--quiet")
        .arg("HEAD^{commit}")
        .output();

    if let Ok(output) = output
        && output.status.success()
    {
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Some(hash);
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

    if let Ok(output) = output
        && output.status.success()
    {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Some(branch);
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

    if let Ok(output) = output
        && output.status.success()
    {
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Some(hash);
    }

    None
}

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

    if let Ok(output) = output
        && output.status.success()
    {
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

    // Should never happen, because we always have at least one commit.
    Vec::new()
}

#[must_use]
pub fn checkout(commit: &str) -> bool {
    let status = Command::new("git")
        .arg("checkout")
        .arg(commit)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    status.is_ok_and(|status| status.success())
}

#[must_use]
pub fn is_working_directory_clean() -> bool {
    let output = Command::new("git")
        .arg("status")
        .arg("--untracked-files=no")
        .arg("--porcelain")
        .output();

    output.is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

#[must_use]
pub fn stash() -> bool {
    let status = Command::new("git")
        .arg("stash")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    status.is_ok_and(|status| status.success())
}
