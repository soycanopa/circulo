package arch_test

import (
	"os/exec"
	"strings"
	"testing"
)

func TestBoundaries(t *testing.T) {
	cases := []struct {
		pkg    string
		banned []string
	}{
		{"github.com/soycanopa/circulo/protocol", []string{
			"github.com/egoist/mygo",
			"github.com/soycanopa/circulo/core",
			"github.com/soycanopa/circulo/ui",
		}},
		{"github.com/soycanopa/circulo/core", []string{
			"github.com/egoist/mygo",
			"github.com/soycanopa/circulo/ui",
			"github.com/soycanopa/circulo/providers",
		}},
		{"github.com/soycanopa/circulo/ui/session", []string{
			"github.com/soycanopa/circulo/core",
			"github.com/soycanopa/circulo/providers",
			"github.com/soycanopa/circulo/persistence",
			"github.com/soycanopa/circulo/git",
		}},
		{"github.com/soycanopa/circulo/cmd/circulo", []string{
			"github.com/soycanopa/circulo/core",
			"github.com/soycanopa/circulo/providers",
		}},
		{"github.com/soycanopa/circulo/driver", []string{
			"github.com/egoist/mygo",
			"github.com/soycanopa/circulo/core",
			"github.com/soycanopa/circulo/ui",
			"github.com/soycanopa/circulo/providers",
		}},
		{"github.com/soycanopa/circulo/providers/opencode", []string{
			"github.com/egoist/mygo",
			"github.com/soycanopa/circulo/core",
			"github.com/soycanopa/circulo/ui",
		}},
		{"github.com/soycanopa/circulo/providers/muse", []string{
			"github.com/egoist/mygo",
			"github.com/soycanopa/circulo/core",
			"github.com/soycanopa/circulo/ui",
		}},
	}
	for _, tc := range cases {
		out, err := exec.Command("go", "list", "-deps", "-f", "{{.ImportPath}}", tc.pkg).CombinedOutput()
		if err != nil {
			t.Fatalf("%s: %v\n%s", tc.pkg, err, out)
		}
		deps := strings.Split(string(out), "\n")
		for _, dep := range deps {
			for _, ban := range tc.banned {
				if dep == ban || strings.HasPrefix(dep, ban+"/") {
					t.Errorf("%s imports %s", tc.pkg, dep)
				}
			}
		}
	}
}
