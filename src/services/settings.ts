export type ThemePreference = "system" | "light" | "dark";

export type AppSettings = {
  captureTerminalEvents: boolean;
  retentionDays: number;
  theme: ThemePreference;
  /** Endereço do servidor Nextcloud (ex.: https://nuvem.exemplo.com) — não é segredo. */
  nextcloudUrl: string;
  nextcloudUsername: string;
  nextcloudSyncEnabled: boolean;
  /** Contexto universal entre conversas: arquivo no projeto + injeção validada. */
  contextCarryoverEnabled: boolean;
  /** Provedor de nuvem: "nextcloud" | "google" | "onedrive". */
  cloudProvider: string;
  /** Client IDs OAuth (não secretos) — tokens ficam só no chaveiro. */
  googleClientId: string;
  onedriveClientId: string;
  /** E-mail da conta cloud conectada — a identidade entre máquinas. */
  cloudAccount: string;
};

export async function getAppSettings() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AppSettings>("get_app_settings");
}

export async function updateAppSettings(settings: AppSettings) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AppSettings>("update_app_settings", settings);
}
