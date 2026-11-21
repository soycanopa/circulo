//go:build windows

package main

import (
	"context"
	"os/exec"
)

func command(ctx context.Context, name string, arg ...string) *exec.Cmd {
	return exec.CommandContext(ctx, name, arg...)
}
