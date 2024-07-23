# git-slides

[![license: GPL v3+](https://img.shields.io/badge/license-GPLv3+-blue)](https://www.gnu.org/licenses/gpl-3.0)
![GitHub Tag](https://img.shields.io/github/v/tag/qrichert/git-slides?sort=semver&filter=*.*.*&label=release)
[![crates.io](https://img.shields.io/crates/d/git-slides?logo=rust&logoColor=white&color=orange)](https://crates.io/crates/git-slides)

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

## Installation

### Directly

```console
$ wget https://github.com/qrichert/git-slides/releases/download/X.X.X/git-slides-X.X.X-xxx
$ sudo install ./git-slides /usr/local/bin/git-slides
```

### Manual Build

#### System-wide

```console
$ git clone https://github.com/qrichert/git-slides.git
$ cd git-slides
$ make build
$ sudo make install
```

#### Through Cargo

```shell
cargo install git-slides
cargo install --git https://github.com/qrichert/git-slides.git
```
