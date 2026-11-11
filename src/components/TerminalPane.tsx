import { useEffect, useRef, useState } from "react";
import type { Terminal } from "@xterm/xterm";
import { terminalLines, type Agent } from "../data/workspace";
import { addCustomAgent, type ConnectedAgent } from "../services/agents";
import { closeTerminal, isTauriRuntime, launchTerminal, listenToTerminal, resizeTerminal, sendTerminalInput } from "../services/terminal";
import { OrchestratorBar, type OrchestratorBarHandle } from "./OrchestratorBar";
import "@xterm/xterm/css/xterm.css";

/** Dispara a atualização da lista de agentes instalados no App. */
export const AGENTS_CHANGED_EVENT = "hostdrive:agents-changed";

type Props = {
  /** Agentes instalados descobertos — catálogo + personalizados. */
  agents: Agent[];
  /** Agentes conectados ao projeto — as abas. */
  connectedAgents: ConnectedAgent[];
  workingDirectory: string | null;
  active: boolean;
  onAgentStarted(): void;
};

export function TerminalPane({ agents, connectedAgents, workingDirectory, active, onAgentStarted }: Props) {
  const [activeAgent, setActiveAgent] = useState<string | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [customLabel, setCustomLabel] = useState("");
  const [customExecutable, setCustomExecutable] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "starting" | "running" | "exited" | "error">("idle");
  const [resumedSession, setResumedSession] = useState(false);
  const [hiddenAgents, setHiddenAgents] = useState<Set<string>>(new Set());
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const sessionRef = useRef<string | null>(null);
  const orchestratorRef = useRef<OrchestratorBarHandle>(null);
  const addButtonRef = useRef<HTMLButtonElement>(null);
  const [pickerPos, setPickerPos] = useState<{ top: number; left: number } | null>(null);
  const native = isTauriRuntime();
  const busy = status === "running" || status === "starting";
  const canSwitchAgent = !busy;
  const installedAgent = agents.find((agent) => agent.id === activeAgent);
  const visibleConnected = connectedAgents.filter((agent) => !hiddenAgents.has(agent.id));
  const canResume = canSwitchAgent && Boolean(installedAgent?.supportsResume) && Boolean(workingDirectory);

  // A aba ativa segue os agentes conectados visíveis: se o atual saiu da
  // lista (ou foi fechado) e não está mais instalado, cai no mais recente
  // conectado visível.
  useEffect(() => {
    if (!activeAgent || hiddenAgents.has(activeAgent)) { setActiveAgent(visibleConnected[0]?.id ?? null); return; }
    const stillValid =
      visibleConnected.some((agent) => agent.id === activeAgent) ||
      agents.some((agent) => agent.id === activeAgent && !hiddenAgents.has(agent.id));
    if (!stillValid) setActiveAgent(visibleConnected[0]?.id ?? null);
  }, [connectedAgents, agents, activeAgent, hiddenAgents]);

  useEffect(() => {
    if (!native || !hostRef.current) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    let observer: ResizeObserver | undefined;
    let input: { dispose(): void } | undefined;

    Promise.all([import("@xterm/xterm"), import("@xterm/addon-fit")]).then(async ([{ Terminal }, { FitAddon }]) => {
      if (disposed || !hostRef.current) return;
      const terminal = new Terminal({ cursorBlink: true, fontFamily: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 14, theme: { background: "#0A0D13", foreground: "#E9EDF2", cursor: "#34D6BC", selectionBackground: "#1A2130" } });
      const fit = new FitAddon();
      terminal.loadAddon(fit);
      terminal.open(hostRef.current);
      fit.fit();
      terminal.writeln("\x1b[38;2;52;214;188mEureka42 — PTY pronto\x1b[0m");
      terminalRef.current = terminal;
      input = terminal.onData((data) => { if (sessionRef.current) void sendTerminalInput(sessionRef.current, data); });
      cleanup = await listenToTerminal(
        (event) => { if (event.sessionId === sessionRef.current) terminal.write(new Uint8Array(event.data)); },
        (event) => { if (event.sessionId === sessionRef.current) { sessionRef.current = null; setSessionId(null); setStatus("exited"); } }
      );
      observer = new ResizeObserver(() => { fit.fit(); if (sessionRef.current) void resizeTerminal(sessionRef.current, terminal.cols, terminal.rows); });
      observer.observe(hostRef.current);
    });

    return () => {
      disposed = true; observer?.disconnect(); input?.dispose(); cleanup?.(); terminalRef.current?.dispose(); terminalRef.current = null;
      if (sessionRef.current) void closeTerminal(sessionRef.current);
    };
  }, [native]);

  useEffect(() => {
    if (!active) return;
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLocaleLowerCase() === "k") {
        event.preventDefault();
        orchestratorRef.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active]);

  // Fecha o popover com Escape e ao clicar fora.
  useEffect(() => {
    if (!pickerOpen) return;
    function onKeyDown(event: KeyboardEvent) { if (event.key === "Escape") setPickerOpen(false); }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [pickerOpen]);

  /** Conecta o agente: inicia a sessão de imediato, retomando a última
      sessão nativa quando a CLI validada suporta — escolher o agente já
      é o ato de conectar (sem botão "Retomar"). */
  async function startSession(agentId: string, resume: boolean) {
    const terminal = terminalRef.current;
    if (!terminal || !workingDirectory || !agentId || busy) return;
    setStatus("starting");
    try {
      terminal.reset();
      terminal.writeln("\x1b[38;2;52;214;188mEureka42 — conectando…\x1b[0m");
      const started = await launchTerminal(agentId, workingDirectory, terminal.cols, terminal.rows, resume);
      sessionRef.current = started.sessionId; setSessionId(started.sessionId); setStatus("running"); setResumedSession(started.resumed);
      terminal.writeln(`\x1b[38;2;52;214;188mEureka42 — ${started.resumed ? "sessão retomada" : "agente conectado"}.\x1b[0m`);
      terminal.focus();
      onAgentStarted();
    } catch (error) {
      setStatus("error");
      terminal.writeln(`\r\n\x1b[31m${String(error)}\x1b[0m`);
    }
  }

  async function stopSession() {
    const currentSession = sessionRef.current;
    if (!currentSession) return;
    try {
      await closeTerminal(currentSession);
      terminalRef.current?.writeln("\r\n\x1b[38;2;242;179;61mSessão encerrada pelo usuário.\x1b[0m");
      sessionRef.current = null;
      setSessionId(null);
      setStatus("exited");
    } catch (error) {
      setStatus("error");
      terminalRef.current?.writeln(`\r\n\x1b[31m${String(error)}\x1b[0m`);
    }
  }

  function switchAgent(agentId: string) {
    if (!canSwitchAgent) return "Encerre a sessão atual antes de trocar de agente.";
    setActiveAgent(agentId);
  }

  function resumeFromCommand() {
    if (!canResume || !activeAgent) return "Retomada indisponível para este agente ou sessão em andamento.";
    void startSession(activeAgent, true);
  }

  /** Escolha no picker conecta direto: abre a sessão na hora (retomando
      quando a CLI suporta). */
  function connectInstalled(agentId: string) {
    const agent = agents.find((candidate) => candidate.id === agentId);
    setActiveAgent(agentId);
    unhideAgent(agentId);
    closePicker();
    void startSession(agentId, agent?.supportsResume ?? false);
  }

  async function registerCustom() {
    setAddError(null);
    try {
      const created = await addCustomAgent(customLabel, customExecutable);
      window.dispatchEvent(new CustomEvent(AGENTS_CHANGED_EVENT));
      setPickerOpen(false);
      setCustomLabel("");
      setCustomExecutable("");
      setActiveAgent(created.id);
      unhideAgent(created.id);
      void startSession(created.id, false);
    } catch (error) {
      setAddError(String(error));
    }
  }

  function unhideAgent(agentId: string) {
    setHiddenAgents((current) => {
      if (!current.has(agentId)) return current;
      const next = new Set(current);
      next.delete(agentId);
      return next;
    });
  }

  /** Fecha a aba do agente: encerra a sessão em andamento (quando é a
      aba ativa) e some com a aba — sem apagar histórico algum. */
  async function closeAgentTab(agentId: string) {
    if (agentId === activeAgent && status === "running") await stopSession();
    setHiddenAgents((current) => new Set(current).add(agentId));
  }

  function openPicker() {
    const rect = addButtonRef.current?.getBoundingClientRect();
    if (rect) {
      // Posição fixa: a barra de abas tem overflow-x e cortaria qualquer
      // dropdown posicionado dentro dela.
      const left = Math.min(rect.left, window.innerWidth - 306);
      setPickerPos({ top: rect.bottom + 8, left: Math.max(8, left) });
    }
    setAddError(null);
    setPickerOpen(true);
  }

  function closePicker() {
    setPickerOpen(false);
    setPickerPos(null);
  }

  const unconnectedInstalled = agents.filter((agent) => !connectedAgents.some((connected) => connected.id === agent.id));

  return <main className={active ? "workspace" : "workspace workspace--inactive"} aria-hidden={!active}>
    <div className="agent-tabs" role="tablist" aria-label="Agentes conectados">
      {visibleConnected.map((agent) => {
        const installed = agents.find((candidate) => candidate.id === agent.id);
        const unavailableHere = !installed;
        return <div className="agent-tab-wrap" key={agent.id}>
          <button
            role="tab"
            aria-selected={agent.id === activeAgent}
            disabled={unavailableHere || !canSwitchAgent}
            className={agent.id === activeAgent ? "agent-tab agent-tab--active" : "agent-tab"}
            title={unavailableHere ? `${agent.executable} não está instalado nesta máquina` : agent.executable}
            onClick={() => setActiveAgent(agent.id)}
          >{agent.label}{agent.running ? <span className="agent-tab-dot" aria-label="sessão em andamento" /> : null}{unavailableHere ? <small>ausente</small> : null}</button>
          <button
            className="agent-tab-close"
            aria-label={`Fechar aba de ${agent.label}`}
            title={agent.id === activeAgent && status === "running" ? `Encerrar sessão de ${agent.label} e fechar a aba` : `Fechar aba de ${agent.label}`}
            disabled={agent.id === activeAgent && status === "starting"}
            onClick={() => void closeAgentTab(agent.id)}
          ><svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" /></svg></button>
        </div>;
      })}
      {native ? <div className="agent-add">
        <button ref={addButtonRef} className="agent-add-button" onClick={() => (pickerOpen ? closePicker() : openPicker())} aria-expanded={pickerOpen} disabled={status === "starting"} title={busy ? "Encerre a sessão atual antes de conectar outro agente" : "Escolha um agente para conectar"}>
          <svg width="11" height="11" viewBox="0 0 11 11" aria-hidden="true"><path d="M5.5 1v9M1 5.5h9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" /></svg>
          Escolha seu agente
        </button>
        {pickerOpen && pickerPos ? <div className="agent-picker" style={{ position: "fixed", top: pickerPos.top, left: pickerPos.left, right: "auto" }} role="dialog" aria-label="Conectar agente">
          {unconnectedInstalled.length > 0 ? <><p className="agent-picker-heading">Instalados nesta máquina — clique para conectar</p>{unconnectedInstalled.map((agent) => <button className="agent-picker-row" key={agent.id} onClick={() => connectInstalled(agent.id)}>
            <span><strong>{agent.label}</strong><small>{agent.version ?? "versão desconhecida"}</small></span><span className="agent-picker-action">Conectar</span>
          </button>)}</> : null}
          <p className="agent-picker-heading">Qualquer executável</p>
          <label className="agent-picker-field"><span>Nome</span><input value={customLabel} onChange={(event) => setCustomLabel(event.target.value)} placeholder="Minha CLI" aria-label="Nome do agente" /></label>
          <label className="agent-picker-field"><span>Comando</span><input value={customExecutable} onChange={(event) => setCustomExecutable(event.target.value)} placeholder="mycli" aria-label="Comando executável do agente" onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); void registerCustom(); } }} /></label>
          {addError ? <p className="agent-picker-error" role="alert">{addError}</p> : null}
          <button className="primary-button" onClick={() => void registerCustom()} disabled={!customLabel.trim() || !customExecutable.trim()}>Adicionar e conectar</button>
        </div> : null}
      </div> : null}
      {native && visibleConnected.length === 0 ? <span className="agent-tabs-empty">Nenhum agente conectado — clique em <strong>Escolha seu agente</strong> para começar.</span> : null}
      {native ? status === "running"
        ? <button className="start-session start-session--stop" onClick={stopSession}>Encerrar sessão</button>
        : activeAgent
          ? <div className="start-session-group">
              <button className="start-session" onClick={() => startSession(activeAgent, false)} disabled={!workingDirectory || status === "starting"}>{status === "starting" ? "Conectando…" : "Nova sessão"}</button>
            </div>
          : null
        : null}
    </div>
    <section className="terminal" aria-label="Terminal">
      <div ref={hostRef} className={native ? "xterm-host" : "xterm-host xterm-host--hidden"} />
      {!native ? <div className="terminal-lines">{terminalLines.map((line, index) => <p className={`terminal-line terminal-line--${line.kind}`} key={`${line.text}-${index}`}>{line.text}</p>)}</div> : null}
      {native ? <div className="terminal-footer">
        <span className="session-status" aria-live="polite">{sessionId ? `Sessão ${sessionId.slice(0, 8)}${resumedSession ? " · retomada" : ""}` : status === "error" ? "Falha ao iniciar" : "Sem sessão ativa"}</span>
        <OrchestratorBar ref={orchestratorRef} agents={agents} activeAgentId={activeAgent ?? ""} canSwitch={canSwitchAgent} canResume={canResume} onSwitchAgent={switchAgent} onResume={resumeFromCommand} />
      </div> : <div className="command-bar"><span className="terminal-caret">›</span><span>Execute a versão Tauri para usar o PTY real.</span></div>}
    </section>
  </main>;
}
