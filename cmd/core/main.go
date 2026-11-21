// Command core is the Circulo core process.
// It listens on loopback and speaks protocol. It does not draw a window.
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"path/filepath"
	"time"

	"github.com/soycanopa/circulo/core"
	"github.com/soycanopa/circulo/driver"
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
	opencodeDrv, stopOpenCode, errOpenCode := startOpenCode(ctx)
	defer stopOpenCode()
	museDrv, stopMuse, errMuse := startMuse(ctx)
	defer stopMuse()
	drivers := map[string]driver.Driver{}
	if opencodeDrv != nil {
		drivers["opencode"] = opencodeDrv
	} else {
		log.Printf("opencode: %v", errOpenCode)
	}
	if museDrv != nil {
		drivers["muse"] = museDrv
	} else {
		log.Printf("muse: %v", errMuse)
	}
	var boot error
	if len(drivers) == 0 {
		boot = errors.Join(errOpenCode, errMuse)
	}
	reg := core.NewRegistry(db, nil, boot)
	reg.Drivers = drivers
	if err := reg.Serve(ctx, ln); err != nil {
		log.Fatal(err)
	}
}
