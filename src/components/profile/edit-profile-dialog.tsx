import { ImagePlus, Pencil, Trash2 } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { ProfileAvatar } from "@/components/profile/profile-avatar"
import { AvatarImageError, compressAvatarImage } from "@/lib/avatar-image"
import { PROFILE_AVATAR_COLORS } from "@/lib/profile-identity"
import { cn } from "@/lib/utils"

interface EditProfileDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	initials: string
	name: string
	handle: string
	avatarColor: string
	avatarImage: string | null
	onSave: (value: {
		name: string
		handle: string
		avatarColor: string
		avatarImage: string | null
	}) => void
}

export function EditProfileDialog({
	open,
	onOpenChange,
	initials,
	name,
	handle,
	avatarColor,
	avatarImage,
	onSave,
}: EditProfileDialogProps) {
	const [draftName, setDraftName] = useState(name)
	const [draftHandle, setDraftHandle] = useState(handle)
	const [draftColor, setDraftColor] = useState(avatarColor)
	const [draftImage, setDraftImage] = useState<string | null>(avatarImage)
	const [showAvatarEditor, setShowAvatarEditor] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [processing, setProcessing] = useState(false)
	const fileInputRef = useRef<HTMLInputElement>(null)

	useEffect(() => {
		if (!open) return
		setDraftName(name)
		setDraftHandle(handle)
		setDraftColor(avatarColor)
		setDraftImage(avatarImage)
		setShowAvatarEditor(false)
		setError(null)
		setProcessing(false)
	}, [open, name, handle, avatarColor, avatarImage])

	useEffect(() => {
		if (!open) return
		function onKeyDown(event: KeyboardEvent) {
			if (event.key === "Escape") onOpenChange(false)
		}
		window.addEventListener("keydown", onKeyDown)
		return () => window.removeEventListener("keydown", onKeyDown)
	}, [open, onOpenChange])

	async function handlePickFile(file: File | undefined) {
		if (!file) return
		setError(null)
		setProcessing(true)
		try {
			setDraftImage(await compressAvatarImage(file))
		} catch (cause) {
			setError(
				cause instanceof AvatarImageError
					? cause.message
					: "Could not process that image.",
			)
		} finally {
			setProcessing(false)
		}
	}

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
					Photo and colors are stored locally on this machine.
				</p>

				<div className="mt-5 flex flex-col items-center gap-4">
					<div className="relative">
						<ProfileAvatar
							initials={initials}
							color={draftColor}
							image={draftImage}
							className="size-20"
							textClassName="text-2xl"
						/>
						<button
							type="button"
							onClick={() => setShowAvatarEditor((value) => !value)}
							aria-label="Edit avatar"
							className="absolute bottom-0 end-0 flex size-7 items-center justify-center rounded-full bg-black/45 text-white backdrop-blur-sm transition-colors hover:bg-black/60"
						>
							<Pencil className="size-3" />
						</button>
					</div>

					<input
						ref={fileInputRef}
						type="file"
						accept="image/*"
						className="hidden"
						onChange={(event) => {
							void handlePickFile(event.target.files?.[0])
							event.target.value = ""
						}}
					/>

					{showAvatarEditor ? (
						<div className="flex w-full flex-col items-center gap-3">
							<div className="flex flex-wrap items-center justify-center gap-2">
								<button
									type="button"
									disabled={processing}
									onClick={() => fileInputRef.current?.click()}
									className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/60 px-3 text-xs text-foreground transition-colors hover:bg-accent disabled:opacity-50"
								>
									<ImagePlus className="size-3.5" />
									{processing
										? "Processing…"
										: draftImage
											? "Replace photo"
											: "Upload photo"}
								</button>
								{draftImage ? (
									<button
										type="button"
										onClick={() => {
											setDraftImage(null)
											setError(null)
										}}
										className="inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
									>
										<Trash2 className="size-3.5" />
										Remove photo
									</button>
								) : null}
							</div>

							<div className="flex flex-wrap items-center justify-center gap-2">
								{PROFILE_AVATAR_COLORS.map((color) => (
									<button
										key={color}
										type="button"
										aria-label={`Use color ${color}`}
										onClick={() => setDraftColor(color)}
										className={cn(
											"size-6 rounded-full border-2 transition-transform hover:scale-110",
											!draftImage && draftColor === color
												? "scale-110 border-foreground"
												: "border-transparent",
										)}
										style={{ backgroundColor: color }}
									/>
								))}
							</div>

							{draftImage ? (
								<p className="text-center text-xs text-muted-foreground">
									Colors apply when no photo is set.
								</p>
							) : (
								<p className="text-center text-xs text-muted-foreground">
									Pick a color or upload a photo.
								</p>
							)}
						</div>
					) : null}

					{error ? <p className="text-center text-xs text-destructive">{error}</p> : null}
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
						disabled={processing}
						onClick={() => {
							onSave({
								name: draftName.trim() || name,
								handle: draftHandle.trim() || handle,
								avatarColor: draftColor,
								avatarImage: draftImage,
							})
							onOpenChange(false)
						}}
						className="h-8 rounded-md bg-primary px-3 text-xs text-primary-foreground hover:opacity-90 disabled:opacity-50"
					>
						Save
					</button>
				</div>
			</div>
		</div>
	)
}