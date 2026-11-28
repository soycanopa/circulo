//! Git for the core: conversation-aware checkpoints and, later, worktrees.
//!
//! Only the core uses this crate. A checkpoint is a commit of the whole worktree
//! — tracked and untracked files, ignoring what `.gitignore` ignores — stored
//! under a private ref, so the user's branch, index and working tree are never
//! touched while capturing. Rewinding restores that snapshot and removes files
//! created after it, then the core trims the transcript to the same point.
//!
//! Checkpoints work in the project folder itself. Worktrees are a separate,
//! optional isolation mode and do not have to exist for this to be useful. The
//! reference (Waku) separates them the same way.
//!
//! Everything here runs a `git` child process; the core calls it off the UI
//! path.

#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use uuid::Uuid;

/// Whether the snapshot is the state the turn started from or the one it left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointKind {
    Start,
    End,
}

impl CheckpointKind {
    fn label(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

/// Where a checkpoint lives: a private ref and the commit it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub commit: String,
    pub turn: u64,
    pub kind: CheckpointKind,
    /// The ref that keeps the commit reachable across Git's garbage collection.
    pub git_ref: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git: {0}")]
    Io(#[from] std::io::Error),
    #[error("git failed: {0}")]
    Failed(String),
    #[error("this folder is not a Git repository")]
    NotARepository,
}

/// Whether `cwd` is inside a Git worktree. A missing `git` binary reads as
/// `false`: checkpoints are an enhancement, never a reason a turn fails.
pub fn is_repo(cwd: &Path) -> bool {
    git(cwd, ["rev-parse", "--is-inside-work-tree"])
        .map(|output| output.trim() == "true")
        .unwrap_or(false)
}

/// The branch the project is on. A detached head has no branch name; an empty
/// repository has none either.
pub fn branch(cwd: &Path) -> Option<String> {
    if let Ok(name) = git(cwd, ["symbolic-ref", "--short", "HEAD"]) {
        let name = name.trim();
        if !name.is_empty() {
            return Some(name.to_owned());
        }
    }
    None
}

/// Captures the worktree as a dangling commit and points a private ref at it.
///
/// `session` and `turn` name the ref (`refs/circulo/session-<id>/turn-<n>`).
/// A non-repository returns `Ok(None)`; a real Git failure is an error the
/// caller can log and ignore. The user's index is never used: the snapshot
/// stages into a temporary index and drops it.
pub fn capture(
    cwd: &Path,
    session: Uuid,
    turn: u64,
    kind: CheckpointKind,
) -> Result<Option<Checkpoint>, GitError> {
    if !is_repo(cwd) {
        return Ok(None);
    }

    let common = git(cwd, ["rev-parse", "--git-common-dir"])?;
    let common = common.trim();
    if common.is_empty() {
        return Err(GitError::Failed("git returned no common directory".into()));
    }
    let common = {
        let path = PathBuf::from(common);
        if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        }
    };

    let index = common.join(format!(".circulo-checkpoint-index-{}", Uuid::new_v4()));
    let result = (|| {
        let head = match git(cwd, ["rev-parse", "--verify", "HEAD"]) {
            Ok(head) => Some(head.trim().to_owned()),
            Err(_) => None,
        };

        if let Some(head) = &head {
            git_with_index(cwd, &index, ["read-tree", head])?;
        }
        // `-A` includes edits, deletions and untracked files; ignored files
        // stay out, and `clean -fd` later leaves them alone too.
        git_with_index(cwd, &index, ["add", "-A", "--", "."])?;
        let tree = git_with_index(cwd, &index, ["write-tree"])?;
        let tree = tree.trim();
        if tree.is_empty() {
            return Err(GitError::Failed("git write-tree returned no tree".into()));
        }

        let mut arguments = vec![
            "commit-tree".to_owned(),
            tree.to_owned(),
            "-m".to_owned(),
            "Circulo checkpoint".to_owned(),
        ];
        if let Some(head) = &head {
            arguments.push("-p".to_owned());
            arguments.push(head.clone());
        }
        let commit = git(cwd, arguments)?;
        let commit = commit.trim().to_owned();

        let git_ref = checkpoint_ref(session, turn, kind);
        git(cwd, ["update-ref", &git_ref, &commit])?;
        Ok(Checkpoint {
            commit,
            turn,
            kind,
            git_ref,
        })
    })();

    let _ = std::fs::remove_file(&index);
    let _ = std::fs::remove_file(index.with_extension("lock"));
    result.map(Some)
}

/// Restores the worktree to `commit`: tracked files go back to their snapshot,
/// and files created after it are removed. Files matched by `.gitignore` are
/// left alone. The user's index is reset to `HEAD`, so the restored files read
/// as ordinary working-tree changes.
///
/// This is destructive by design; the interface confirms before calling it.
pub fn restore(cwd: &Path, commit: &str) -> Result<(), GitError> {
    if !is_repo(cwd) {
        return Err(GitError::NotARepository);
    }

    git(
        cwd,
        [
            "restore",
            "--source",
            commit,
            "--worktree",
            "--staged",
            "--",
            ".",
        ],
    )?;
    git(cwd, ["clean", "-fd", "--", "."])?;
    if git(cwd, ["rev-parse", "--verify", "HEAD"]).is_ok() {
        git(cwd, ["reset", "--quiet", "--", "."])?;
    }
    Ok(())
}

/// The commit a checkpoint ref names, or `None` when it was never captured.
pub fn resolved(cwd: &Path, session: Uuid, turn: u64, kind: CheckpointKind) -> Option<String> {
    git(
        cwd,
        [
            "rev-parse",
            "--verify",
            &checkpoint_ref(session, turn, kind),
        ],
    )
    .ok()
    .map(|commit| commit.trim().to_owned())
    .filter(|commit| !commit.is_empty())
}

/// Whether that checkpoint exists.
pub fn has_checkpoint(cwd: &Path, session: Uuid, turn: u64, kind: CheckpointKind) -> bool {
    resolved(cwd, session, turn, kind).is_some()
}

fn checkpoint_ref(session: Uuid, turn: u64, kind: CheckpointKind) -> String {
    format!(
        "refs/circulo/session-{session}/turn-{turn}-{}",
        kind.label()
    )
}

fn git<I, S>(cwd: &Path, args: I) -> Result<String, GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git_output(cwd, args, &[])?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(GitError::Failed(command_error(&output)))
}

