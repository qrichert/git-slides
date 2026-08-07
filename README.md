# git-slides

![Crates.io License](https://img.shields.io/crates/l/git-slides)
![GitHub Tag](https://img.shields.io/github/v/tag/qrichert/git-slides?sort=semver&filter=*.*.*&label=release)
[![crates.io](https://img.shields.io/crates/d/git-slides?logo=rust&logoColor=white&color=orange)](https://crates.io/crates/git-slides)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/qrichert/git-slides/ci.yml?label=tests)](https://github.com/qrichert/git-slides/actions)

_Navigate through Git commits like presentation slides._

```console
$ git slides next
  1/7 7171da7 Introduction to Version Control: Git Basics
  2/7 ebde0ee Essential Git Commands: A Practical Overview
* 3/7 813f075 Branching Strategies: Enhancing Workflow Efficiency
  4/7 865c830 Collaboration with Git: Merging and Conflict Resolution
  5/7 ebe0dc2 Git Workflows: Centralized vs. Distributed Models
  6/7 9202f1e Advanced Git Features: Stashing, Rebasing, and Tagging
```

## Usage

The executable must be on your `PATH`, then you can use it as a regular
Git command:

```console
$ git slides start feat/my-presentation
```

```
usage: git-slides [<options>] <command> [<args>]

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
```

## Installation

Install from [crates.io] with Cargo:

```shell
cargo install git-slides
```

Pre-built binaries for Linux and macOS are available on the [latest
GitHub release].

[Documentation] is available on docs.rs.

[crates.io]: https://crates.io/crates/git-slides
[latest GitHub release]:
  https://github.com/qrichert/git-slides/releases/latest
[Documentation]: https://docs.rs/git-slides

## Acknowledgements

_2026-08-05_: Stumbled upon [gelisam/git-slides], almost exactly two
years after starting this project. API looks surprisingly similar, which
is a good sign for us I think. This project has nice edit features we
don't have and I'm very tempted to steal those ideas for a future
release.

<!-- Edit: Stolen. -->

[gelisam/git-slides]: https://github.com/gelisam/git-slides
