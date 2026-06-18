import { SettingsRow } from "./settings-row";
import { SettingsSection } from "./settings-section";

const shortcuts = [
	{ keys: "⌘N", action: "New Thread", scope: "Global" },
	{ keys: "⌥⌘N", action: "New Chat", scope: "Global" },
	{ keys: "⌘K", action: "Command Palette", scope: "Global" },
	{ keys: "Escape", action: "Go to Home", scope: "Global" },
	{ keys: "J / K", action: "Navigate sessions", scope: "Global" },
	{ keys: "⌘Z", action: "Undo last turn", scope: "Session" },
	{ keys: "⇧⌘Z", action: "Redo turn", scope: "Session" },
	{ keys: "⌘Enter", action: "Submit review comment", scope: "Review" },
	{ keys: "⇧⌘D", action: "Toggle review panel", scope: "Session" },
	{ keys: "⇧⌘F", action: "Toggle review expanded", scope: "Session" },
];

export function KeyboardShortcutsSettings() {
	return (
		<div className="space-y-8">
			<div>
				<h2 className="text-xl font-semibold">Keyboard Shortcuts</h2>
			</div>
			{/* Global shortcuts */}
			<SettingsSection title="Global">
				{shortcuts
					.filter((s) => s.scope === "Global")
					.map((s) => (
						<SettingsRow key={s.keys} label={s.action}>
							<kbd className="inline-flex items-center rounded-md border border-border bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
								{s.keys}
							</kbd>
						</SettingsRow>
					))}
			</SettingsSection>

			{/* Session shortcuts */}
			<SettingsSection title="Session">
				{shortcuts
					.filter((s) => s.scope === "Session")
					.map((s) => (
						<SettingsRow key={s.keys} label={s.action}>
							<kbd className="inline-flex items-center rounded-md border border-border bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
								{s.keys}
							</kbd>
						</SettingsRow>
					))}
			</SettingsSection>

			{/* Review shortcuts */}
			<SettingsSection title="Review">
				{shortcuts
					.filter((s) => s.scope === "Review")
					.map((s) => (
						<SettingsRow key={s.keys} label={s.action}>
							<kbd className="inline-flex items-center rounded-md border border-border bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
								{s.keys}
							</kbd>
						</SettingsRow>
					))}
			</SettingsSection>
		</div>
	);
}
