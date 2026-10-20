import type { Agent } from "../data/workspace";

type TauriWindow = Window & { __TAURI_INTERNALS__?: unknown };

export async function detectInstalledAgents(): Promise<Agent[] | null> {
  if (!(window as TauriWindow).__TAURI_INTERNALS__) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Agent[]>("detect_agents");
}
