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

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

const TMP_DIR: &str = env!("CARGO_TARGET_TMPDIR");

pub fn init(dir: &str) -> PathBuf {
    let dir = PathBuf::from(TMP_DIR).join(dir);
    println!("git init: '{}'.", dir.display());
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir(&dir).unwrap();

    Command::new("git")
        .arg("init")
        .arg("--initial-branch=main")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    Command::new("git")
        .arg("config")
        .arg("--local")
        .arg("user.name")
        .arg("Git Slides")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    Command::new("git")
        .arg("config")
        .arg("--local")
        .arg("user.email")
        .arg("git@slides")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    dir
}

pub fn commit(dir: &Path, message: &str) {
    Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("--message")
        .arg(message)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
}

pub fn status(dir: &Path) -> String {
    // Returns the commit title.
    let output = Command::new("git")
        .arg("rev-list")
        .arg("--max-count=1")
        .arg("--no-commit-header")
        .arg("--format=%s")
        .arg("HEAD")
        .current_dir(dir)
        .output()
        .unwrap();

    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub fn checkout(dir: &Path, ref_: &str) {
    Command::new("git")
        .arg("checkout")
        .arg(ref_)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
}

pub fn add(dir: &Path, file: &Path) {
    Command::new("git")
        .arg("add")
        .arg(file)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
}

pub fn has_stashed_changes(dir: &Path) -> bool {
    let output = Command::new("git")
        .arg("stash")
        .arg("list")
        .current_dir(dir)
        .output()
        .unwrap();

    !String::from_utf8_lossy(&output.stdout).trim().is_empty()
}
