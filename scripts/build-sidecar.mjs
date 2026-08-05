// Build the `circulo-mcp` sidecar for bundling.
// Tauri v2 `externalBin` expects `src-tauri/binaries/<name>-<host-triple>`.
// Pass `dev` to build the debug profile instead of release.
import { execSync } from "node:child_process"
import { cpSync, mkdirSync, readFileSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const profile = process.argv[2] === "dev" ? "dev" : "release"
const profileDir = profile === "dev" ? "debug" : "release"
const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const srcTauri = join(root, "src-tauri")
const binName = "circulo-mcp"

execSync(`cargo build ${profile === "dev" ? "" : "--release"} --bin ${binName}`, {
	cwd: srcTauri,
	stdio: "inherit",
})

const meta = JSON.parse(
	execSync("cargo metadata --no-deps --format-version 1", {
		cwd: srcTauri,
		encoding: "utf8",
	}),
)
const targetDir = meta.target_directory

const hostMatch = execSync("rustc -vV").toString().match(/host: (\S+)/)
if (!hostMatch) throw new Error("Could not determine host triple")
const host = hostMatch[1]

const binariesDir = join(srcTauri, "binaries")
mkdirSync(binariesDir, { recursive: true })
const from = join(targetDir, profileDir, binName)
const to = join(binariesDir, `${binName}-${host}`)
cpSync(from, to)
console.log(`sidecar built (${profile}): ${to}`)
