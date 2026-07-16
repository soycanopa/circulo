import type { Transition } from "motion/react"

export const layoutSpring: Transition = {
	type: "spring",
	stiffness: 420,
	damping: 38,
	mass: 0.85,
}

export const panelEase: Transition = {
	duration: 0.28,
	ease: [0.32, 0.72, 0, 1],
}

export const diffPanelSpring: Transition = {
	type: "spring",
	stiffness: 220,
	damping: 30,
	mass: 0.95,
}

export const terminalDrawer: Transition = {
	duration: 0.34,
	ease: [0.32, 0.72, 0, 1],
}

export const fadeSlideUp = {
	initial: { opacity: 0, y: 6 },
	animate: { opacity: 1, y: 0 },
	exit: { opacity: 0, y: 4 },
	transition: { duration: 0.16, ease: [0.22, 1, 0.36, 1] as const },
}