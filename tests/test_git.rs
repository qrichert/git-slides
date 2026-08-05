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

use git_slides::git::find_git_directory;

static CURRENT_DIRECTORY_LOCK: Mutex<()> = Mutex::new(());

fn find_git_directory_from(dir: &Path) -> Option<PathBuf> {
    let _lock = CURRENT_DIRECTORY_LOCK.lock().unwrap();
    let initial_dir = env::current_dir().unwrap();

    env::set_current_dir(dir).unwrap();
    let git_dir = find_git_directory();
    env::set_current_dir(initial_dir).unwrap();

    git_dir
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

#[cfg(unix)]
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

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(OsString::from_vec(name));
    let dir = git::init_at(dir);
    let nested_dir = dir.join("nested");
    fs::create_dir(&nested_dir).unwrap();

    let git_dir = find_git_directory_from(&nested_dir).unwrap();

    assert!(git_dir.is_absolute());
    assert_eq!(git_dir, dir.join(".git"));
}
