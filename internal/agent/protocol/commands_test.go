package protocol

import (
	"reflect"
	"testing"
)

// The whole point of P4 is that presentation order and resolution precedence
// are two different orderings. If these tests ever read the same, one of them
// is wrong.

func cmd(name string, scope CommandScope) CommandInfo {
	return CommandInfo{Name: name, Scope: scope}
}

func names(in []CommandInfo) []string {
	out := make([]string, 0, len(in))
	for _, c := range in {
		out = append(out, c.Name)
	}
	return out
}

func TestNormalizeScope_MapsProviderVocabulary(t *testing.T) {
	for raw, want := range map[string]CommandScope{
		"project":  ScopeProject,
		"user":     ScopeUser,
		"skill":    ScopeSkill,
		"custom":   ScopeExtension,
		"file":     ScopeProjectFile,
		"builtin":  ScopeBuiltin,
		"":         ScopeBuiltin,
		"nonsense": ScopeBuiltin,
	} {
		if got := NormalizeScope(raw); got != want {
			t.Errorf("NormalizeScope(%q) = %q, want %q", raw, got, want)
		}
	}
}

// A backend's own word must never reach the UI: an unknown value becomes the
// neutral builtin rather than crossing the boundary as a string nobody can
// render.
func TestNormalizeScope_UnknownBecomesBuiltin(t *testing.T) {
	if got := NormalizeScope("some-future-source"); got != ScopeBuiltin {
		t.Errorf("unknown source = %q, want %q", got, ScopeBuiltin)
	}
}

func TestResolveCommands_EmptyAndNameless(t *testing.T) {
	if got := ResolveCommands(nil); len(got) != 0 {
		t.Errorf("nil input produced %d entries", len(got))
	}
	// A command with no name cannot be invoked, so it must not occupy a slot.
	if got := ResolveCommands([]CommandInfo{cmd("", ScopeUser)}); len(got) != 0 {
		t.Errorf("a nameless command survived: %+v", got)
	}
}

// A project command shadows a builtin, or the builtin would make the project's
// command unreachable — the opposite of what a project command is for.
func TestResolveCommands_ProjectShadowsBuiltin(t *testing.T) {
	got := ResolveCommands([]CommandInfo{
		cmd("review", ScopeBuiltin),
		cmd("review", ScopeProject),
	})
	if len(got) != 1 {
		t.Fatalf("collision not collapsed: %+v", got)
	}
	if got[0].Scope != ScopeProject {
		t.Errorf("winner scope = %q, want %q", got[0].Scope, ScopeProject)
	}
}

func TestResolveCommands_InputOrderDoesNotMatter(t *testing.T) {
	all := []CommandInfo{
		cmd("a", ScopeSkill),
		cmd("a", ScopeProject),
		cmd("a", ScopeBuiltin),
		cmd("b", ScopeUser),
		cmd("b", ScopeExtension),
	}
	forward := ResolveCommands(all)
	reversed := ResolveCommands([]CommandInfo{all[4], all[3], all[2], all[1], all[0]})
	if !reflect.DeepEqual(names(forward), names(reversed)) {
		t.Errorf("order changed the result: %v vs %v", names(forward), names(reversed))
	}
	// And the winner is stable, not just the ordering.
	byName := map[string]CommandInfo{}
	for _, c := range forward {
		byName[c.Name] = c
	}
	if byName["a"].Scope != ScopeProject || byName["b"].Scope != ScopeUser {
		t.Errorf("wrong winners: a=%q b=%q", byName["a"].Scope, byName["b"].Scope)
	}
}

// The assertion that makes P4 worth having: the winner of a collision is NOT
// the entry shown first. If these two ever agree, one of the orderings has
// collapsed into the other.
func TestResolveCommands_DisplayOrderDiffersFromPrecedence(t *testing.T) {
	got := ResolveCommands([]CommandInfo{
		cmd("review", ScopeProject),
		cmd("review", ScopeBuiltin),
	})
	if len(got) != 1 {
		t.Fatalf("expected the collision collapsed: %+v", got)
	}
	// Resolution: project wins.
	if got[0].Scope != ScopeProject {
		t.Errorf("resolution winner = %q, want project", got[0].Scope)
	}
	// Presentation: the builtin ranks first even though it lost. That
	// mismatch is the feature; if the two ranks ever agree, one ordering has
	// collapsed into the other.
	if ScopeBuiltin.displayRank() >= ScopeProject.displayRank() {
		t.Error("display order no longer puts the builtin before the project command")
	}
	if ScopeProject.resolutionRank() <= ScopeBuiltin.resolutionRank() {
		t.Error("resolution order no longer lets a project command outrank a builtin")
	}
}

func TestResolveCommands_DisplayPutsBuiltinsFirst(t *testing.T) {
	got := ResolveCommands([]CommandInfo{
		cmd("z-skill", ScopeSkill),
		cmd("a-user", ScopeUser),
		cmd("m-builtin", ScopeBuiltin),
		cmd("b-project", ScopeProject),
	})
	want := []string{"m-builtin", "b-project", "a-user", "z-skill"}
	if !reflect.DeepEqual(names(got), want) {
		t.Errorf("display order = %v, want %v", names(got), want)
	}
}

func TestResolveCommands_TiesBreakByName(t *testing.T) {
	// Same scope, different names: the picker must be stable, otherwise it
	// reshuffles between renders.
	first := names(ResolveCommands([]CommandInfo{cmd("b", ScopeUser), cmd("a", ScopeUser)}))
	second := names(ResolveCommands([]CommandInfo{cmd("a", ScopeUser), cmd("b", ScopeUser)}))
	if !reflect.DeepEqual(first, second) {
		t.Errorf("unstable ordering: %v vs %v", first, second)
	}
	if first[0] != "a" {
		t.Errorf("tie not broken by name: %v", first)
	}
}

// A lower-precedence command must not erase a higher one just by appearing.
func TestResolveCommands_LowerScopeNeverWins(t *testing.T) {
	for _, weaker := range []CommandScope{ScopeBuiltin, ScopeSkill, ScopeExtension, ScopeProjectFile} {
		got := ResolveCommands([]CommandInfo{cmd("x", ScopeUser), cmd("x", weaker)})
		if len(got) != 1 || got[0].Scope != ScopeUser {
			t.Errorf("%q lost to a lower scope: %+v", weaker, got)
		}
	}
}