fn git_with_index<I, S>(cwd: &Path, index: &Path, args: I) -> Result<String, GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git_output(cwd, args, &[("GIT_INDEX_FILE", index.as_os_str())])?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(GitError::Failed(command_error(&output)))
}

fn git_output<I, S>(cwd: &Path, args: I, envs: &[(&str, &OsStr)]) -> Result<Output, GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command.current_dir(cwd).args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    // A checkpoint must not ask the user for an identity or open an editor.
    command
        .env("GIT_AUTHOR_NAME", "Circulo")
        .env("GIT_AUTHOR_EMAIL", "checkpoint@circulo.local")
        .env("GIT_COMMITTER_NAME", "Circulo")
        .env("GIT_COMMITTER_EMAIL", "checkpoint@circulo.local")
        .env("GIT_TERMINAL_PROMPT", "0");
    Ok(command.output()?)
}

fn command_error(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        format!("git exited with {}", output.status)
    } else {
        stderr.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    struct Repo {
        directory: PathBuf,
    }

    impl Repo {
        fn new() -> Option<Self> {
            if !git_available() {
                return None;
            }
            let directory =
                std::env::temp_dir().join(format!("circulo-git-test-{}", Uuid::new_v4()));
            fs::create_dir_all(&directory).expect("temp repo");
            let repo = Self { directory };
            repo.run(["init", "-q"]);
            repo.run(["config", "user.name", "Test"]);
            repo.run(["config", "user.email", "test@example.com"]);
            repo.write("a.txt", "one");
            repo.run(["add", "a.txt"]);
            repo.run(["commit", "-qm", "init"]);
            Some(repo)
        }

        fn run<I, S>(&self, args: I)
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            let status = Command::new("git")
                .current_dir(&self.directory)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git command failed");
        }

        fn write(&self, name: &str, contents: &str) {
            fs::write(self.directory.join(name), contents).expect("write");
        }

        fn read(&self, name: &str) -> String {
            fs::read_to_string(self.directory.join(name)).expect("read")
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }

    #[test]
    fn a_checkpoint_restores_the_worktree_and_removes_later_files() {
        let Some(repo) = Repo::new() else {
            return;
        };
        let session = Uuid::new_v4();

        let first = capture(&repo.directory, session, 1, CheckpointKind::Start)
            .expect("capture")
            .expect("a repo has checkpoints");
        assert!(first.commit.len() >= 7);

        repo.write("a.txt", "two");
        repo.write("new.txt", "later");
        let second = capture(&repo.directory, session, 2, CheckpointKind::End)
            .expect("capture")
            .expect("a repo has checkpoints");
        assert_ne!(first.commit, second.commit);

        repo.write("a.txt", "three");
        repo.write("another.txt", "later still");

        restore(&repo.directory, &first.commit).expect("restore first");
        assert_eq!(repo.read("a.txt"), "one");
        assert!(!repo.directory.join("new.txt").exists());
        assert!(!repo.directory.join("another.txt").exists());

        restore(&repo.directory, &second.commit).expect("restore second");
        assert_eq!(repo.read("a.txt"), "two");
        assert_eq!(repo.read("new.txt"), "later");
    }

    #[test]
    fn the_users_branch_and_index_are_not_used_for_capture() {
        let Some(repo) = Repo::new() else {
            return;
        };
        repo.write("staged.txt", "staged");
        repo.run(["add", "staged.txt"]);
        repo.write("a.txt", "unstaged");

        let checkpoint = capture(&repo.directory, Uuid::new_v4(), 1, CheckpointKind::Start)
            .expect("capture")
            .expect("a repo has checkpoints");

        // The user's index still holds the staged file exactly as it was.
        let staged = git(&repo.directory, ["diff", "--cached", "--name-only"]).expect("diff");
        assert_eq!(staged.trim(), "staged.txt");
        assert!(!checkpoint.commit.is_empty());
    }

    #[test]
    fn a_non_repository_has_no_checkpoints() {
        let directory = std::env::temp_dir().join(format!("circulo-git-plain-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).expect("temp");
        assert!(
            capture(&directory, Uuid::new_v4(), 1, CheckpointKind::Start)
                .expect("not a repo is not an error")
                .is_none()
        );
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_branch_is_the_current_one() {
        let Some(repo) = Repo::new() else {
            return;
        };
        repo.run(["checkout", "-q", "-b", "feature/test"]);
        assert_eq!(branch(&repo.directory).as_deref(), Some("feature/test"));
    }
}
