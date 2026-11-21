// Command circulo is the native window.
// It starts the core as a separate process and only speaks protocol.
package main

import (
	"context"
	"log"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"

	"github.com/soycanopa/circulo/ui/session"
)

func main() {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	model := &session.Model{}
	app := session.NewApp(model)

	mygo.App.WhenReady(func() {
		win := mygo.NewWindow(mygo.WindowOptions{
			Title:    "Circulo",
			Width:    980,
			Height:   680,
			StateKey: "main",
			Content:  ui.View(app.View),
		})
		go connect(ctx, win, app)
	})
	if err := mygo.App.Run(); err != nil {
		log.Fatal(err)
	}
}
