import { useEffect, useState } from "react"
import { ProfileAvatar } from "@/components/profile/profile-avatar"
import { PROFILE_AVATAR_COLORS } from "@/lib/profile-identity"
import { cn } from "@/lib/utils"

interface EditProfileDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	initials: string
	name: string
	handle: string
	avatarColor: string
	onSave: (value: { name: string; handle: string; avatarColor: string }) => void
}

export function EditProfileDialog({
	open,
	onOpenChange,
	initials,
	name,
	handle,
	avatarColor,
	onSave,
}: EditProfileDialogProps) {
	const [draftName, setDraftName] = useState(name)
	const [draftHandle, setDraftHandle] = useState(handle)
	const [draftColor, setDraftColor] = useState(avatarColor)

	useEffect(() => {
		if (!open) return
		setDraftName(name)
		setDraftHandle(handle)
		setDraftColor(avatarColor)
	}, [open, name, handle, avatarColor])

	useEffect(() => {
		if (!open) return
		function onKeyDown(event: KeyboardEvent) {
			if (event.key === "Escape") onOpenChange(false)
		}
		window.addEventListener("keydown", onKeyDown)
		return () => window.removeEventListener("keydown", onKeyDown)
	}, [open, onOpenChange])

	if (!open) return null

	return (
		<div className="fixed inset-0 z-[80] flex items-center justify-center p-4">
			<button
				type="button"
				aria-label="Close"
				className="absolute inset-0 bg-black/50"
				onClick={() => onOpenChange(false)}
			/>
			<div className="relative z-10 w-full max-w-md rounded-xl border border-border/60 bg-card p-5 shadow-xl">
				<h2 className="text-sm font-medium text-foreground">Edit profile</h2>
				<p className="mt-1 text-xs text-muted-foreground">
					Stored locally on this machine.
				</p>

				<div className="mt-5 flex flex-col items-center gap-4">
					<ProfileAvatar
						initials={initials}
						color={draftColor}
						className="size-14"
						textClassName="text-lg"
					/>
					<div className="flex flex-wrap justify-center gap-2">
						{PROFILE_AVATAR_COLORS.map((color) => (
							<button
								key={color}
								type="button"
								aria-label={`Avatar color ${color}`}
								onClick={() => setDraftColor(color)}
								className={cn(
									"size-6 rounded-full border-2 transition-transform",
									draftColor === color
										? "scale-110 border-foreground"
										: "border-transparent",
								)}
								style={{ backgroundColor: color }}
							/>
						))}
					</div>
				</div>

				<div className="mt-5 space-y-3">
					<label className="block space-y-1.5">
						<span className="text-xs text-muted-foreground">Name</span>
						<input
							value={draftName}
							onChange={(event) => setDraftName(event.target.value)}
							className="h-9 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
						/>
					</label>
					<label className="block space-y-1.5">
						<span className="text-xs text-muted-foreground">Handle</span>
						<input
							value={draftHandle}
							onChange={(event) => setDraftHandle(event.target.value)}
							className="h-9 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
						/>
					</label>
				</div>

				<div className="mt-6 flex justify-end gap-2">
					<button
						type="button"
						onClick={() => onOpenChange(false)}
						className="h-8 rounded-md border border-border px-3 text-xs text-foreground hover:bg-accent"
					>
						Cancel
					</button>
					<button
						type="button"
						onClick={() => {
							onSave({
								name: draftName.trim() || name,
								handle: draftHandle.trim() || handle,
								avatarColor: draftColor,
							})
							onOpenChange(false)
						}}
						className="h-8 rounded-md bg-primary px-3 text-xs text-primary-foreground hover:opacity-90"
					>
						Save
					</button>
				</div>
			</div>
		</div>
	)
}