export type SyncOutcome = {
  status: "disabled" | "first_sync" | "pushed" | "pulled" | "conflict" | "unchanged";
  message: string;
  /** Em conflito de arquivos: caminhos relativos afetados. */
  conflictFiles?: string[];
};

/** Estado da conexão cloud — a conta (e-mail) é a identidade entre máquinas. */
export type CloudStatus = {
  provider: string;
  providerLabel: string;
  syncEnabled: boolean;
  configured: boolean;
  account: string;
  hasPassphrase: boolean;
};

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

export async function getCloudStatus() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudStatus>("get_cloud_status");
}

/** Abre o navegador, espera o login OAuth e retorna o e-mail da conta conectada. */
export async function connectCloud() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("connect_cloud");
}

export async function disconnectCloud() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("disconnect_cloud");
}

export async function testCloudConnection() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("test_cloud_connection");
}

/** `prefer`/`filesPrefer`: omitidos resolvem automaticamente quando não há conflito; "local" ou "remote" resolvem um conflito já avisado ao usuário. */
export async function syncProjectNow(projectId: string, prefer?: "local" | "remote", filesPrefer?: "local" | "remote") {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SyncOutcome>("sync_project_now", { projectId, prefer: prefer ?? null, filesPrefer: filesPrefer ?? null });
}
