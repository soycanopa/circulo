import { openUrl } from "@tauri-apps/plugin-opener"
import { useAtom } from "jotai"
import { Eye, EyeOff, KeyRound, Loader2 } from "lucide-react"
import { useCallback, useEffect, useMemo, useState } from "react"
import { Button } from "@/components/ui/button"
import { useCredentialHistory } from "@/hooks/use-credential-actions"
import {
	credentialModeLabel,
	emptyCredentialValues,
	validateCredentialValues,
} from "@/lib/credential-presentation"
import { respondCredential } from "@/lib/tauri"
import { cn } from "@/lib/utils"
import {
	activeCredentialAtom,
	messagesAtom,
	promptInFlightAtom,
	sessionStatusAtom,
} from "@/stores/atoms"
import type { CredentialResponseAction } from "@/types/acp"

export function CredentialPrompt() {
	const [credential, setCredential] = useAtom(activeCredentialAtom)
	const [, setSessionStatus] = useAtom(sessionStatusAtom)
	const [, setPromptInFlight] = useAtom(promptInFlightAtom)
	const [, setMessages] = useAtom(messagesAtom)
	const { recordCredentialTurn } = useCredentialHistory()
	const [values, setValues] = useState<Record<string, string>>({})
	const [errors, setErrors] = useState<Record<string, string>>({})
	const [revealed, setRevealed] = useState<Record<string, boolean>>({})
	const [submitting, setSubmitting] = useState(false)

	useEffect(() => {
		if (!credential) return
		setValues(emptyCredentialValues(credential.fields))
		setErrors({})
		setRevealed({})
	}, [credential])

	const handleDecision = useCallback(
		async (action: CredentialResponseAction) => {
			if (!credential || submitting) return
			setSubmitting(true)

			let responseValues: Record<string, string> | undefined
			if (action === "accept" && credential.mode !== "url") {
				const nextErrors = validateCredentialValues(credential.fields, values)
				if (Object.keys(nextErrors).length > 0) {
					setErrors(nextErrors)
					setSubmitting(false)
					return
				}
				responseValues = values
			}

			try {
				if (action === "accept" && credential.mode === "url" && credential.url) {
					await openUrl(credential.url)
				}
				await respondCredential(credential.requestId, { action, values: responseValues })
				recordCredentialTurn(setMessages, credential, responseValues ?? {}, action)
			} catch {
				setSubmitting(false)
				return
			}

			setCredential(null)
			setPromptInFlight(true)
			setSessionStatus("generating")
			setSubmitting(false)
		},
		[
			credential,
			recordCredentialTurn,
			setCredential,
			setMessages,
			setPromptInFlight,
			setSessionStatus,
			submitting,
			values,
		],
	)

	useEffect(() => {
		if (!credential) return

		function onKeyDown(event: KeyboardEvent) {
			if (event.key === "Escape") {
				event.preventDefault()
				void handleDecision("cancel")
			}
		}

		window.addEventListener("keydown", onKeyDown)
		return () => window.removeEventListener("keydown", onKeyDown)
	}, [credential, handleDecision])

	const modeLabel = useMemo(
		() => (credential ? credentialModeLabel(credential.mode) : null),
		[credential],
	)

	if (!credential) return null

	return (
		<div className="px-3 py-3">
			<div className="mb-3 flex items-start gap-2.5">
				<KeyRound className="mt-0.5 size-4 shrink-0 text-[#3B5EF9]" />
				<div className="min-w-0 flex-1">
					<div className="mb-1 flex flex-wrap items-center gap-2">
						<p className="text-sm font-medium text-foreground">{credential.title}</p>
						{modeLabel ? (
							<span className="rounded-md border border-border px-1.5 py-0.5 text-[10px] text-muted-foreground">
								{modeLabel}
							</span>
						) : null}
					</div>
					{credential.description ? (
						<p className="text-xs text-muted-foreground">{credential.description}</p>
					) : null}
					{credential.serviceName ? (
						<p className="mt-1 text-[11px] text-muted-foreground">
							Servicio: {credential.serviceName}
						</p>
					) : null}
				</div>
			</div>

			{credential.mode === "url" && credential.url ? (
				<div className="mb-3 rounded-md border border-popover-border bg-[#222222] px-3 py-2">
					<p className="mb-2 text-xs text-muted-foreground">
						Se abrirá una ventana segura para autorizar el acceso. Revisa la URL antes de
						continuar.
					</p>
					<p className="break-all font-mono text-[11px] text-foreground/90">{credential.url}</p>
				</div>
			) : (
				<div className="mb-3 space-y-2">
					{credential.fields.map((field) => {
						const isSecret = field.secret !== false
						const isVisible = revealed[field.key] ?? false
						return (
							<label key={field.key} className="block">
								<span className="mb-1 block text-xs text-muted-foreground">{field.label}</span>
								<div className="relative">
									<input
										type={isSecret && !isVisible ? "password" : "text"}
										autoComplete="off"
										spellCheck={false}
										value={values[field.key] ?? ""}
										placeholder={field.placeholder}
										disabled={submitting}
										onChange={(event) => {
											const nextValue = event.target.value
											setValues((current) => ({ ...current, [field.key]: nextValue }))
											if (errors[field.key]) {
												setErrors((current) => {
													const next = { ...current }
													delete next[field.key]
													return next
												})
											}
										}}
										className={cn(
											"w-full rounded-md border border-popover-border bg-[#222222] px-3 py-2 pr-9 font-mono text-xs text-foreground outline-none focus:border-[#3B5EF9]/60",
											errors[field.key] && "border-destructive/70",
										)}
									/>
									{isSecret ? (
										<button
											type="button"
											className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
											onClick={() =>
												setRevealed((current) => ({
													...current,
													[field.key]: !isVisible,
												}))
											}
											aria-label={isVisible ? "Ocultar valor" : "Mostrar valor"}
										>
											{isVisible ? (
												<EyeOff className="size-3.5" />
											) : (
												<Eye className="size-3.5" />
											)}
										</button>
									) : null}
								</div>
								{errors[field.key] ? (
									<span className="mt-1 block text-[11px] text-destructive">
										{errors[field.key]}
									</span>
								) : null}
							</label>
						)
					})}
				</div>
			)}

			<div className="flex flex-wrap gap-2">
				{credential.sourceUrl ? (
					<Button
						size="sm"
						variant="secondary"
						disabled={submitting}
						onClick={() => void openUrl(credential.sourceUrl!)}
					>
						Abrir en 1Password
					</Button>
				) : null}
				<Button
					size="sm"
					className="bg-[#3B5EF9] text-white hover:opacity-90"
					disabled={submitting}
					onClick={() => void handleDecision("accept")}
				>
					{submitting ? <Loader2 className="mr-1 size-3.5 animate-spin" /> : null}
					{credential.mode === "url" ? "Autorizar" : "Enviar"}
				</Button>
				<Button
					size="sm"
					variant="secondary"
					disabled={submitting}
					onClick={() => void handleDecision("decline")}
				>
					Rechazar
				</Button>
				<Button
					size="sm"
					variant="ghost"
					disabled={submitting}
					onClick={() => void handleDecision("cancel")}
				>
					Cancelar
				</Button>
			</div>
		</div>
	)
}