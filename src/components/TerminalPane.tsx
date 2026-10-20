import { useEffect, useRef, useState } from "react";
import type { Terminal } from "@xterm/xterm";
import { terminalLines, type Agent } from "../data/workspace";
import { closeTerminal, isTauriRuntime, launchTerminal, listenToTerminal, resizeTerminal, sendTerminalInput } from "../services/terminal";
import { OrchestratorBar, type OrchestratorBarHandle } from "./OrchestratorBar";
import "@xterm/xterm/css/xterm.css";

export function TerminalPane({ agents, workingDirectory, active }: { agents: Agent[]; workingDirectory: string | null; active: boolean }) {
  const [activeAgent, setActiveAgent] = useState("codex");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "starting" | "running" | "exited" | "error">("idle");
  const [resumedSession, setResumedSession] = useState(false);
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const sessionRef = useRef<string | null>(null);
  const orchestratorRef = useRef<OrchestratorBarHandle>(null);
  const native = isTauriRuntime();
  const canSwitchAgent = status !== "running" && status !== "starting";
  const currentAgent = agents.find((agent) => agent.id === activeAgent);
  const canResume = canSwitchAgent && Boolean(currentAgent?.supportsResume) && Boolean(workingDirectory);

  useEffect(() => {
    if (!native || !hostRef.current) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    let observer: ResizeObserver | undefined;
    let input: { dispose(): void } | undefined;

    Promise.all([import("@xterm/xterm"), import("@xterm/addon-fit")]).then(async ([{ Terminal }, { FitAddon }]) => {
      if (disposed || !hostRef.current) return;
      const terminal = new Terminal({ cursorBlink: true, fontFamily: "SFMono-Regular, Menlo, Consolas, monospace", fontSize: 13, theme: { background: "#050708", foreground: "#d7dfe2", cursor: "#42d7c1" } });
      const fit = new FitAddon();
      terminal.loadAddon(fit);
      terminal.open(hostRef.current);
      fit.fit();
      terminal.writeln("\x1b[38;2;66;215;193mHostDrive — PTY pronto\x1b[0m");
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

  async function startSession(resume: boolean) {
    const terminal = terminalRef.current;
    if (!terminal || !workingDirectory || status === "starting" || status === "running") return;
    setStatus("starting");
    try {
      terminal.reset();
      terminal.writeln(resume ? "\x1b[38;2;66;215;193mHostDrive — retomando última sessão…\x1b[0m" : "\x1b[38;2;66;215;193mHostDrive — iniciando agente…\x1b[0m");
      const started = await launchTerminal(activeAgent, workingDirectory, terminal.cols, terminal.rows, resume);
      sessionRef.current = started.sessionId; setSessionId(started.sessionId); setStatus("running"); setResumedSession(started.resumed); terminal.focus();
    } catch (error) { setStatus("error"); terminal.writeln(`\r\n\x1b[31m${String(error)}\x1b[0m`); }
  }

  async function stopSession() {
    const currentSession = sessionRef.current;
    if (!currentSession) return;
    try {
      await closeTerminal(currentSession);
      terminalRef.current?.writeln("\r\n\x1b[38;2;255;181;71mSessão encerrada pelo usuário.\x1b[0m");
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
    if (!canResume) return "Retomada indisponível para este agente ou sessão em andamento.";
    void startSession(true);
  }

  return <main className={active ? "workspace" : "workspace workspace--inactive"} aria-hidden={!active}>
    <div className="agent-tabs" role="tablist" aria-label="Agentes">
      {agents.map((agent) => <button role="tab" aria-selected={agent.id === activeAgent} disabled={!agent.available || !canSwitchAgent} className={agent.id === activeAgent ? "agent-tab agent-tab--active" : "agent-tab"} key={agent.id} onClick={() => setActiveAgent(agent.id)}>{agent.label}</button>)}
      {native ? status === "running"
        ? <button className="start-session start-session--stop" onClick={stopSession}>Encerrar sessão</button>
        : <div className="start-session-group">
            <button className="start-session" onClick={() => startSession(false)} disabled={!workingDirectory || status === "starting"}>{status === "starting" ? "Abrindo…" : "Nova sessão"}</button>
            {currentAgent?.supportsResume ? <button className="start-session start-session--resume" onClick={() => startSession(true)} disabled={!workingDirectory || status === "starting"} title="Continua a última sessão nativa deste agente neste diretório">Retomar</button> : null}
          </div>
        : null}
    </div>
    <section className="terminal" aria-label="Terminal">
      <div ref={hostRef} className={native ? "xterm-host" : "xterm-host xterm-host--hidden"} />
      {!native ? <div className="terminal-lines">{terminalLines.map((line, index) => <p className={`terminal-line terminal-line--${line.kind}`} key={`${line.text}-${index}`}>{line.text}</p>)}</div> : null}
      {native ? <div className="terminal-footer">
        <span className="session-status" aria-live="polite">{sessionId ? `Sessão ${sessionId.slice(0, 8)}${resumedSession ? " · retomada" : ""}` : status === "error" ? "Falha ao iniciar" : "Sem sessão ativa"}</span>
        <OrchestratorBar ref={orchestratorRef} agents={agents} activeAgentId={activeAgent} canSwitch={canSwitchAgent} canResume={canResume} onSwitchAgent={switchAgent} onResume={resumeFromCommand} />
      </div> : <div className="command-bar"><span className="terminal-caret">›</span><span>Execute a versão Tauri para usar o PTY real.</span></div>}
    </section>
  </main>;
}
