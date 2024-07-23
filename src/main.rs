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

mod cmd;
mod git;

use std::env;
use std::path::PathBuf;

use cmd::Cmd;

fn main() {
    let mut args = env::args().peekable();
    args.next();

    if let Some(arg) = args.peek() {
        match arg.as_str() {
            "-h" | "--help" => {
                args.next();
                help();
                return;
            }
            "-v" | "--version" => {
                args.next();
                version();
                return;
            }
            _ => (),
        }
    }

    ensure_git_executable_is_in_path();
    let git_dir = get_git_directory_or_exit();

    let cmd = Cmd::new(git_dir);

    if let Some(arg) = args.next() {
        return match arg.as_str() {
            "start" => {
                // `start` may be followed by `ref`.
                cmd.start(args.next());
            }
            "stop" => cmd.stop(),
            "next" | "n" => {
                // `next` may be followed by `n`.
                if let Some(n) = args.peek() {
                    if let Ok(n) = n.parse::<usize>() {
                        return cmd.next(n);
                    }
                }
                cmd.next(1);
            }
            "previous" | "p" => {
                // `previous` may be followed by `n`.
                if let Some(n) = args.peek() {
                    if let Ok(n) = n.parse::<usize>() {
                        return cmd.previous(n);
                    }
                }
                cmd.previous(1);
            }
            "go" => {
                // `go` must be followed by `n`.
                if let Some(n) = args.peek() {
                    if let Ok(n) = n.parse::<usize>() {
                        return cmd.go(n);
                    }
                }
                eprintln!("fatal: Need a slide number.");
                std::process::exit(2);
            }
            "status" => cmd.status(),
            "list" => cmd.list(),
            arg => {
                eprintln!("Unknown argument: '{arg}'.\n");
                help();
                std::process::exit(2);
            }
        };
    }

    help();
}

fn ensure_git_executable_is_in_path() {
    if !git::is_git_in_path() {
        eprintln!("fatal: Did not find git executable.");
        std::process::exit(1);
    }
}

fn get_git_directory_or_exit() -> PathBuf {
    let Some(git_dir) = git::find_git_directory() else {
        eprintln!("fatal: Not a git repository (or any of the parent directories): .git");
        std::process::exit(1);
    };
    git_dir
}

fn help() {
    println!(
        "\
usage: {bin} [<options>] <command> [<args>]

Commands:
  start [<ref>]        Start presentation.
  stop                 End presentation.
  next, n [<n>]        Go forward one or <n> slides.
  previous, p [<n>]    Go back one or <n> slides.
  go <n>               Go to slide <n>.
  status               Show current status.
  list                 List all slides.

Options:
  -h, --help           Show this message and exit.
  -v, --version        Show the version and exit.
",
        bin = env!("CARGO_BIN_NAME"),
    );
}

fn version() {
    println!("{} {}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
}
