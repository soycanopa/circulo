package session

import (
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/markdown"

	"github.com/soycanopa/circulo/protocol"
)

// Graphite surfaces, the same roles Waku uses: canvas, raised rows, composer, coral accent.
// Color is reserved for the mark, a live reply, and a permission or error.
const (
	colorCanvas   = "#1A1A1A"
	colorSidebar  = "#181818"
	colorRaised   = "#232323"
	colorComposer = "#212121"
	colorText     = "#E2E2E2"
	colorMuted    = "#A3A3A3"
	colorGhost    = "#7D7D7D"
	colorAccent   = "#E2795B"
	colorDanger   = "#E2726A"
)

// App is the native window: projects, the transcript, and the composer.
type App struct {
	Model *Model
	// Send delivers a request to the core. It is safe to call from the UI thread.
	Send         func(protocol.Envelope)
	path         string
	agent        string
	query        string
	draft        string
	modelLabel   string
	variantLabel string
	synced       string
	rail         string
	seq          int64
	md           markdown.Options
	list         ui.ListState
}

// NewApp builds the window state. Model is updated on the UI thread.
func NewApp(m *Model) *App {
	a := &App{Model: m, agent: "opencode", rail: "Files"}
	a.md.ShareSelection = true
	a.md.Theme = &markdown.Theme{
		FontSize: 15,
		Link:     ui.Hex(colorAccent),
		Code:     ui.Hex("#E0A882"),
	}
	return a
}

// View builds one frame. MyGo calls it on the main thread.
func (a *App) View(c *ui.Context) {
	ui.Column(c).Fill().Background(ui.Hex(colorCanvas)).Children(func() {
		a.titlebar(c)
		ui.Row(c).Grow(1).Children(func() {
			a.sidebar(c)
			a.transcript(c)
			a.railView(c)
		})
	})
}

func (a *App) titlebar(c *ui.Context) {
	bar := c.TitleBar()
	h := bar.Height
	if h < 38 {
		h = 38
	}
	title := "Circulo"
	if name := a.selectedTitle(); name != "" {
		title = name
	}
	ui.Row(c).Height(h).Padding(0, bar.Right, 0, bar.Left).Gap(12).DragWindow().Children(func() {
		ui.Text(c, "●").TextColor(ui.Hex(colorAccent)).FontSize(12)
		ui.Text(c, title).TextColor(ui.Hex(colorText))
		ui.Spacer(c).Grow(1)
		status := a.Model.Status
		if status == "" {
			status = "Starting core"
		}
		if !banner(status) {
			ui.Text(c, status).TextColor(ui.Hex(colorGhost)).FontSize(12)
		}
	})
}

func (a *App) sidebar(c *ui.Context) {
	ui.Column(c).Width(252).Padding(12, 12, 16, 12).Gap(8).Background(ui.Hex(colorSidebar)).Children(func() {
		ui.Text(c, "Circulo").Bold().FontSize(16).TextColor(ui.Hex(colorText))
		ui.TextInput(c, &a.query).Placeholder("Filter sessions")
		ui.TextInput(c, &a.path).Placeholder("Project folder")
		if a.agent == "" {
			a.agent = "opencode"
		}
		ui.Combobox(c, &a.agent, []string{"opencode", "muse"}).Label("Agent")
		if ui.Button(c, "Add").Clicked() {
			a.request(protocol.Envelope{Type: protocol.TypeAdd, Path: a.path, Agent: a.agent})
		}
		shown := false
		for _, project := range a.Model.Projects {
			sessions := a.sessionsOf(project.ID)
			if len(sessions) == 0 && a.query != "" {
				continue
			}
			shown = true
			label := project.Name
			if project.Agent != "" {
				label += " · " + project.Agent
			}
			ui.Text(c, label).FontSize(12).TextColor(ui.Hex(colorGhost))
			for _, sess := range sessions {
				a.sessionButton(c, sess)
			}
		}
		if !shown {
			for _, sess := range a.filtered(a.Model.Sessions) {
				a.sessionButton(c, sess)
			}
		}
		if len(a.Model.Projects) > 0 {
			project := a.currentProject()
			if ui.Button(c, "New session").Clicked() {
				a.request(protocol.Envelope{Type: protocol.TypeCreate, Project: project})
			}
		}
	})
}

func (a *App) sessionButton(c *ui.Context, sess SessionInfo) {
	title := sess.Title
	if title == "" {
		title = sess.ID
	}
	if sess.ID == a.Model.Selected {
		title = "● " + title
	}
	row := ui.Button(c, title)
	if sess.ID == a.Model.Selected {
		row = row.Background(ui.Hex(colorRaised))
	}
	if row.Clicked() {
		a.request(protocol.Envelope{Type: protocol.TypeOpen, ID: sess.ID})
	}
}

func (a *App) transcript(c *ui.Context) {
	ui.Column(c).Grow(1).Padding(16, 16, 12, 16).Gap(8).Children(func() {
		if status := a.Model.Status; banner(status) {
			ui.Row(c).Padding(8, 10).Radius(8).Background(ui.Hex("#3A2422")).Children(func() {
				ui.Text(c, status).TextColor(ui.Hex(colorDanger))
			})
		}
		a.permissionCard(c)

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
		}).Grow(1).Gap(16).Justify(ui.End)

		if !a.list.AtEnd() && ui.Button(c, "Jump to latest").Clicked() {
			a.list.ScrollToEnd()
		}
		a.composer(c)
	})
}

