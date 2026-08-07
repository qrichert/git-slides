use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

pub fn init(dir: &str) -> PathBuf {
    let dir = env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("git-slides-{dir}-{}", process::id()));
    init_at(dir)
}

pub fn init_at(dir: PathBuf) -> PathBuf {
    println!("git init: '{}'.", dir.display());
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir(&dir).unwrap();

    let status = Command::new("git")
        .arg("init")
        .arg("--initial-branch=main")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("git")
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
    assert!(status.success());

    let status = Command::new("git")
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
    assert!(status.success());

    dir
}

pub fn init_bare(name: &str) -> PathBuf {
    let source = init(&format!("{name}_source"));
    commit(&source, "Slide 1");

    let dir = env::temp_dir().join(format!("git-slides-{name}-{}-bare", process::id()));
    let parent = dir.parent().unwrap();
    assert!(
        parent
            .ancestors()
            .all(|ancestor| !ancestor.join(".git").exists())
    );

    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }

    let status = Command::new("git")
        .arg("clone")
        .arg("--quiet")
        .arg("--bare")
        .arg(source)
        .arg(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert!(status.success());

    dir
}

pub fn remove_bare(dir: &Path) {
    fs::remove_dir_all(dir).unwrap();
}

pub fn add_worktree(dir: &Path, name: &str) -> PathBuf {
    let worktree = dir.join(name);
    let status = Command::new("git")
        .arg("worktree")
        .arg("add")
        .arg("--quiet")
        .arg("-b")
        .arg(name)
        .arg(&worktree)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert!(status.success());

    worktree
}

pub fn remove_worktree(dir: &Path, worktree: &Path) {
    let status = Command::new("git")
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(worktree)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert!(status.success());
}

pub fn directory(dir: &Path) -> PathBuf {
    let mut output = Command::new("git")
        .arg("rev-parse")
        .arg("--absolute-git-dir")
        .current_dir(dir)
        .output()
        .unwrap();

    assert!(output.status.success());

    if output.stdout.last() == Some(&b'\n') {
        _ = output.stdout.pop();
    }

    #[cfg(windows)]
    if output.stdout.last() == Some(&b'\r') {
        _ = output.stdout.pop();
    }

    assert!(!output.stdout.is_empty());

    #[cfg(unix)]
    let git_dir = {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt as _;

        OsString::from_vec(output.stdout)
    };

    #[cfg(not(unix))]
    let git_dir = String::from_utf8(output.stdout).unwrap();

    PathBuf::from(git_dir)
}

pub fn commit(dir: &Path, message: &str) {
    let status = Command::new("git")
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

    assert!(status.success());
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
    let status = Command::new("git")
        .arg("checkout")
        .arg(ref_)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert!(status.success());
}

pub fn add(dir: &Path, file: &Path) {
    let status = Command::new("git")
        .arg("add")
        .arg(file)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert!(status.success());
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
