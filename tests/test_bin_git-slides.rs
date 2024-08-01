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

mod git;

use std::path::Path;
use std::process::Command;
use std::{env, fs};

const GIT_SLIDES: &str = env!("CARGO_BIN_EXE_git-slides");

struct Output {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn run(dir: &Path, args: &[&str]) -> Output {
    let mut output = Command::new(GIT_SLIDES);

    for arg in args {
        output.arg(arg);
    }

    let output = output.current_dir(dir).output().unwrap();

    Output {
        exit_code: output.status.code().unwrap(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

#[test]
fn help() {
    // Works outside of git repository.
    let output = run(&env::temp_dir(), &["--help"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
    assert!(output.stdout.contains("-v, --version"));
    assert!(output.stdout.contains("start [<ref>]"));
    assert!(output.stdout.contains("stop"));
    assert!(output.stdout.contains("next, n [<n>]"));
    assert!(output.stdout.contains("previous, p [<n>]"));
    assert!(output.stdout.contains("go <n>"));
    assert!(output.stdout.contains("status"));
    assert!(output.stdout.contains("list"));
}

#[test]
fn no_args_shows_help() {
    let dir = git::init("no_args_shows_help");

    let output = run(&dir, &[]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
}

#[test]
fn no_args_but_presentation_is_started_shows_status() {
    let dir = git::init("no_args_but_presentation_is_started_shows_status");
    git::commit(&dir, "Slide 1");

    run(&dir, &["start"]);

    let output = run(&dir, &[]);

    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 1");
    assert!(output.stdout.contains("* 1/1"));
}

#[test]
fn version() {
    // Works outside of git repository.
    let output = run(&env::temp_dir(), &["--version"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn git_not_in_path() {
    let output = Command::new(GIT_SLIDES).env("PATH", "").output().unwrap();

    assert_eq!(output.status.code().unwrap(), 1);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).to_string(),
        "fatal: Did not find git executable.\n"
    );
}

#[test]
fn not_a_git_directory() {
    let output = run(&env::temp_dir(), &[]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "fatal: Not a git repository (or any of the parent directories): .git\n"
    );
}

#[test]
fn start_regular() {
    let dir = git::init("start_regular");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    let store_file = dir.join(".git/git-slides");

    assert_eq!(git::status(&dir), "Slide 3");
    assert!(!store_file.is_file());

    let output = run(&dir, &["start"]);
    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.starts_with("Presentation started at"));

    assert_eq!(git::status(&dir), "Slide 1"); // Goes to first slide.
    assert!(store_file.is_file()); // Store file created.
}

#[test]
fn start_shows_status() {
    let dir = git::init("start_shows_status");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");

    let output = run(&dir, &["start"]);

    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 1");
    assert!(output.stdout.contains("* 1/2"));
}

#[test]
fn start_at_ref() {
    let dir = git::init("start_at_ref");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    assert_eq!(git::status(&dir), "Slide 3");

    let output = run(&dir, &["start", "HEAD~"]);

    // Presentation goes only up to slide 2.
    assert!(!output.stdout.contains("Slide 3"));
    assert!(output.stdout.contains("* 1/2"));
    assert!(output.stdout.contains("  2/2"));
}

#[test]
fn start_bad_ref() {
    let dir = git::init("start_bad_ref");

    // Will always work, as there are no commits in this repo.
    let output = run(&dir, &["start", "abcdefg"]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stderr, "error: Bad ref input: 'abcdefg'.\n");
}

#[test]
fn start_in_dirty_working_directory() {
    let dir = git::init("start_in_dirty_working_directory");
    git::commit(&dir, "Initial commit");

    let new_file = dir.join("hello.txt");

    let _ = fs::write(&new_file, ":)");
    git::add(&dir, &new_file);

    let output = run(&dir, &["start"]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: Working directory contains uncommitted changes.\n"
    );
}

#[test]
fn start_in_half_dirty_working_directory() {
    let dir = git::init("start_in_half_dirty_working_directory");
    git::commit(&dir, "Initial commit");

    let new_file = dir.join("hello.txt");

    let _ = fs::write(new_file, ":)");

    // This version doesn't 'git add' the file; it remains untracked.

    let output = run(&dir, &["start"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.starts_with("Presentation started at"));
}

#[test]
fn start_in_repo_without_commits() {
    let dir = git::init("start_in_repo_without_commits");

    let output = run(&dir, &["start"]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: No HEAD commit. Please provide a valid ref.\n"
    );
}

#[test]
fn all_methods_requiring_presentation_to_be_started() {
    let dir = git::init("all_methods_requiring_presentation_to_be_started");

    let output = run(&dir, &["stop"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stderr, "You need to start by 'git slides start'.\n");

    let output = run(&dir, &["next"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stderr, "You need to start by 'git slides start'.\n");

    let output = run(&dir, &["previous"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stderr, "You need to start by 'git slides start'.\n");

    let output = run(&dir, &["go", "1"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stderr, "You need to start by 'git slides start'.\n");

    let output = run(&dir, &["status"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stderr, "You need to start by 'git slides start'.\n");

    let output = run(&dir, &["list"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stderr, "You need to start by 'git slides start'.\n");
}

#[test]
fn next_regular() {
    let dir = git::init("next_regular");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    run(&dir, &["start"]);
    assert_eq!(git::status(&dir), "Slide 1");

    let output = run(&dir, &["next"]);
    assert_eq!(git::status(&dir), "Slide 2");
    assert_eq!(output.exit_code, 0);
    assert!(!output
        .stdout
        .contains("You've reached the end of the presentation.\n"));

    let output = run(&dir, &["next"]);
    assert_eq!(git::status(&dir), "Slide 3");
    assert_eq!(output.exit_code, 0);
    assert!(output
        .stdout
        .contains("You've reached the end of the presentation.\n"));
}

#[test]
fn next_shorthand() {
    let dir = git::init("next_shorthand");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");

    run(&dir, &["start"]);
    assert_eq!(git::status(&dir), "Slide 1");

    let output = run(&dir, &["n"]);
    assert_eq!(git::status(&dir), "Slide 2");
    assert_eq!(output.exit_code, 0);
}

#[test]
fn next_with_offset() {
    let dir = git::init("next_with_offset");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");
    git::commit(&dir, "Slide 4");

    run(&dir, &["start"]);
    assert_eq!(git::status(&dir), "Slide 1");

    let output = run(&dir, &["next", "2"]);
    assert_eq!(git::status(&dir), "Slide 3");
    assert_eq!(output.exit_code, 0);
    assert!(!output
        .stdout
        .contains("You've reached the end of the presentation.\n"));

    // Does not overflow.
    let output = run(&dir, &["next", "10"]);
    assert_eq!(git::status(&dir), "Slide 4");
    assert_eq!(output.exit_code, 0);
    assert!(output
        .stdout
        .contains("You've reached the end of the presentation.\n"));
}

#[test]
fn next_with_overlow() {
    let dir = git::init("next_with_overflow");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");
    git::commit(&dir, "Slide 4");

    run(&dir, &["start"]);
    assert_eq!(git::status(&dir), "Slide 1");

    run(&dir, &["next"]);
    run(&dir, &["next"]);
    run(&dir, &["next"]);
    let output = run(&dir, &["next"]);

    assert_eq!(git::status(&dir), "Slide 4");
    assert_eq!(output.exit_code, 0);
    assert!(output
        .stdout
        .contains("You've reached the end of the presentation.\n"));
}

#[test]
fn next_error_getting_current_commit() {
    let dir = git::init("next_error_getting_current_commit");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Not in presentation");

    run(&dir, &["start", "HEAD~"]);

    git::checkout(&dir, "main"); // Commit not in presentation.

    let output = run(&dir, &["next"]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: Current HEAD not part of presentation.\n"
    );
}

#[test]
fn previous_regular() {
    let dir = git::init("previous_regular");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    run(&dir, &["start"]);
    run(&dir, &["go", "3"]);
    assert_eq!(git::status(&dir), "Slide 3");

    let output = run(&dir, &["previous"]);
    assert_eq!(git::status(&dir), "Slide 2");
    assert_eq!(output.exit_code, 0);
    assert!(!output
        .stdout
        .contains("You're at the start of the presentation.\n"));

    let output = run(&dir, &["previous"]);
    assert_eq!(git::status(&dir), "Slide 1");
    assert_eq!(output.exit_code, 0);
    assert!(output
        .stdout
        .contains("You're at the start of the presentation.\n"));
}

#[test]
fn previous_shorthand() {
    let dir = git::init("previous_shorthand");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");

    run(&dir, &["start"]);
    run(&dir, &["go", "2"]);
    assert_eq!(git::status(&dir), "Slide 2");

    let output = run(&dir, &["p"]);
    assert_eq!(git::status(&dir), "Slide 1");
    assert_eq!(output.exit_code, 0);
}

#[test]
fn previous_with_offset() {
    let dir = git::init("previous_with_offset");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");
    git::commit(&dir, "Slide 4");

    run(&dir, &["start"]);
    run(&dir, &["go", "4"]);
    assert_eq!(git::status(&dir), "Slide 4");

    let output = run(&dir, &["previous", "2"]);
    assert_eq!(git::status(&dir), "Slide 2");
    assert_eq!(output.exit_code, 0);
    assert!(!output
        .stdout
        .contains("You're at the start of the presentation.\n"));

    // Does not overflow.
    let output = run(&dir, &["previous", "10"]);
    assert_eq!(git::status(&dir), "Slide 1");
    assert_eq!(output.exit_code, 0);
    assert!(output
        .stdout
        .contains("You're at the start of the presentation.\n"));
}

#[test]
fn previous_with_overlow() {
    let dir = git::init("previous_with_overflow");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");
    git::commit(&dir, "Slide 4");

    run(&dir, &["start"]);
    run(&dir, &["go", "4"]);
    assert_eq!(git::status(&dir), "Slide 4");

    run(&dir, &["previous"]);
    run(&dir, &["previous"]);
    run(&dir, &["previous"]);
    let output = run(&dir, &["previous"]);

    assert_eq!(git::status(&dir), "Slide 1");
    assert_eq!(output.exit_code, 0);
    assert!(output
        .stdout
        .contains("You're at the start of the presentation.\n"));
}

#[test]
fn previous_error_getting_current_commit() {
    let dir = git::init("previous_error_getting_current_commit");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Not in presentation");

    run(&dir, &["start", "HEAD~"]);

    git::checkout(&dir, "main"); // Commit not in presentation.

    let output = run(&dir, &["previous"]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: Current HEAD not part of presentation.\n"
    );
}

#[test]
fn stop_regular() {
    let dir = git::init("stop_regular");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    let store_file = dir.join(".git/git-slides");

    assert_eq!(git::status(&dir), "Slide 3");
    assert!(!store_file.is_file());

    run(&dir, &["start"]);
    assert_eq!(git::status(&dir), "Slide 1");
    assert!(store_file.is_file());

    let output = run(&dir, &["stop"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.starts_with("Presentation stopped.\n"));
    assert!(!store_file.is_file()); // Removed store file.

    // By default, switch back to initial branch.
    assert!(output.stdout.contains("Going back to branch 'main'.\n"));
    assert_eq!(git::status(&dir), "Slide 3");
}

#[test]
fn stop_started_from_detached() {
    let dir = git::init("stop_started_from_detached");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    git::checkout(&dir, "HEAD~");
    assert_eq!(git::status(&dir), "Slide 2"); // Start from slide 2.

    run(&dir, &["start"]);
    assert_eq!(git::status(&dir), "Slide 1");

    let output = run(&dir, &["stop"]);

    // Switch back to initial commit.
    assert!(output.stdout.contains("Going back to commit"));
    assert_eq!(git::status(&dir), "Slide 2"); // Back to slide 2.
}

#[test]
fn stop_in_dirty_working_directory() {
    let dir = git::init("stop_in_dirty_working_directory");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");

    run(&dir, &["start"]);

    let new_file = dir.join("hello.txt");

    let _ = fs::write(&new_file, ":)");
    git::add(&dir, &new_file);

    assert!(!git::has_stashed_changes(&dir));

    let output = run(&dir, &["stop"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("Stashed uncommitted changes."));

    assert!(git::has_stashed_changes(&dir));
}

#[test]
fn go_regular() {
    let dir = git::init("go_regular");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    let output = run(&dir, &["start"]);
    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 1");

    let output = run(&dir, &["go", "2"]);
    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 2");

    let output = run(&dir, &["go", "3"]);
    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 3");

    // Go backwards.
    let output = run(&dir, &["go", "1"]);
    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 1");
}

#[test]
fn go_to_current() {
    let dir = git::init("go_to_current");
    git::commit(&dir, "Slide 1");

    let output = run(&dir, &["start"]);
    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 1");

    // Go to current.
    let output = run(&dir, &["go", "1"]);
    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 1");
}

#[test]
fn go_shows_status() {
    let dir = git::init("go_shows_status");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");

    run(&dir, &["start"]);

    let output = run(&dir, &["go", "2"]);

    assert_eq!(output.exit_code, 0);
    assert_eq!(git::status(&dir), "Slide 2");
    assert!(output.stdout.contains("* 2/2"));
}

#[test]
fn go_in_dirty_working_directory() {
    let dir = git::init("go_in_dirty_working_directory");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");

    run(&dir, &["start"]);

    let new_file = dir.join("hello.txt");

    let _ = fs::write(&new_file, ":)");
    git::add(&dir, &new_file);

    assert!(!git::has_stashed_changes(&dir));

    let output = run(&dir, &["go", "2"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("Stashed uncommitted changes."));
    assert!(output.stdout.contains("* 2/2"));

    assert!(git::has_stashed_changes(&dir));
}

#[test]
fn go_no_index() {
    let dir = git::init("go_no_index");

    let output = run(&dir, &["go"]);

    assert_eq!(output.exit_code, 2);
    assert_eq!(output.stderr, "fatal: Need a slide number.\n");
}

#[test]
fn go_bad_index() {
    let dir = git::init("go_bad_index");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");

    run(&dir, &["start"]);

    let output = run(&dir, &["go", "0"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: Bad slide index. Slide 0 does not exist.\nPossible values range from 1 to 2.\n"
    );

    let output = run(&dir, &["go", "3"]);
    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: Bad slide index. Slide 3 does not exist.\nPossible values range from 1 to 2.\n"
    );
}

#[test]
fn status_full() {
    let dir = git::init("status_full");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    run(&dir, &["start"]);

    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("(Start)"));
    assert!(output.stdout.contains("* 1/3"));
    assert!(output.stdout.contains("2/3"));
    assert!(output.stdout.contains("3/3"));
    assert!(output.stdout.contains("(End)"));
}

#[test]
fn status_cut() {
    let dir = git::init("status_cut");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");
    git::commit(&dir, "Slide 4");
    git::commit(&dir, "Slide 5");
    git::commit(&dir, "Slide 6");
    git::commit(&dir, "Slide 7");

    run(&dir, &["start"]);

    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(output.stdout.contains("(Start)"));
    assert!(output.stdout.contains("* 1/7"));
    assert!(output.stdout.contains("2/7"));
    assert!(output.stdout.contains("3/7"));
    assert!(output.stdout.contains("4/7"));
    assert!(!output.stdout.contains("5/7"));

    run(&dir, &["go", "2"]);
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(output.stdout.contains("(Start)"));
    assert!(output.stdout.contains("1/7"));
    assert!(output.stdout.contains("* 2/7"));
    assert!(output.stdout.contains("3/7"));
    assert!(output.stdout.contains("4/7"));
    assert!(output.stdout.contains("5/7"));
    assert!(!output.stdout.contains("6/7"));

    run(&dir, &["go", "3"]);
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(!output.stdout.contains("(Start)"));
    assert!(output.stdout.contains("1/7"));
    assert!(output.stdout.contains("2/7"));
    assert!(output.stdout.contains("* 3/7"));
    assert!(output.stdout.contains("4/7"));
    assert!(output.stdout.contains("5/7"));
    assert!(output.stdout.contains("6/7"));
    assert!(!output.stdout.contains("7/7"));

    run(&dir, &["go", "4"]);
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(!output.stdout.contains("1/7"));
    assert!(output.stdout.contains("2/7"));
    assert!(output.stdout.contains("3/7"));
    assert!(output.stdout.contains("* 4/7"));
    assert!(output.stdout.contains("5/7"));
    assert!(output.stdout.contains("6/7"));
    assert!(output.stdout.contains("7/7"));
    assert!(!output.stdout.contains("(End)"));

    run(&dir, &["go", "5"]);
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(!output.stdout.contains("2/7"));
    assert!(output.stdout.contains("3/7"));
    assert!(output.stdout.contains("4/7"));
    assert!(output.stdout.contains("* 5/7"));
    assert!(output.stdout.contains("6/7"));
    assert!(output.stdout.contains("7/7"));
    assert!(output.stdout.contains("(End)"));

    run(&dir, &["go", "6"]);
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(!output.stdout.contains("3/7"));
    assert!(output.stdout.contains("4/7"));
    assert!(output.stdout.contains("5/7"));
    assert!(output.stdout.contains("* 6/7"));
    assert!(output.stdout.contains("7/7"));
    assert!(output.stdout.contains("(End)"));

    run(&dir, &["go", "7"]);
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(!output.stdout.contains("4/7"));
    assert!(output.stdout.contains("5/7"));
    assert!(output.stdout.contains("6/7"));
    assert!(output.stdout.contains("* 7/7"));
    assert!(output.stdout.contains("(End)"));
}

#[test]
fn status_number_padding() {
    let dir = git::init("status_number_padding");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");
    git::commit(&dir, "Slide 4");
    git::commit(&dir, "Slide 5");
    git::commit(&dir, "Slide 6");
    git::commit(&dir, "Slide 7");
    git::commit(&dir, "Slide 8");
    git::commit(&dir, "Slide 9");
    git::commit(&dir, "Slide 10");

    run(&dir, &["start"]);

    // Padded, even if bigger number not in output.
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(output.stdout.contains("*  1/10"));
    assert!(output.stdout.contains("   2/10"));
    assert!(!output.stdout.contains("  10/10"));

    run(&dir, &["go", "9"]);
    let output = run(&dir, &["status"]);
    println!("{}", output.stdout);
    assert!(!output.stdout.contains("6/10"));
    assert!(output.stdout.contains("*  9/10"));
    assert!(output.stdout.contains("  10/10"));
}

#[test]
fn status_error_getting_current_commit() {
    let dir = git::init("status_error_getting_current_commit");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Not in presentation");

    run(&dir, &["start", "HEAD~"]);

    git::checkout(&dir, "main"); // Commit not in presentation.

    let output = run(&dir, &["status"]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: Current HEAD not part of presentation.\n"
    );
}

#[test]
fn list() {
    let dir = git::init("list");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");

    run(&dir, &["start"]);

    let output = run(&dir, &["list"]);
    println!("{}", output.stdout);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("* 1/3"));
    assert!(output.stdout.contains("2/3"));
    assert!(output.stdout.contains("3/3"));
}

#[test]
fn list_number_padding() {
    let dir = git::init("list_number_padding");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Slide 2");
    git::commit(&dir, "Slide 3");
    git::commit(&dir, "Slide 4");
    git::commit(&dir, "Slide 5");
    git::commit(&dir, "Slide 6");
    git::commit(&dir, "Slide 7");
    git::commit(&dir, "Slide 8");
    git::commit(&dir, "Slide 9");
    git::commit(&dir, "Slide 10");

    run(&dir, &["start"]);

    let output = run(&dir, &["list"]);
    println!("{}", output.stdout);
    assert!(output.stdout.contains("*  1/10"));
    assert!(output.stdout.contains("  10/10"));
}

#[test]
fn list_error_getting_current_commit() {
    let dir = git::init("list_error_getting_current_commit");
    git::commit(&dir, "Slide 1");
    git::commit(&dir, "Not in presentation");

    run(&dir, &["start", "HEAD~"]);

    git::checkout(&dir, "main"); // Commit not in presentation.

    let output = run(&dir, &["list"]);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "error: Current HEAD not part of presentation.\n"
    );
}