func (a *App) permissionCard(c *ui.Context) {
	p := a.Model.Permission
	if p == nil || (a.Model.Selected != "" && p.Session != a.Model.Selected) {
		return
	}
	ui.Column(c).Padding(12).Gap(8).Radius(10).Background(ui.Hex(colorRaised)).Children(func() {
		ui.Text(c, p.Tool).Bold().TextColor(ui.Hex(colorAccent))
		if p.Text != "" && p.Text != p.Tool {
			ui.Text(c, p.Text).TextColor(ui.Hex(colorMuted))
		}
		ui.Row(c).Gap(8).Children(func() {
			for _, choice := range p.Choices {
				label := choice.Label
				if label == "" {
					label = choice.Decision
				}
				ch := choice
				if ui.Button(c, label).Clicked() {
					a.request(protocol.Envelope{
						Type: protocol.TypePermitReply, ID: p.ID, Session: p.Session,
						Text: ch.ID, Requirement: p.Requirement,
					})
					a.Model.Permission = nil
				}
			}
		})
	})
}

func (a *App) composer(c *ui.Context) {
	ui.Column(c).Padding(10).Gap(8).Radius(12).Background(ui.Hex(colorComposer)).Children(func() {
		a.controls(c)
		ui.TextArea(c, &a.draft).Placeholder("Message").Label("Message").Lines(1, 6).Grow(1).OnTextKey(func(k ui.TextKey) bool {
			if k.Key == ui.KeyEscape && a.Model.Busy() && a.Model.Selected != "" {
				a.request(protocol.Envelope{Type: protocol.TypeAbort, ID: a.Model.Selected})
				return true
			}
			if k.Key != ui.KeyEnter {
				return false
			}
			if k.Mods&ui.Shift != 0 {
				return false
			}
			if k.Mods&ui.Cmd != 0 && a.Model.Busy() {
				a.sendDraft("steer")
				k.SetText("")
				return true
			}
			if k.Mods != 0 {
				return false
			}
			if a.Model.Busy() {
				a.sendDraft("queue")
			} else {
				a.sendDraft("")
			}
			k.SetText("")
			return true
		})
		ui.Row(c).Gap(8).Children(func() {
			ui.Spacer(c).Grow(1)
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
			if id := a.Model.RewindID(); id != "" && ui.Button(c, "Rewind").Clicked() {
				a.request(protocol.Envelope{Type: protocol.TypeRewind, ID: id, Session: a.Model.Selected})
			}
		})
	})
}

func (a *App) railView(c *ui.Context) {
	ui.Column(c).Width(220).Padding(12).Gap(8).Background(ui.Hex(colorSidebar)).Children(func() {
		ui.Row(c).Gap(6).Children(func() {
			for _, tab := range []string{"Files", "Diff", "Terminal"} {
				name := tab
				b := ui.Button(c, name)
				if a.rail == name {
					b = b.TextColor(ui.Hex(colorText))
				}
				if b.Clicked() {
					a.rail = name
				}
			}
		})
		ui.Text(c, a.rail+" will show the worktree here.").TextColor(ui.Hex(colorGhost)).FontSize(13)
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
	ui.Column(c).Selectable().Padding(4, 0).Gap(4).Children(func() {
		ui.Text(c, roleLabel(m.Role)).Bold().FontSize(12).TextColor(ui.Hex(colorMuted))
		if m.Done {
			markdown.View(c, m.Text, &a.md)
		} else {
			markdown.Stream(c, m.Text, &a.md)
		}
	})
}

func (a *App) selectedTitle() string {
	var project, session string
	for _, sess := range a.Model.Sessions {
		if sess.ID != a.Model.Selected {
			continue
		}
		session = sess.Title
		if session == "" {
			session = sess.ID
		}
		for _, p := range a.Model.Projects {
			if p.ID == sess.Project {
				project = p.Name
			}
		}
	}
	if project == "" {
		return session
	}
	if session == "" {
		return project
	}
	return project + "  /  " + session
}

func (a *App) currentProject() string {
	if a.Model.Selected != "" {
		for _, sess := range a.Model.Sessions {
			if sess.ID == a.Model.Selected {
				return sess.Project
			}
		}
	}
	if len(a.Model.Projects) > 0 {
		return a.Model.Projects[0].ID
	}
	return ""
}

func (a *App) sessionsOf(project string) []SessionInfo {
	var out []SessionInfo
	for _, sess := range a.Model.Sessions {
		if sess.Project == project {
			out = append(out, sess)
		}
	}
	return a.filtered(out)
}

func (a *App) filtered(in []SessionInfo) []SessionInfo {
	q := strings.ToLower(strings.TrimSpace(a.query))
	if q == "" {
		return in
	}
	var out []SessionInfo
	for _, sess := range in {
		title := sess.Title
		if title == "" {
			title = sess.ID
		}
		if strings.Contains(strings.ToLower(title), q) || strings.Contains(strings.ToLower(sess.Agent), q) {
			out = append(out, sess)
		}
	}
	return out
}

func roleLabel(role string) string {
	if role == "user" {
		return "You"
	}
	return "Agent"
}

func banner(status string) bool {
	switch status {
	case "", "Idle", "Streaming", "Sending", "Starting core", "Core connected", "Rewound the files and the transcript":
		return false
	default:
		return true
	}
}
