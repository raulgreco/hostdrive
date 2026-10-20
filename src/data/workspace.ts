export type Agent = {
  id: string;
  label: string;
  executable?: string;
  version: string | null;
  available: boolean;
  path?: string | null;
  authStatus?: "UNKNOWN" | "REQUIRES_LOGIN" | "AUTHENTICATED" | "ERROR";
  error?: string | null;
  /** REQ-007 — retomada só é oferecida quando o backend confirma suporte verificado. */
  supportsResume?: boolean;
};

export const agents: Agent[] = [
  { id: "codex", label: "Codex", version: "0.154.0", available: true, authStatus: "UNKNOWN", supportsResume: true },
  { id: "claude", label: "Claude", version: "2.1.273", available: true, authStatus: "UNKNOWN", supportsResume: true },
  { id: "opencode", label: "OpenCode", version: "1.18.31", available: true, authStatus: "UNKNOWN", supportsResume: true },
  { id: "kimi", label: "Kimi", version: "0.43.1", available: true, authStatus: "UNKNOWN", supportsResume: true }
];

export const projects = ["HostDrive", "Sistema CPA", "Portal interno"];

export const terminalLines = [
  { kind: "muted", text: "HostDrive — sessão local" },
  { kind: "prompt", text: "~/Projetos/hostdrive  git:(—)  codex" },
  { kind: "plain", text: "Diretório carregado. Nenhum processo de agente iniciado." },
  { kind: "success", text: "✓ Codex 0.154.0 detectado" },
  { kind: "success", text: "✓ Claude 2.1.273 detectado" },
  { kind: "success", text: "✓ OpenCode 1.18.31 detectado" },
  { kind: "success", text: "✓ Kimi 0.43.1 detectado" },
  { kind: "warning", text: "Rust ausente — build Tauri aguardando pré-requisito" },
  { kind: "prompt", text: "Selecione um agente para abrir uma sessão PTY." }
] as const;
