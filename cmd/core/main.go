// Command core is the Circulo core process.
// It listens on loopback and speaks protocol. It does not draw a window.
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"path/filepath"
	"time"

	"github.com/soycanopa/circulo/core"
	"github.com/soycanopa/circulo/persistence"
)

func main() {
	addr := flag.String("listen", "127.0.0.1:0", "loopback address to bind")
	delay := flag.Duration("delay", 30*time.Millisecond, "pause between live-reply chunks in -demo")
	demo := flag.Bool("demo", false, "stream the canned transcript instead of OpenCode")
	data := flag.String("data", "", "SQLite database path")
	flag.Parse()

	ln, err := core.Listen(*addr)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Println(ln.Addr().String())

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt)
	defer stop()
	if *demo {
		if err := core.Serve(ctx, ln, *delay); err != nil {
			log.Fatal(err)
		}
		return
	}
	path := *data
	if path == "" {
		dir, err := os.UserConfigDir()
		if err != nil {
			log.Fatal(err)
		}
		path = filepath.Join(dir, "Circulo", "circulo.db")
	}
	db, err := persistence.Open(path)
	if err != nil {
		log.Fatal(err)
	}
	defer db.Close()
	drv, stopDrv, boot := startOpenCode(ctx)
	defer stopDrv()
	if err := core.NewRegistry(db, drv, boot).Serve(ctx, ln); err != nil {
		log.Fatal(err)
	}
}
