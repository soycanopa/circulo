// Package appservice hosts Wails-bound services — the composition layer that
// is allowed to import the Wails runtime (AGENTS.md architecture invariants:
// adapters and orchestrator must not).
package appservice

import (
	"github.com/wailsapp/wails/v3/pkg/application"
)

// Dialog exposes native dialogs to the frontend via generated bindings.
type Dialog struct{}

// NewDialog builds the dialog service.
func NewDialog() *Dialog { return &Dialog{} }

// PickFolder shows a native folder picker and returns the chosen absolute path
// ("" when the user cancels — not an error).
func (d *Dialog) PickFolder() (string, error) {
	dlg := application.Get().Dialog.OpenFile()
	dlg.CanChooseDirectories(true)
	dlg.CanChooseFiles(false)
	dlg.CanCreateDirectories(true)
	dlg.SetTitle("Choose a project folder")
	return dlg.PromptForSingleSelection()
}
