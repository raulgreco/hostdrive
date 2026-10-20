export type MemoryEntry = { section: string; content: string; provenance: string; updatedAt: number };
export type ProjectTask = { id: string; title: string; completed: boolean; provenance: string; createdAt: number; updatedAt: number };
export type GitStatus = { available: boolean; branch: string | null; head: string | null; dirty: boolean; changes: string[]; error: string | null };

async function call<T>(command: string, args: Record<string, unknown>) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}
export const listProjectMemory = (projectId: string) => call<MemoryEntry[]>("list_project_memory", { projectId });
export const saveProjectMemory = (projectId: string, section: string, content: string) => call<MemoryEntry>("save_project_memory", { projectId, section, content });
export const listProjectTasks = (projectId: string) => call<ProjectTask[]>("list_project_tasks", { projectId });
export const addProjectTask = (projectId: string, title: string) => call<ProjectTask>("add_project_task", { projectId, title });
export const setProjectTaskCompleted = (taskId: string, completed: boolean) => call<void>("set_project_task_completed", { taskId, completed });
export const getGitStatus = (projectId: string) => call<GitStatus>("get_git_status", { projectId });
