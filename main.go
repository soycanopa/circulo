package main

import (
	"context"
	"embed"
	"log"
	"net/http"
	"os"

	"circulogo/internal/agent"
	"circulogo/internal/appservice"
	"circulogo/internal/opencode"
	"circulogo/internal/orchestrator"
	"circulogo/internal/relay"
	"circulogo/internal/store"

	"github.com/wailsapp/wails/v3/pkg/application"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	// Composition root: the only place that wires concrete adapters to the
	// orchestrator (AGENTS.md architecture invariants).
	stPath, err := store.DefaultPath()
	if err != nil {
		log.Printf("settings path unavailable, using cwd store: %v", err)
		stPath = "circulogo-settings.json"
	}
	st := store.New(stPath)
	orch := orchestrator.New(st, func(cfg store.Project) (agent.Adapter, error) {
		return opencode.NewAdapter(opencode.AdapterConfig{
			ProjectID: cfg.ID,
			Mode:      cfg.Mode,
			Dir:       cfg.Path,
			URL:       cfg.URL,
		}), nil
	})

	// The adapters tie their processes to their own lifetime (adapter.Stop),
	// so a background context is the right scope here.
	if err := orch.Load(context.Background()); err != nil {
		// A corrupt settings file must not brick the app: start empty.
		log.Printf("loading projects failed (starting empty): %v", err)
	}
	defer orch.Shutdown() // NFR-4: no orphaned opencode serve processes

	api := relay.New(orch)
	dialogs := appservice.NewDialog()

	// Opt-in loopback listener serving the same relay mux — lets curl/scripts
	// exercise the agent API without the webview (docs/implement.md E2E).
	// Binds 127.0.0.1 only, and only when CIRCULOGO_DEBUG_ADDR is set.
	if addr := os.Getenv("CIRCULOGO_DEBUG_ADDR"); addr != "" {
		go func() {
			// Same exposure shape as the remote design (docs/remote.md F1):
			// one port serving the embedded UI next to the /agent relay.
			mux := http.NewServeMux()
			mux.Handle("/agent", http.StripPrefix("/agent", api))
			mux.Handle("/agent/", http.StripPrefix("/agent", api))
			mux.Handle("/", application.AssetFileServerFS(assets))
			log.Printf("debug UI+API listening on http://%s", addr)
			if err := http.ListenAndServe(addr, mux); err != nil {
				log.Printf("debug listener stopped: %v", err)
			}
		}()
	}

	app := application.New(application.Options{
		Name:        "circuloGo",
		Description: "Local-first orchestrator for coding-agent CLIs",
		Services: []application.Service{
			application.NewService(dialogs),
			application.NewServiceWithOptions(api, application.ServiceOptions{
				// Same-origin streaming API for the webview (docs/trd.md §4).
				Route: "/agent",
			}),
		},
		Assets: application.AssetOptions{
			Handler: application.AssetFileServerFS(assets),
		},
		Mac: application.MacOptions{
			ApplicationShouldTerminateAfterLastWindowClosed: true,
		},
	})

	app.Window.NewWithOptions(application.WebviewWindowOptions{
		Title:     "circuloGo",
		Width:     1280,
		Height:    800,
		MinWidth:  960,
		MinHeight: 640,
		Mac: application.MacWindow{
			InvisibleTitleBarHeight: 44,
			Backdrop:                application.MacBackdropTranslucent,
			TitleBar:                application.MacTitleBarHiddenInset,
		},
		// zinc-950: match the UI theme so there is no white flash on launch.
		BackgroundColour: application.NewRGB(9, 9, 11),
		URL:              "/",
	})

	if err := app.Run(); err != nil {
		log.Fatal(err)
	}
}
