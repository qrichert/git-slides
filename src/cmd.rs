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

mod output;

use std::cell::OnceCell;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::{cmp, fs};

use super::git::{self, Commit};

const STORE_FILE: &str = env!("CARGO_BIN_NAME");

const COLOR_RESET: &str = "\x1b[m";
const COLOR_FAINT: &str = "\x1b[2m";
const COLOR_YELLOW: &str = "\x1b[33m";

pub struct Cmd {
    git_dir: PathBuf,
    history: OnceCell<Vec<Commit>>,
}

impl Cmd {
    pub fn new(git_dir: PathBuf) -> Self {
        Self {
            git_dir,
            history: OnceCell::new(),
        }
    }

    pub fn start(&self, ref_: Option<String>) {
        if !git::is_working_directory_clean() {
            eprintln!("error: Working directory contains uncommitted changes.");
            std::process::exit(1);
        }

        let commit_hash = if let Some(ref_) = ref_ {
            git::ref_to_commit_hash(&ref_).unwrap_or_else(|| {
                eprintln!("error: Bad ref input: '{ref_}'.");
                std::process::exit(1);
            })
        } else {
            git::current_commit_hash().unwrap_or_else(|| {
                eprintln!("error: No HEAD commit. Please provide a valid ref.");
                std::process::exit(1);
            })
        };

        let branch_name = git::current_branch().unwrap_or_default();

        let store_file = self.store_file();
        #[cfg(not(tarpaulin_include))]
        {
            if fs::write(store_file, format!("{branch_name}:{commit_hash}\n")).is_err() {
                eprintln!("error: Cannot write '.git/{STORE_FILE}'. Aborting.");
                std::process::exit(1);
            }
        }

        println!("Presentation started at {commit_hash}.");

        self.go(1);
    }

    pub fn stop(&self) {
        self.ensure_presentation_is_started();

        Self::stash_uncommitted_changes();

        println!("Presentation stopped.");

        if let Some(initial_branch) = self.get_initial_branch() {
            println!("Going back to branch '{initial_branch}'.");
            let _ = git::checkout(&initial_branch);
        } else {
            // The user was likely in detached mode when the presentation started.
            let head_commit = self.get_presentation_head_commit_hash();
            println!("Going back to commit {head_commit}.");
            let _ = git::checkout(&head_commit);
        }

        let store_file = self.store_file();
        #[cfg(not(tarpaulin_include))]
        {
            if fs::remove_file(store_file).is_err() {
                eprintln!("error: Cannot remove '.git/{STORE_FILE}'. Aborting.");
                std::process::exit(1);
            }
        }
    }

    pub fn next(&self, offset: usize) {
        self.ensure_presentation_is_started();

        let commits = self.get_history();
        let n = self.get_index_of_current_commit();

        let n = n + 1 + offset;

        if n >= commits.len() {
            println!("You've reached the end of the presentation.");
        }

        self.go(cmp::min(n, commits.len()));
    }

    pub fn previous(&self, offset: usize) {
        self.ensure_presentation_is_started();

        let n = self.get_index_of_current_commit();

        let n = (n + 1).saturating_sub(offset);

        if n <= 1 {
            println!("You're at the start of the presentation.");
        }

        self.go(cmp::max(n, 1));
    }

    pub fn go(&self, n: usize) {
        self.ensure_presentation_is_started();

        let commits = self.get_commits_hashes();

        if n < 1 || n > commits.len() {
            eprintln!("error: Bad slide index. Slide {n} does not exist.");
            eprintln!("Possible values range from 1 to {}.", commits.len());
            std::process::exit(1);
        }

        let go_to = commits.get(n - 1).expect("bounds checked");

        Self::stash_uncommitted_changes();

        if !git::checkout(go_to) {
            eprintln!("error: Could not checkout {go_to}.");
            std::process::exit(1);
        }

        self.status();
    }

