export type ThemePreference = "system" | "light" | "dark";

export type AppSettings = {
  captureTerminalEvents: boolean;
  retentionDays: number;
  theme: ThemePreference;
  /** Endereço do servidor Nextcloud (ex.: https://nuvem.exemplo.com) — não é segredo. */
  nextcloudUrl: string;
  nextcloudUsername: string;
  nextcloudSyncEnabled: boolean;
};

export async function getAppSettings() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AppSettings>("get_app_settings");
}

export async function updateAppSettings(settings: AppSettings) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AppSettings>("update_app_settings", settings);
}
