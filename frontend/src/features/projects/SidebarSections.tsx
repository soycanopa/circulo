/**
 * Sidebar chrome — 1:1 replica of the Circulo Paper frame ("Circulo /
 * General"): header with New project + Search rows, sessions list, footer
 * with the settings gear. Width 260px + surfaces live in AppShell.
 */

import { useState } from "react";
import { FolderPlus, MessageSquarePlus, Search, Settings } from "lucide-react";

import { PickFolder } from "@/bindings/circulogo/internal/appservice/dialog";
import { useAppStore } from "@/lib/agent/store";

export function SidebarHeader() {
  const addProject = useAppStore((s) => s.addProject);
  const closeSession = useAppStore((s) => s.closeSession);
  const setSearch = useAppStore((s) => s.setSessionSearch);
  const sessionSearch = useAppStore((s) => s.sessionSearch);
  const [searchOpen, setSearchOpen] = useState(false);
  const [error, setError] = useState("");

  const pickFolder = async () => {
    setError("");
    const path = await PickFolder().catch(() => "");
    if (!path) return;
    try {
      await addProject(path, "managed");
    } catch (e) {
      setError(String(e));
    }
  };

  const closeSearch = () => {
    setSearch("");
    setSearchOpen(false);
  };

  return (
    <div className="flex shrink-0 flex-col gap-2 px-3 py-4">
      {/* Enters new-session mode: closes the session, unlocks the composer's
          project/branch target strip (owner call). */}
      <button
        type="button"
        className="flex items-center gap-1 rounded-md py-1 text-left hover:bg-bg-hover"
        onClick={closeSession}
      >
        <MessageSquarePlus className="size-3.5 shrink-0 text-text-primary" strokeWidth={2} />
        <span className="text-sm/tight text-text-primary">New session</span>
      </button>
      <button
        type="button"
        className="flex items-center gap-1 rounded-md py-1 text-left hover:bg-bg-hover"
        onClick={() => void pickFolder()}
      >
        <FolderPlus className="size-3.5 shrink-0 text-text-primary" strokeWidth={2} />
        <span className="text-sm/tight text-text-primary">New project</span>
      </button>
      {searchOpen ? (
        <div className="flex items-center gap-1 rounded-md py-1">
          <Search className="size-3.5 shrink-0 text-text-primary" strokeWidth={2} />
          <input
            autoFocus
            data-selectable
            value={sessionSearch}
            placeholder="Search sessions"
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") closeSearch();
            }}
            onBlur={() => {
              if (!sessionSearch.trim()) closeSearch();
            }}
            className="w-full bg-transparent text-sm/tight text-text-primary outline-none placeholder:text-text-tertiary"
          />
        </div>
      ) : (
        <button
          type="button"
          className="flex items-center gap-1 rounded-md py-1 text-left hover:bg-bg-hover"
          onClick={() => setSearchOpen(true)}
        >
          <Search className="size-3.5 shrink-0 text-text-primary" strokeWidth={2} />
          <span className="text-sm/tight text-text-primary">Search</span>
        </button>
      )}
      {error && <div className="px-1 text-xs text-danger">{error}</div>}
    </div>
  );
}

export function SidebarFooter() {
  return (
    <div className="flex h-10 w-full shrink-0 items-center px-3">
      <button
        type="button"
        aria-label="Settings"
        title="Settings"
        className="flex h-[28px] w-[28px] shrink-0 items-center justify-center rounded-md text-text-secondary hover:bg-bg-hover"
      >
        <Settings className="size-4" strokeWidth={2} />
      </button>
    </div>
  );
}