    pub fn status(&self) {
        const SHOW_N_PREVIOUS: usize = 2;
        const SHOW_N_NEXT: usize = 3;

        self.ensure_presentation_is_started();

        let history = self.get_history();
        let n = self.get_index_of_current_commit();

        let display_from = n.saturating_sub(SHOW_N_PREVIOUS);
        let display_to = std::cmp::min(n + SHOW_N_NEXT, history.len() - 1);

        let slide_number_padding = history.len().to_string().len();

        // Acquire the lock once (instead of on every call to `print!`).
        let mut stdout = io::stdout().lock();

        if n.checked_sub(SHOW_N_PREVIOUS).is_none() {
            let _ = writeln!(stdout, "  {COLOR_FAINT}(Start){COLOR_RESET}");
        }

        for i in display_from..=display_to {
            let Commit { hash, title } = history.get(i).expect("bounds have been checked");

            if i == n {
                let _ = write!(stdout, "* ");
            } else {
                let _ = write!(stdout, "  ");
            }

            if i < n {
                let _ = writeln!(
                    stdout,
                    "{COLOR_FAINT}{:>slide_number_padding$}/{} {} {title}{COLOR_RESET}",
                    i + 1,
                    history.len(),
                    &hash[..7],
                );
            } else {
                let _ = writeln!(
                    stdout,
                    "{:>slide_number_padding$}/{} {COLOR_YELLOW}{}{COLOR_RESET} {title}",
                    i + 1,
                    history.len(),
                    &hash[..7],
                );
            }
        }

        if n + SHOW_N_NEXT > history.len() - 1 {
            let _ = writeln!(stdout, "  {COLOR_FAINT}(End){COLOR_RESET}");
        }
    }

    pub fn list(&self) {
        self.ensure_presentation_is_started();

        let history = self.get_history();
        let n = self.get_index_of_current_commit();

        let slide_number_padding = history.len().to_string().len();

        // Pre-allocate a "best-guess" number of characters. Each line
        // includes padding, slide number, commit hash, title and color.
        let mut out = String::with_capacity(history.len() * 72);

        for i in 0..history.len() {
            let Commit { hash, title } = history.get(i).expect("bounds have been checked");

            if i == n {
                let _ = write!(out, "* ");
            } else {
                let _ = write!(out, "  ");
            }

            let _ = writeln!(
                out,
                "{:>slide_number_padding$}/{} {COLOR_YELLOW}{}{COLOR_RESET} {title}",
                i + 1,
                history.len(),
                &hash[..7],
            );
        }

        output::Pager::page_or_print(&out);
    }

    fn ensure_presentation_is_started(&self) {
        if !self.is_presentation_started() {
            eprintln!(
                "You need to start by '{} start'.",
                env!("CARGO_BIN_NAME").replacen('-', " ", 1)
            );
            std::process::exit(1);
        }
    }

    pub fn is_presentation_started(&self) -> bool {
        let store_file = self.store_file();
        store_file.is_file()
    }

    #[cfg(not(tarpaulin_include))] // Does not ignore 'else eprintln()'.
    fn stash_uncommitted_changes() {
        if !git::is_working_directory_clean() {
            if git::stash() {
                println!("Stashed uncommitted changes.");
            } else {
                eprintln!("error: Could not stash uncommitted changes.");
            }
        }
    }

    fn get_commits_hashes(&self) -> Vec<&String> {
        let history = self.get_history();
        history.iter().map(|x| &x.hash).collect()
    }

    fn get_history(&self) -> &Vec<Commit> {
        // This function is expensive, and is called multiple times.
        // Calling it multiple times simplifies the API a lot, so we
        // cache the result instead of changing the API.
        self.history.get_or_init(|| {
            let hash = self.get_presentation_head_commit_hash();
            git::history_up_to_commit(&hash)
        })
    }

    fn get_presentation_head_commit_hash(&self) -> String {
        // <branch name>:<commit hash>
        self.read_store_file()
            .trim()
            .split_once(':')
            .expect("':' is always inserted during 'start'")
            .1
            .to_string()
    }

    fn get_initial_branch(&self) -> Option<String> {
        // <branch name>:<commit hash>
        let store = self.read_store_file();
        let branch = store
            .trim()
            .split_once(':')
            .expect("':' is always inserted during 'start'")
            .0;

        if branch.is_empty() {
            None
        } else {
            Some(branch.to_string())
        }
    }

    #[cfg(not(tarpaulin_include))]
    fn read_store_file(&self) -> String {
        let store_file = self.store_file();
        let Ok(store) = fs::read_to_string(store_file) else {
            eprintln!("error: Cannot read '.git/{STORE_FILE}'. Aborting.");
            std::process::exit(1);
        };
        store
    }

    fn store_file(&self) -> PathBuf {
        self.git_dir.join(STORE_FILE)
    }

    fn get_index_of_current_commit(&self) -> usize {
        let Some(commit) = self.get_index_of_current_commit_checked() else {
            eprintln!("error: Current HEAD not part of presentation.");
            std::process::exit(1);
        };
        commit
    }

    // May return `None` if user checked out to non-presentation commit,
    // or deleted commits.
    fn get_index_of_current_commit_checked(&self) -> Option<usize> {
        let hash = git::current_commit_hash()?;

        let hashes = self.get_commits_hashes();
        hashes.into_iter().position(|x| *x == hash)
    }
}
