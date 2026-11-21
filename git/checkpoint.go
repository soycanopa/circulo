// Package git owns conversation-aware checkpoints of a worktree.
// Only the core process imports it. The UI asks for a rewind through the protocol.
package git

import (
	"errors"
	"fmt"
	"os/exec"
	"strings"
)

// ErrNotRepo means dir is not inside a Git worktree.
var ErrNotRepo = errors.New("git: not a repository")

// Checkpoint records the worktree without changing it.
// A dirty tree becomes a stash commit. A clean tree returns HEAD.
func Checkpoint(dir string) (string, error) {
	if err := run(dir, "rev-parse", "--is-inside-work-tree"); err != nil {
		return "", ErrNotRepo
	}
	out, err := output(dir, "stash", "create")
	if err != nil {
		return "", err
	}
	rev := strings.TrimSpace(out)
	if rev == "" {
		out, err = output(dir, "rev-parse", "HEAD")
		if err != nil {
			return "", err
		}
		rev = strings.TrimSpace(out)
	}
	if !validRev(rev) {
		return "", fmt.Errorf("git: unexpected revision %q", rev)
	}
	return rev, nil
}

// Rewind makes the worktree and index match rev.
// rev must be a full hex object name previously returned by Checkpoint.
func Rewind(dir, rev string) error {
	if !validRev(rev) {
		return fmt.Errorf("git: unexpected revision %q", rev)
	}
	if err := run(dir, "cat-file", "-e", rev+"^{commit}"); err != nil {
		return fmt.Errorf("git: %s is not a commit", rev)
	}
	return run(dir, "reset", "--hard", rev)
}

func validRev(s string) bool {
	if len(s) < 7 || len(s) > 64 {
		return false
	}
	for _, c := range s {
		switch {
		case c >= '0' && c <= '9', c >= 'a' && c <= 'f', c >= 'A' && c <= 'F':
		default:
			return false
		}
	}
	return true
}

func run(dir string, args ...string) error {
	cmd := exec.Command("git", append([]string{"-C", dir}, args...)...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("git %s: %w (%s)", args[0], err, strings.TrimSpace(string(out)))
	}
	return nil
}

func output(dir string, args ...string) (string, error) {
	cmd := exec.Command("git", append([]string{"-C", dir}, args...)...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("git %s: %w (%s)", args[0], err, strings.TrimSpace(string(out)))
	}
	return string(out), nil
}
