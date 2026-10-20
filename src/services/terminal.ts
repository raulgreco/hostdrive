export type TerminalStarted = { sessionId: string; agentId: string; workingDirectory: string; resumed: boolean };
export type TerminalOutput = { sessionId: string; data: number[] };
export type TerminalExited = { sessionId: string };
type TauriWindow = Window & { __TAURI_INTERNALS__?: unknown };

export function isTauriRuntime() { return Boolean((window as TauriWindow).__TAURI_INTERNALS__); }

export async function launchTerminal(agentId: string, workingDirectory: string, cols: number, rows: number, resume = false) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<TerminalStarted>("launch_terminal", { agentId, workingDirectory, cols, rows, resume });
}

export async function sendTerminalInput(sessionId: string, data: string) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("send_terminal_input", { sessionId, data: Array.from(new TextEncoder().encode(data)) });
}

export async function resizeTerminal(sessionId: string, cols: number, rows: number) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("resize_terminal", { sessionId, cols, rows });
}

export async function closeTerminal(sessionId: string) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("close_terminal", { sessionId });
}

export async function listenToTerminal(onOutput: (event: TerminalOutput) => void, onExit: (event: TerminalExited) => void) {
  const { listen } = await import("@tauri-apps/api/event");
  const [stopOutput, stopExit] = await Promise.all([
    listen<TerminalOutput>("terminal-output", (event) => onOutput(event.payload)),
    listen<TerminalExited>("terminal-exited", (event) => onExit(event.payload))
  ]);
  return () => { stopOutput(); stopExit(); };
}
