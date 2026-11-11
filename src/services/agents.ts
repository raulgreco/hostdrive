import type { Agent } from "../data/workspace";

type TauriWindow = Window & { __TAURI_INTERNALS__?: unknown };

/** Agente com ao menos uma sessão no projeto — os "conectados". */
export type ConnectedAgent = {
  id: string;
  label: string;
  executable: string;
  sessionCount: number;
  lastUsedAt: number;
  running: boolean;
};

export async function detectInstalledAgents(): Promise<Agent[] | null> {
  if (!(window as TauriWindow).__TAURI_INTERNALS__) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Agent[]>("detect_agents");
}

export async function listProjectAgents(projectId: string): Promise<ConnectedAgent[]> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ConnectedAgent[]>("list_project_agents", { projectId });
}

export async function addCustomAgent(label: string, executable: string): Promise<Agent> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Agent>("add_custom_agent", { label, executable });
}

export async function removeCustomAgent(id: string): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("remove_custom_agent", { id });
}
