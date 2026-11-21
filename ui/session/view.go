package session

import (
	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/markdown"
)

// App is the native window: a sidebar placeholder and the transcript.
type App struct {
	Model *Model
	md    markdown.Options
	list  ui.ListState
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
		ui.Text(c, "Transcript").TextColor(ui.Hex("#E2795B"))
		ui.Text(c, "Projects and sessions land in the next phase.").TextColor(c.Theme().TextMuted)
	})
}

func (a *App) transcript(c *ui.Context) {
	ui.Column(c).Grow(1).Padding(12).Gap(8).Children(func() {
		status := a.Model.Status
		if status == "" {
			status = "Starting core"
		}
		ui.Text(c, status).TextColor(c.Theme().TextMuted)

		msgs := a.Model.Messages
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
	})
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
