export type ThemePreference = "system" | "light" | "dark";

export type AppSettings = {
  captureTerminalEvents: boolean;
  retentionDays: number;
  theme: ThemePreference;
  /** Sincronização cloud ativa para o provedor escolhido (opt-in). */
  cloudSyncEnabled: boolean;
  /** Contexto universal entre conversas: arquivo no projeto + injeção validada. */
  contextCarryoverEnabled: boolean;
  /** Provedor de nuvem: "" (nenhum) | "google" | "onedrive". */
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
