package session

import (
	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/markdown"

	"github.com/soycanopa/circulo/protocol"
)

// App is the native window: projects, the transcript, and the composer.
type App struct {
	Model *Model
	// Send delivers a request to the core. It is safe to call from the UI thread.
	Send         func(protocol.Envelope)
	path         string
	draft        string
	modelLabel   string
	variantLabel string
	synced       string
	seq          int64
	md           markdown.Options
	list         ui.ListState
}

// NewApp builds the window state. Model is updated on the UI thread.
func NewApp(m *Model) *App {
	a := &App{Model: m}
	a.md.ShareSelection = true
	a.md.Theme = &markdown.Theme{
		FontSize: 15,
		Link:     ui.Hex("#E2795B"),
	}
	return a
}

// View builds one frame. MyGo calls it on the main thread.
func (a *App) View(c *ui.Context) {
	ui.Row(c).Fill().Children(func() {
		a.sidebar(c)
		a.transcript(c)
	})
}

func (a *App) sidebar(c *ui.Context) {
	ui.Column(c).Width(240).Padding(16).Gap(8).Background(ui.Hex("#141414")).Children(func() {
		ui.Text(c, "Circulo").Bold().FontSize(18)
		ui.TextInput(c, &a.path).Placeholder("Project folder")
		if ui.Button(c, "Add").Clicked() {
			a.request(protocol.Envelope{Type: protocol.TypeAdd, Path: a.path})
		}
		for _, sess := range a.Model.Sessions {
			title := sess.Title
			if title == "" {
				title = sess.ID
			}
			if ui.Button(c, title).Clicked() {
				a.request(protocol.Envelope{Type: protocol.TypeOpen, ID: sess.ID})
			}
		}
		if len(a.Model.Projects) > 0 {
			project := a.Model.Projects[0].ID
			if a.Model.Selected != "" {
				for _, sess := range a.Model.Sessions {
					if sess.ID == a.Model.Selected {
						project = sess.Project
					}
				}
			}
			if ui.Button(c, "New session").Clicked() {
				a.request(protocol.Envelope{Type: protocol.TypeCreate, Project: project})
			}
		}
	})
}

func (a *App) transcript(c *ui.Context) {
	ui.Column(c).Grow(1).Padding(12).Gap(8).Children(func() {
		status := a.Model.Status
		if status == "" {
			status = "Starting core"
		}
		ui.Text(c, status).TextColor(c.Theme().TextMuted)

		msgs := a.Model.Visible()
		if a.Model.Selected == "" {
			msgs = a.Model.Messages
		}
		a.list.Key = func(i int) any { return msgs[i].ID }
		a.list.Label = func(i int) string { return roleLabel(msgs[i].Role) }
		a.list.FollowEnd = true
		a.list.FollowAnimation = &ui.Smooth
		ui.List(c, &a.list, len(msgs), func(i int) {
			a.message(c, msgs[i])
		}).Grow(1).Gap(12).Justify(ui.End)

		if !a.list.AtEnd() && ui.Button(c, "Jump to latest").Clicked() {
			a.list.ScrollToEnd()
		}
		a.controls(c)
		ui.Row(c).Gap(8).Children(func() {
			ui.TextInput(c, &a.draft).Placeholder("Message").Grow(1)
			if a.Model.Busy() {
				if ui.Button(c, "Steer").Clicked() {
					a.sendDraft("steer")
				}
				if ui.Button(c, "Queue").Clicked() {
					a.sendDraft("queue")
				}
				if ui.Button(c, "Stop").Clicked() && a.Model.Selected != "" {
					a.request(protocol.Envelope{Type: protocol.TypeAbort, ID: a.Model.Selected})
				}
			} else if ui.PrimaryButton(c, "Send").Clicked() {
				a.sendDraft("")
			}
		})
	})
}

func (a *App) controls(c *ui.Context) {
	label, variant := a.Model.Selection()
	key := a.Model.Selected + "\x00" + label + "\x00" + variant
	if key != a.synced {
		a.modelLabel = label
		a.variantLabel = variant
		a.synced = key
	}
	if len(a.Model.Models) == 0 {
		return
	}
	options := make([]string, len(a.Model.Models))
	for i, it := range a.Model.Models {
		options[i] = it.Label
	}
	ui.Row(c).Gap(8).Children(func() {
		if ui.Combobox(c, &a.modelLabel, options).Grow(1).Changed() {
			a.sendModel()
		}
		it, ok := a.Model.Lookup(a.modelLabel)
		if !ok || len(it.Variants) == 0 {
			return
		}
		if ui.Combobox(c, &a.variantLabel, it.Variants).Changed() {
			a.sendModel()
		}
	})
}

func (a *App) sendDraft(delivery string) {
	if a.draft == "" || a.Model.Selected == "" {
		return
	}
	a.request(protocol.Envelope{
		Type: protocol.TypePrompt, ID: a.Model.Selected, Text: a.draft, Delivery: delivery,
	})
	a.draft = ""
}

func (a *App) sendModel() {
	it, ok := a.Model.Lookup(a.modelLabel)
	if !ok || a.Model.Selected == "" {
		return
	}
	variant := a.variantLabel
	known := false
	for _, v := range it.Variants {
		if v == variant {
			known = true
			break
		}
	}
	if !known && len(it.Variants) > 0 {
		variant = it.Variants[0]
		a.variantLabel = variant
	}
	a.synced = a.Model.Selected + "\x00" + it.Label + "\x00" + variant
	a.request(protocol.Envelope{
		Type: protocol.TypeUse, ID: a.Model.Selected,
		Provider: it.Provider, ModelID: it.ID, Variant: variant,
	})
}

func (a *App) request(e protocol.Envelope) {
	if a.Send == nil {
		return
	}
	a.seq++
	e.V = protocol.Version
	e.Seq = a.seq
	a.Send(e)
}

func (a *App) message(c *ui.Context, m Message) {
	ui.Column(c).Selectable().Padding(8, 4).Gap(4).Children(func() {
		ui.Text(c, roleLabel(m.Role)).Bold()
		if m.Done {
			markdown.View(c, m.Text, &a.md)
		} else {
			markdown.Stream(c, m.Text, &a.md)
		}
	})
}

func roleLabel(role string) string {
	if role == "user" {
		return "You"
	}
	return "Agent"
}
