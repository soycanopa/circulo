/** REST client for the same-origin /agent API (docs/trd.md §4). */

import type {
  AccessMode,
  AccessState,
  HydratedMessage,
  Meta,
  ProjectVcs,
  ProjectView,
  PromptRequest,
  Session,
} from "./protocol";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/agent${path}`, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  if (!res.ok) {
    let detail = `${res.status}`;
    try {
      const body = await res.json();
      if (body?.error) detail = body.error;
    } catch {
      // non-JSON error body; status code is the message
    }
    throw new Error(detail);
  }
  return res.json() as Promise<T>;
}

export const api = {
  listProjects: () => request<ProjectView[]>("/projects"),

  addProject: (path: string, mode: "managed" | "attach", provider: "opencode" | "omp" = "opencode", url?: string) =>
    request<ProjectView>("/projects", {
      method: "POST",
      body: JSON.stringify({ path, mode, provider, url }),
    }),

  removeProject: (projectID: string) =>
    request<{ ok: boolean }>(`/projects/${projectID}`, { method: "DELETE" }),

  listSessions: (projectID: string) =>
    request<Session[]>(`/projects/${projectID}/sessions`),

  createSession: (projectID: string, title?: string, branch?: string) =>
    request<Session>(`/projects/${projectID}/sessions`, {
      method: "POST",
      body: JSON.stringify({ title, branch }),
    }),

  vcs: (projectID: string) => request<ProjectVcs>(`/projects/${projectID}/vcs`),

  branches: (projectID: string) => request<string[]>(`/projects/${projectID}/branches`),

  setBranch: (projectID: string, sessionID: string, branch: string) =>
    request<{ ok: boolean }>(
      `/projects/${projectID}/sessions/${sessionID}/branch`,
      { method: "POST", body: JSON.stringify({ branch }) },
    ),

  renameSession: (projectID: string, sessionID: string, title: string) =>
    request<{ ok: boolean }>(`/projects/${projectID}/sessions/${sessionID}`, {
      method: "PATCH",
      body: JSON.stringify({ title }),
    }),

  deleteSession: (projectID: string, sessionID: string) =>
    request<{ ok: boolean }>(`/projects/${projectID}/sessions/${sessionID}`, {
      method: "DELETE",
    }),

  messages: (projectID: string, sessionID: string, limit = 0) =>
    request<HydratedMessage[]>(
      `/projects/${projectID}/sessions/${sessionID}/messages?limit=${limit}`,
    ),

  prompt: (projectID: string, sessionID: string, req: PromptRequest) =>
    request<{ ok: boolean }>(`/projects/${projectID}/sessions/${sessionID}/prompt`, {
      method: "POST",
      body: JSON.stringify(req),
    }),

  abort: (projectID: string, sessionID: string) =>
    request<{ ok: boolean }>(`/projects/${projectID}/sessions/${sessionID}/abort`, {
      method: "POST",
    }),

  replyPermission: (
    projectID: string,
    sessionID: string,
    permissionID: string,
    response: "once" | "always" | "reject",
  ) =>
    request<{ ok: boolean }>(
      `/projects/${projectID}/sessions/${sessionID}/permissions/${permissionID}`,
      { method: "POST", body: JSON.stringify({ response }) },
    ),

  replyForm: (projectID: string, sessionID: string, formID: string, answer: Record<string, string | string[]>) =>
    request<{ ok: boolean }>(
      `/projects/${projectID}/sessions/${sessionID}/forms/${formID}`,
      { method: "POST", body: JSON.stringify({ answer }) },
    ),

  meta: (projectID: string) => request<Meta>(`/projects/${projectID}/meta`),

  /** Current access mode for a project (GET returns AccessState). */
  access: (projectID: string) =>
    request<AccessState>(`/projects/${projectID}/access`),

  /** Switch a project's access mode (restarts omp with the new flag). */
  setAccess: (projectID: string, mode: AccessMode) =>
    request<{ ok: boolean }>(`/projects/${projectID}/access`, {
      method: "POST",
      body: JSON.stringify({ mode }),
    }),

  openTerminal: (projectID: string) =>
    request<{ id: string }>(`/projects/${projectID}/terminals`, { method: "POST" }),

  closeTerminal: (projectID: string, termId: string) =>
    request<{ ok: boolean }>(
      `/projects/${projectID}/terminals/${termId}`,
      { method: "DELETE" },
    ),

  writeTerminal: (projectID: string, termId: string, data: string) =>
    request<{ ok: boolean }>(
      `/projects/${projectID}/terminals/${termId}/write`,
      { method: "POST", body: JSON.stringify({ data }) },
    ),

  resizeTerminal: (projectID: string, termId: string, cols: number, rows: number) =>
    request<{ ok: boolean }>(
      `/projects/${projectID}/terminals/${termId}/resize`,
      { method: "POST", body: JSON.stringify({ cols, rows }) },
    ),
};
