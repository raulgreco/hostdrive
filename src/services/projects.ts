export type Project = {
  id: string;
  name: string;
  path: string;
  isFavorite: boolean;
  isArchived: boolean;
  createdAt: number;
  lastOpenedAt: number;
  nextcloudSyncId: string | null;
  lastSyncedAt: number;
  /** Opt-in por projeto — nunca ligado por padrão (pode conter segredos colados no terminal). */
  syncTerminalHistory: boolean;
};

export async function listProjects() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Project[]>("list_projects", { includeArchived: false });
}

export async function chooseAndAddProject() {
  const [{ open }, { invoke }] = await Promise.all([
    import("@tauri-apps/plugin-dialog"),
    import("@tauri-apps/api/core")
  ]);
  const path = await open({ directory: true, multiple: false, title: "Abrir projeto" });
  if (!path) return null;
  return invoke<Project>("add_project", { path });
}

export async function setProjectFavorite(projectId: string, favorite: boolean) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("set_project_favorite", { projectId, favorite });
}

export async function setProjectArchived(projectId: string, archived: boolean) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("set_project_archived", { projectId, archived });
}

export async function setProjectTerminalSync(projectId: string, enabled: boolean) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("set_project_terminal_sync", { projectId, enabled });
}
