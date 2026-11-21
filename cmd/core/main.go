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
	"time"

	"github.com/soycanopa/circulo/core"
)

func main() {
	addr := flag.String("listen", "127.0.0.1:0", "loopback address to bind")
	delay := flag.Duration("delay", 30*time.Millisecond, "pause between live-reply chunks")
	flag.Parse()

	ln, err := core.Listen(*addr)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Println(ln.Addr().String())

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt)
	defer stop()
	if err := core.Serve(ctx, ln, *delay); err != nil {
		log.Fatal(err)
	}
}
