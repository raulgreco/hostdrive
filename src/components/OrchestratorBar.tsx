import { forwardRef, useImperativeHandle, useRef, useState } from "react";
import type { Agent } from "../data/workspace";

export type OrchestratorCommand =
  | { kind: "switch"; agentId: string }
  | { kind: "resume" }
  | { kind: "help" }
  | { kind: "unknown"; command: string }
  | { kind: "none" };

/**
 * REQ-010 — interpreta comandos digitados na barra do orquestrador.
 * Puro e isolado da UI para ser testável e para nunca tocar o fluxo de
 * bytes do PTY: comandos fora do padrão `///...` são `{ kind: "none" }`,
 * nunca repassados ao agente (ver T-018 em docs/09-TESTES-E-ACEITE.md).
 */
export function parseOrchestratorCommand(raw: string): OrchestratorCommand {
  const trimmed = raw.trim();
  if (!trimmed.startsWith("///")) return { kind: "none" };
  const [command, ...rest] = trimmed.slice(3).trim().split(/\s+/).filter(Boolean);
  if (!command) return { kind: "help" };
  const normalized = command.toLocaleLowerCase();
  if (normalized === "agente" || normalized === "agent") return { kind: "switch", agentId: (rest[0] ?? "").toLocaleLowerCase() };
  if (normalized === "retomar" || normalized === "resume" || normalized === "continuar") return { kind: "resume" };
  if (normalized === "ajuda" || normalized === "help" || normalized === "?") return { kind: "help" };
  return { kind: "unknown", command: normalized };
}

export type OrchestratorBarHandle = { focus(): void };

type Props = {
  agents: Agent[];
  activeAgentId: string;
  canSwitch: boolean;
  canResume: boolean;
  onSwitchAgent(agentId: string): string | void;
  onResume(): string | void;
};

const HELP_TEXT = "Comandos: ///agente <id> troca o agente da aba (encerre a sessão antes); ///retomar continua a última sessão nativa; ///ajuda lista estes comandos.";

export const OrchestratorBar = forwardRef<OrchestratorBarHandle, Props>(function OrchestratorBar(
  { agents, activeAgentId, canSwitch, canResume, onSwitchAgent, onResume },
  ref
) {
  const [value, setValue] = useState("");
  const [feedback, setFeedback] = useState<string | null>(null);
  const [highlight, setHighlight] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useImperativeHandle(ref, () => ({
    focus() { inputRef.current?.focus(); inputRef.current?.select(); }
  }));

  const paletteMatch = /^\/\/\/(agente|agent)\b\s*(.*)$/i.exec(value.trimStart());
  const suggestions = paletteMatch
    ? agents.filter((agent) => agent.available && agent.id !== activeAgentId && `${agent.id} ${agent.label}`.toLocaleLowerCase().includes(paletteMatch[2].trim().toLocaleLowerCase()))
    : [];

  function applySwitch(agentId: string) {
    if (!canSwitch) { setFeedback("Encerre a sessão atual antes de trocar de agente."); return; }
    const agent = agents.find((candidate) => candidate.id === agentId);
    if (!agent) { setFeedback(`Agente "${agentId}" não reconhecido.`); return; }
    if (!agent.available) { setFeedback(`${agent.label} não está instalado neste computador.`); return; }
    const error = onSwitchAgent(agentId);
    setFeedback(error ?? `Agente ativo: ${agent.label}. Use "Iniciar sessão" para abrir.`);
  }

  function runCommand() {
    const command = parseOrchestratorCommand(value);
    if (command.kind === "none") { setFeedback(value.trim() ? "Comandos do orquestrador começam com ///. Digite no terminal para enviar ao agente." : null); return; }
    if (command.kind === "help") { setFeedback(HELP_TEXT); return; }
    if (command.kind === "unknown") { setFeedback(`Comando "///${command.command}" desconhecido. ${HELP_TEXT}`); return; }
    if (command.kind === "resume") {
      if (!canResume) { setFeedback("Retomada indisponível: agente sem suporte verificado ou sessão já em andamento."); return; }
      const error = onResume();
      setFeedback(error ?? "Retomando a última sessão nativa…");
      return;
    }
    if (command.kind === "switch") {
      if (!command.agentId) return; // só abriu a paleta; nada a confirmar ainda.
      applySwitch(command.agentId);
    }
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (suggestions.length > 0 && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      setHighlight((current) => {
        const delta = event.key === "ArrowDown" ? 1 : -1;
        return (current + delta + suggestions.length) % suggestions.length;
      });
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (suggestions.length > 0) { applySwitch(suggestions[highlight].id); setValue(""); return; }
      runCommand();
      setValue("");
      return;
    }
    if (event.key === "Escape") { setValue(""); setFeedback(null); }
  }

  return <div className="orchestrator-bar">
    <div className="orchestrator-input-row">
      <span className="orchestrator-caret" aria-hidden="true">///</span>
      <input
        ref={inputRef}
        value={value}
        onChange={(event) => { setValue(event.target.value); setHighlight(0); }}
        onKeyDown={onKeyDown}
        placeholder="agente <id> · retomar · ajuda — (⌘K/Ctrl+K)"
        aria-label="Comando do orquestrador"
        autoComplete="off"
      />
    </div>
    {suggestions.length > 0 ? <ul className="orchestrator-palette" role="listbox" aria-label="Agentes disponíveis">
      {suggestions.map((agent, index) => <li key={agent.id} role="option" aria-selected={index === highlight} className={index === highlight ? "orchestrator-option orchestrator-option--active" : "orchestrator-option"} onMouseEnter={() => setHighlight(index)} onMouseDown={(event) => { event.preventDefault(); applySwitch(agent.id); setValue(""); }}>
        <strong>{agent.label}</strong><small>{agent.version ?? "versão desconhecida"}</small>
      </li>)}
    </ul> : null}
    {feedback ? <p className="orchestrator-feedback" role="status">{feedback}</p> : null}
  </div>;
});
