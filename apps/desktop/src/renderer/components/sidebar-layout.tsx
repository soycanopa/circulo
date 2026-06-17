/**
 * Sidebar shell layout: wraps child routes with the sidebar + SidebarInset chrome.
 * Reads from SidebarSlotContext to allow child routes to override sidebar content.
 */
import { Button } from "@circulo/ui/components/button";
import {
	Sidebar,
	SidebarHeader,
	SidebarInset,
	SidebarProvider,
	useSidebar,
} from "@circulo/ui/components/sidebar";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@circulo/ui/components/tooltip";
import { Outlet, useNavigate } from "@tanstack/react-router";
import { useAtomValue, useSetAtom } from "jotai";
import {
	MessageCircleIcon,
	MessageCirclePlusIcon,
	PanelLeftIcon,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { chatInitDirectoryAtom } from "../atoms/chat";
import { serverConnectedAtom } from "../atoms/connection";
import {
	useAgents,
	useProjectList,
	useSetCommandPaletteOpen,
} from "../hooks/use-agents";
import { useAgentActions } from "../hooks/use-server";
import type { Agent } from "../lib/types";
import { getHomeDir } from "../services/backend";
import { loadProjectSessions } from "../services/connection-manager";
import { APP_BAR_HEIGHT, AppBar } from "./app-bar";
import { AppSidebarContent } from "./sidebar";
import { useSidebarSlot } from "./sidebar-slot-context";
import { UpdateBanner } from "./update-banner";

// ============================================================
// Constants
// ============================================================

const isMac =
	typeof window !== "undefined" &&
	"circulo" in window &&
	window.circulo.platform === "darwin";
const isElectronEnv = typeof window !== "undefined" && "circulo" in window;

/** Pixel offset from the left edge where window controls (toggle + new thread) start */
const WINDOW_CONTROLS_LEFT = isMac && isElectronEnv ? 93 : 8;
/** Total width reserved for traffic lights + window control buttons */
const WINDOW_CONTROLS_INSET = isMac && isElectronEnv ? 160 : 72;

// ============================================================
// NarrowWindowCollapser
// ============================================================

/**
 * Watches the window width and auto-collapses the sidebar when it drops below
 * COLLAPSE_THRESHOLD px, restoring it when the window grows back above the threshold.
 * Must be rendered inside a <SidebarProvider>.
 */
const COLLAPSE_THRESHOLD = 600;

function NarrowWindowCollapser() {
	const { open, setOpen } = useSidebar();
	// Track whether the last collapse was triggered by us (vs. the user manually toggling)
	const collapsedByUsRef = useRef(false);

	useEffect(() => {
		const check = () => {
			const narrow = window.innerWidth < COLLAPSE_THRESHOLD;
			if (narrow && open) {
				collapsedByUsRef.current = true;
				setOpen(false);
			} else if (!narrow && !open && collapsedByUsRef.current) {
				// Only re-open if WE collapsed it — don't override the user's manual close
				collapsedByUsRef.current = false;
				setOpen(true);
			} else if (!narrow) {
				// Window grew back — reset the flag regardless so we don't re-open unexpectedly
				if (!open) collapsedByUsRef.current = false;
			}
		};

		check();
		window.addEventListener("resize", check);
		return () => window.removeEventListener("resize", check);
	}, [open, setOpen]);

	return null;
}

// ============================================================
// WindowControls
// ============================================================

/**
 * Absolutely positioned window controls (sidebar toggle + conditional new thread + new chat)
 * that stay next to the macOS traffic lights regardless of sidebar state.
 * The new-thread and new-chat buttons only appear when the sidebar is collapsed.
 * Must be rendered inside a SidebarProvider.
 */
function WindowControls({ onNavigateChat }: { onNavigateChat?: () => void }) {
	const { toggleSidebar, open } = useSidebar();
	const navigate = useNavigate();

	const handleNewChat = useCallback(() => {
		if (onNavigateChat) {
			onNavigateChat();
		} else {
			navigate({ to: "/" });
		}
	}, [onNavigateChat, navigate]);

	return (
		<div
			className="absolute z-50 flex items-center gap-0.5"
			style={{
				top: 8,
				left: WINDOW_CONTROLS_LEFT,
				// @ts-expect-error -- vendor-prefixed CSS property
				WebkitAppRegion: "no-drag",
			}}
		>
			<Tooltip>
				<TooltipTrigger
					render={
						<Button
							variant="ghost"
							size="icon"
							className="size-7 shrink-0"
							onClick={toggleSidebar}
						/>
					}
				>
					<PanelLeftIcon className="size-3.5" />
				</TooltipTrigger>
				<TooltipContent>Toggle sidebar (&#8984;B)</TooltipContent>
			</Tooltip>
			{!open && (
				<>
					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="ghost"
									size="icon"
									className="size-7 shrink-0"
									onClick={() => navigate({ to: "/" })}
								/>
							}
						>
							<MessageCirclePlusIcon className="size-3.5" />
						</TooltipTrigger>
						<TooltipContent>New thread (&#8984;N)</TooltipContent>
					</Tooltip>
					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="ghost"
									size="icon"
									className="size-7 shrink-0"
									onClick={handleNewChat}
								/>
							}
						>
							<MessageCircleIcon className="size-3.5" />
						</TooltipTrigger>
						<TooltipContent>New Chat</TooltipContent>
					</Tooltip>
				</>
			)}
		</div>
	);
}

// ============================================================
// SidebarResizeHandle
// ============================================================

const SIDEBAR_RESIZE_KEY = "circulo-sidebar-width";
const SIDEBAR_MIN_WIDTH = 200;
const SIDEBAR_MAX_WIDTH = 500;

function getPersistedSidebarWidth(): number | null {
	try {
		const raw = localStorage.getItem(SIDEBAR_RESIZE_KEY);
		if (!raw) return null;
		const val = Number(raw);
		return Number.isFinite(val)
			? Math.max(SIDEBAR_MIN_WIDTH, Math.min(SIDEBAR_MAX_WIDTH, val))
			: null;
	} catch {
		return null;
	}
}

function SidebarResizeHandle({
	sidebarWidth,
	onWidthChange,
}: {
	sidebarWidth: number;
	onWidthChange: (w: number) => void;
}) {
	const draggingRef = useRef(false);
	const startXRef = useRef(0);
	const startWidthRef = useRef(0);
	const [isDragging, setIsDragging] = useState(false);

	const handleMouseDown = useCallback(
		(e: React.MouseEvent) => {
			e.preventDefault();
			draggingRef.current = true;
			startXRef.current = e.clientX;
			startWidthRef.current = sidebarWidth;
			setIsDragging(true);
		},
		[sidebarWidth],
	);

	useEffect(() => {
		if (!isDragging) return;

		const handleMouseMove = (e: MouseEvent) => {
			if (!draggingRef.current) return;
			const delta = e.clientX - startXRef.current;
			const newWidth = Math.max(
				SIDEBAR_MIN_WIDTH,
				Math.min(SIDEBAR_MAX_WIDTH, startWidthRef.current + delta),
			);
			onWidthChange(newWidth);
		};

		const handleMouseUp = () => {
			draggingRef.current = false;
			setIsDragging(false);
			localStorage.setItem(SIDEBAR_RESIZE_KEY, String(sidebarWidth));
		};

		window.addEventListener("mousemove", handleMouseMove);
		window.addEventListener("mouseup", handleMouseUp);
		return () => {
			window.removeEventListener("mousemove", handleMouseMove);
			window.removeEventListener("mouseup", handleMouseUp);
		};
	}, [isDragging, sidebarWidth, onWidthChange]);

	const { state } = useSidebar();
	if (state === "collapsed") return null;

	return (
		<div
			data-slot="sidebar-resize-handle"
			onMouseDown={handleMouseDown}
			className="relative z-20 shrink-0 cursor-col-resize select-none"
			style={{ width: isDragging ? 4 : 2 }}
		>
			{isDragging && (
				<div
					className="absolute inset-y-0 left-0 right-0"
					style={{
						background:
							"linear-gradient(to right, rgba(0,0,0,0), rgba(111,203,243,0.4) 50%, rgba(0,0,0,0))",
					}}
				/>
			)}
		</div>
	);
}

// ============================================================
// SidebarLayout
// ============================================================

export function SidebarLayout() {
	const navigate = useNavigate();
	const { content: slotContent, footer: slotFooter } = useSidebarSlot();

	// ---- Sidebar-specific data ----
	const agents = useAgents();
	const projects = useProjectList();
	const setCommandPaletteOpen = useSetCommandPaletteOpen();
	const { renameSession, deleteSession, forkSession } = useAgentActions();
	const serverConnected = useAtomValue(serverConnectedAtom);

	const setChatInitDirectory = useSetAtom(chatInitDirectoryAtom);

	// Home directory for Chat feature
	const [homeDirectory, setHomeDirectory] = useState<string | null>(null);
	useEffect(() => {
		getHomeDir().then((dir) => setHomeDirectory(dir));
	}, []);

	// Load chat sessions from home directory on startup so they persist across restarts
	const homeSessionsLoadedRef = useRef(false);
	useEffect(() => {
		if (homeDirectory && serverConnected && !homeSessionsLoadedRef.current) {
			homeSessionsLoadedRef.current = true;
			loadProjectSessions(homeDirectory);
		}
	}, [homeDirectory, serverConnected]);

	const [sidebarWidth, setSidebarWidth] = useState(
		() => getPersistedSidebarWidth() ?? 280,
	);
	const handleSidebarWidthChange = useCallback(
		(w: number) => setSidebarWidth(w),
		[],
	);

	// Sub-agents are filtered at the API level (roots: true)
	const visibleAgents = agents;

	const handleRenameSession = useCallback(
		async (agent: Agent, title: string) => {
			await renameSession(agent.directory, agent.sessionId, title);
		},
		[renameSession],
	);

	const handleDeleteSession = useCallback(
		async (agent: Agent) => {
			await deleteSession(agent.directory, agent.sessionId);
		},
		[deleteSession],
	);

	const handleForkSession = useCallback(
		async (agent: Agent) => {
			const forked = await forkSession(agent.directory, agent.sessionId);
			if (forked) {
				navigate({
					to: "/project/$projectSlug/session/$sessionId",
					params: { projectSlug: agent.projectSlug, sessionId: forked.id },
				});
			}
		},
		[forkSession, navigate],
	);

	const handleOpenCommandPalette = useCallback(() => {
		setCommandPaletteOpen(true);
	}, [setCommandPaletteOpen]);

	const handleNavigateChat = useCallback(() => {
		if (homeDirectory) {
			setChatInitDirectory(homeDirectory);
			navigate({ to: "/" });
		}
	}, [navigate, homeDirectory, setChatInitDirectory]);

	return (
		<div
			className="relative flex h-screen text-foreground"
			style={
				{
					"--window-controls-inset": `${WINDOW_CONTROLS_INSET}px`,
				} as React.CSSProperties
			}
		>
			<SidebarProvider
				embedded
				defaultOpen={true}
				style={
					{ "--sidebar-width": `${sidebarWidth}px` } as React.CSSProperties
				}
			>
				<NarrowWindowCollapser />
				<Sidebar collapsible="offcanvas" variant="sidebar">
					{/* Sidebar header */}
					<SidebarHeader
						className="flex-row items-center gap-1 shrink-0"
						style={{
							height: APP_BAR_HEIGHT,
							// @ts-expect-error -- vendor-prefixed CSS property
							WebkitAppRegion: "drag",
						}}
					/>
					{slotContent ?? (
						<AppSidebarContent
							agents={visibleAgents}
							projects={projects}
							onOpenCommandPalette={handleOpenCommandPalette}
							onRenameSession={handleRenameSession}
							onDeleteSession={handleDeleteSession}
							onForkSession={handleForkSession}
							serverConnected={serverConnected}
							homeDirectory={homeDirectory}
							onNavigateChat={handleNavigateChat}
						/>
					)}
					{slotFooter !== false && slotFooter}
				</Sidebar>
				<SidebarResizeHandle
					sidebarWidth={sidebarWidth}
					onWidthChange={handleSidebarWidthChange}
				/>
				<SidebarInset>
					<UpdateBanner />
					<AppBar />
					<div
						data-slot="content-area"
						className="relative min-h-0 min-w-0 flex-1 overflow-hidden"
					>
						<Outlet />
					</div>
				</SidebarInset>
				<WindowControls onNavigateChat={handleNavigateChat} />
			</SidebarProvider>
		</div>
	);
}
