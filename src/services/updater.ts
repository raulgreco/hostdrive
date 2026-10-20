import { isTauriRuntime } from "./terminal";

export type UpdateInfo = { version: string; date: string | null; body: string | null };

/**
 * REQ (atualização automática) — consulta `latest.json` publicado nas
 * Releases do GitHub (configurado em `src-tauri/tauri.conf.json` →
 * `plugins.updater.endpoints`), assinado com a chave privada que só existe
 * fora do repositório. Nunca baixa nem instala nada sozinho — isso é sempre
 * uma ação explícita do usuário (`downloadAndInstallUpdate`).
 */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!isTauriRuntime()) return null;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return null;
  return { version: update.version, date: update.date ?? null, body: update.body ?? null };
}

export async function downloadAndInstallUpdate(onProgress?: (downloaded: number, total: number | null) => void) {
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) throw new Error("Nenhuma atualização disponível no momento da instalação.");
  let downloaded = 0;
  let total: number | null = null;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") total = event.data.contentLength ?? null;
    if (event.event === "Progress") { downloaded += event.data.chunkLength; onProgress?.(downloaded, total); }
  });
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}
