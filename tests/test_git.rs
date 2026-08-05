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

#[allow(dead_code)]
mod git;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use git_slides::git::{find_git_directory, history_up_to_commit};

static CURRENT_DIRECTORY_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_repositories_are_outside_project_repository() {
    let dir = git::init("test_repositories_are_outside_project_repository");

    // If concurrent setup temporarily removes a fixture's `.git`, Git
    // searches parent directories. Keep fixtures outside the project to
    // protect its repository.
    assert!(!dir.starts_with(env!("CARGO_MANIFEST_DIR")));
}

fn find_git_directory_from(dir: &Path) -> Option<PathBuf> {
    let _lock = CURRENT_DIRECTORY_LOCK.lock().unwrap();
    let initial_dir = env::current_dir().unwrap();

    env::set_current_dir(dir).unwrap();
    let git_dir = find_git_directory();
    env::set_current_dir(initial_dir).unwrap();

    git_dir
}

fn history_up_to_commit_from(dir: &Path, commit: &str) -> Vec<git_slides::git::Commit> {
    let _lock = CURRENT_DIRECTORY_LOCK.lock().unwrap();
    let initial_dir = env::current_dir().unwrap();

    env::set_current_dir(dir).unwrap();
    let history = history_up_to_commit(commit);
    env::set_current_dir(initial_dir).unwrap();

    history
}

#[test]
fn find_git_directory_rejects_bare_repository() {
    let dir = git::init_bare("find_git_directory_rejects_bare_repository");

    let git_dir = find_git_directory_from(&dir);

    git::remove_bare(&dir);

    assert!(git_dir.is_none());
}

#[test]
fn find_git_directory_is_absolute_from_nested_directory() {
    let dir = git::init("find_git_directory_is_absolute_from_nested_directory");
    let nested_dir = dir.join("nested");
    fs::create_dir(&nested_dir).unwrap();

    let git_dir = find_git_directory_from(&nested_dir).unwrap();

    assert!(git_dir.is_absolute());
    assert_eq!(git_dir, dir.join(".git"));
}

#[test]
fn history_up_to_invalid_commit_is_empty() {
    let dir = git::init("history_up_to_invalid_commit_is_empty");
    git::commit(&dir, "Slide 1");

    let history = history_up_to_commit_from(&dir, "invalid");

    assert!(history.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn find_git_directory_preserves_non_utf8_path() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;

    let mut name = format!(
        "find_git_directory_preserves_non_utf8_path_{}-",
        std::process::id()
    )
    .into_bytes();
    name.push(0xff);

    let dir = env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(OsString::from_vec(name));
    let dir = git::init_at(dir);
    let nested_dir = dir.join("nested");
    fs::create_dir(&nested_dir).unwrap();

    let git_dir = find_git_directory_from(&nested_dir);
    // GitHub Actions' cache glob cannot traverse this non-UTF-8 path.
    fs::remove_dir_all(&dir).unwrap();

    let git_dir = git_dir.unwrap();
    assert!(git_dir.is_absolute());
    assert_eq!(git_dir, dir.join(".git"));
}
