export type SessionSummary = { id: string; projectId: string; agentId: string; workingDirectory: string; status: string; startedAt: number; endedAt: number | null; eventCount: number; capturedBytes: number };

export type SessionFilters = { agentId?: string; status?: string; query?: string; startedAfter?: number };
export type TerminalEvent = { id: number; direction: string; text: string; createdAt: number };
export type ExportResult = { path: string; format: string; redactionApplied: boolean };

export async function listTerminalSessions(projectId: string, filters: SessionFilters = {}, limit = 50, offset = 0) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SessionSummary[]>("list_terminal_sessions", { projectId, limit, offset, agentId: filters.agentId || null, status: filters.status || null, query: filters.query || null, startedAfter: filters.startedAfter || null });
}

export async function getTerminalSessionEvents(sessionId: string, limit = 200, offset = 0) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<TerminalEvent[]>("get_terminal_session_events", { sessionId, limit, offset });
}

export async function exportTerminalSession(sessionId: string, format: "md" | "json" | "txt") {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ExportResult>("export_terminal_session", { sessionId, format });
}
