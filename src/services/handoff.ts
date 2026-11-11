export type HandoffPackage = { id: string; status: string; packagePath: string; targetAgent: string; warnings: string[] };
export async function prepareHandoff(projectId: string, sourceAgent: string, targetAgent: string) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<HandoffPackage>("prepare_handoff", { projectId, sourceAgent, targetAgent });
}
