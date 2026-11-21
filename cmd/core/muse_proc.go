package main

import (
	"context"
	"fmt"
	"os"
	"os/exec"

	"github.com/soycanopa/circulo/driver"
	"github.com/soycanopa/circulo/providers/muse"
)

// startMuse runs `muse serve` on stdio. MUSE_BIN overrides the executable.
func startMuse(ctx context.Context) (driver.Driver, func(), error) {
	noop := func() {}
	bin := os.Getenv("MUSE_BIN")
	if bin == "" {
		bin = "muse"
	}
	if _, err := exec.LookPath(bin); err != nil {
		return nil, noop, fmt.Errorf("muse: %w", err)
	}
	client, err := muse.Start(ctx, bin)
	if err != nil {
		return nil, noop, err
	}
	return client, client.Stop, nil
}
