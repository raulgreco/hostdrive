export type SyncOutcome = { status: "disabled" | "first_sync" | "pushed" | "pulled" | "conflict" | "unchanged"; message: string };

/** `true` só quando a senha de app e a frase de criptografia já estão no chaveiro do sistema. Nunca devolve os valores. */
export async function hasNextcloudCredentials() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("has_nextcloud_credentials");
}

/** Grava no chaveiro do sistema operacional — nunca no SQLite, nunca lido de volta pela UI. */
export async function setNextcloudCredentials(appPassword: string, passphrase: string) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("set_nextcloud_credentials", { appPassword, passphrase });
}

export async function clearNextcloudCredentials() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("clear_nextcloud_credentials");
}

export async function testNextcloudConnection() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("test_nextcloud_connection");
}

/** `prefer`: omitido resolve automaticamente quando não há conflito; "local" ou "remote" resolve um conflito já avisado ao usuário. */
export async function syncProjectNow(projectId: string, prefer?: "local" | "remote") {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SyncOutcome>("sync_project_now", { projectId, prefer: prefer ?? null });
}
