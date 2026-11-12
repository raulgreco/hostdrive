import { useCallback, useEffect, useRef, useState } from "react";
import { Sidebar, type ProjectHistoryStats } from "./components/Sidebar";
import { WindowControls } from "./components/WindowControls";
import { StatusDot } from "./components/StatusDot";
import { TerminalWorkspace } from "./components/TerminalWorkspace";
import { HistoryPane } from "./components/HistoryPane";
import { SettingsDialog } from "./components/SettingsDialog";
import { MemoryPane } from "./components/MemoryPane";
import { TasksPane } from "./components/TasksPane";
import { GitPane } from "./components/GitPane";
import { HandoffDialog } from "./components/HandoffDialog";
import { UpdateBanner } from "./components/UpdateBanner";
import { AboutDialog } from "./components/AboutDialog";
import { SplashScreen } from "./components/SplashScreen";
import logoEscuro from "./assets/brand/logo-escuro.svg";
import logoClaro from "./assets/brand/logo-claro.svg";
import { agents as fallbackAgents } from "./data/workspace";
import { detectInstalledAgents, listProjectAgents, type ConnectedAgent } from "./services/agents";
import { chooseAndAddProject, listProjects, setProjectArchived, setProjectFavorite, setProjectFilesSync, setProjectTerminalSync, type Project } from "./services/projects";
import { getAppSettings, type AppSettings, type ThemePreference } from "./services/settings";
import { getCloudStatus, syncProjectNow } from "./services/sync";
import { isTauriRuntime } from "./services/terminal";
import { checkForUpdate, type UpdateInfo } from "./services/updater";

/** NFR-006 — aplica a preferência de tema e devolve o valor resolvido
    ("light"/"dark") para quem precisa trocar ativos por tema. */
function useAppliedTheme(preference: ThemePreference): "light" | "dark" {
  const [resolved, setResolved] = useState<"light" | "dark">("dark");
  useEffect(() => {
    const root = document.documentElement;
    if (preference !== "system") { root.dataset.theme = preference; setResolved(preference); return; }
    const media = window.matchMedia("(prefers-color-scheme: light)");
    const apply = () => { const value = media.matches ? "light" : "dark"; root.dataset.theme = value; setResolved(value); };
    apply();
    media.addEventListener("change", apply);
    return () => { delete root.dataset.theme; media.removeEventListener("change", apply); };
  }, [preference]);
  return resolved;
}

/** Baralho de Chips: as ferramentas do projeto são fichas de cor indexadas
    na identidade Eureka42. A ficha ativa sai do baralho e sua cor carimba
    o filete da área de trabalho. */
const UTILITY_CHIPS = [
  { id: "Terminal", code: "T-01", color: "#34D6BC", ink: "#0A0D13" },
  { id: "Histórico", code: "H-02", color: "#8B7CF6", ink: "#0A0D13" },
  { id: "Memória", code: "M-03", color: "#F2B33D", ink: "#0A0D13" },
  { id: "Git", code: "G-04", color: "#0E8A77", ink: "#FFFFFF" },
  { id: "Tarefas", code: "K-05", color: "#9A6406", ink: "#FFFFFF" },
] as const;

export function App() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [utility, setUtility] = useState<string>("Terminal");
  const [agents, setAgents] = useState(fallbackAgents);
  const [connectedAgents, setConnectedAgents] = useState<ConnectedAgent[]>([]);
  const [projectError, setProjectError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [handoffOpen, setHandoffOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [theme, setTheme] = useState<ThemePreference>("dark");
  const [cloudSyncEnabled, setCloudSyncEnabled] = useState(false);
  const [cloudLabel, setCloudLabel] = useState("Local");
  const [syncStatus, setSyncStatus] = useState<string | null>(null);
  const [syncConflict, setSyncConflict] = useState<{ projectId: string; message: string; files: string[] } | null>(null);
  const [availableUpdate, setAvailableUpdate] = useState<UpdateInfo | null>(null);
  const selectedProject = projects.find((project) => project.id === selectedProjectId) ?? null;
  const resolvedTheme = useAppliedTheme(theme);
  const activeChip = UTILITY_CHIPS.find((chip) => chip.id === utility) ?? UTILITY_CHIPS[0];

  const previousProjectRef = useRef<string | null>(null);
  const selectedProjectIdRef = useRef<string | null>(null);
  selectedProjectIdRef.current = selectedProjectId;

  const applyCloudSettings = useCallback((settings: AppSettings) => {
    setTheme(settings.theme);
    setCloudSyncEnabled(settings.nextcloudSyncEnabled);
  }, []);

  useEffect(() => {
    let active = true;
    Promise.all([
      detectInstalledAgents(),
      isTauriRuntime() ? listProjects() : Promise.resolve([]),
      isTauriRuntime() ? getAppSettings().catch(() => null) : Promise.resolve(null)
    ]).then(([detected, stored, settings]) => {
      if (!active) return;
      if (detected) setAgents(detected);
      setProjects(stored); setSelectedProjectId(stored[0]?.id ?? null);
      if (settings) applyCloudSettings(settings);
    }).catch((error) => { if (active) setProjectError(String(error)); });
    return () => { active = false; };
  }, [applyCloudSettings]);

  // Agentes conectados do projeto selecionado — só aparecem os que já foram
  // usados em alguma sessão; recarrega ao trocar de projeto.
  useEffect(() => {
    if (!isTauriRuntime() || !selectedProjectId) { setConnectedAgents([]); return; }
    let active = true;
    listProjectAgents(selectedProjectId)
      .then((rows) => { if (active) setConnectedAgents(rows); })
      .catch(() => { if (active) setConnectedAgents([]); });
    return () => { active = false; };
  }, [selectedProjectId]);

  const refreshConnectedAgents = useCallback(() => {
    if (!isTauriRuntime() || !selectedProjectIdRef.current) return;
    listProjectAgents(selectedProjectIdRef.current)
      .then(setConnectedAgents)
      .catch(() => undefined);
  }, []);

  // Histórico por projeto na sidebar: total de sessões e última atividade
  // de cada projeto (agregado dos agentes conectados dele). Recarrega
  // quando a lista de projetos muda e quando sessões são iniciadas.
  const [projectStats, setProjectStats] = useState<Record<string, ProjectHistoryStats>>({});
  useEffect(() => {
    if (!isTauriRuntime() || projects.length === 0) { setProjectStats({}); return; }
    let active = true;
    Promise.all(projects.map((project) =>
      listProjectAgents(project.id)
        .then((rows): [string, ProjectHistoryStats] => {
          const last = rows.reduce((latest, row) => (row.lastUsedAt > latest.lastUsedAt ? row : latest), rows[0] ?? { lastUsedAt: 0, label: "" });
          return [project.id, { sessions: rows.reduce((total, row) => total + row.sessionCount, 0), lastLabel: last.label ?? "", lastUsedAt: last.lastUsedAt }];
        })
        .catch((): [string, ProjectHistoryStats] => [project.id, { sessions: 0, lastLabel: "", lastUsedAt: 0 }])
    )).then((entries) => { if (active) setProjectStats(Object.fromEntries(entries)); });
    return () => { active = false; };
  }, [projects, connectedAgents]);

  useEffect(() => {
    function refreshDetectedAgents() {
      detectInstalledAgents().then((detected) => { if (detected) setAgents(detected); }).catch(() => undefined);
    }
    window.addEventListener("hostdrive:agents-changed", refreshDetectedAgents);
    return () => window.removeEventListener("hostdrive:agents-changed", refreshDetectedAgents);
  }, []);

  // Identidade cloud no título: provedor + conta (ou "Local" sem nuvem).
  useEffect(() => {
    if (!isTauriRuntime()) return;
    let active = true;
    getCloudStatus().then((cloud) => {
      if (!active) return;
      setCloudLabel(cloud.syncEnabled
        ? (cloud.account ? `${cloud.providerLabel} · ${cloud.account}` : cloud.providerLabel)
        : "Local");
    }).catch(() => undefined);
    return () => { active = false; };
  }, [settingsOpen, selectedProjectId]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key === ",") { event.preventDefault(); setSettingsOpen(true); }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  // Checagem de atualização: só consulta o manifesto assinado nas Releases
  // do GitHub e avisa — nunca baixa/instala nada sem o usuário clicar.
  useEffect(() => {
    let active = true;
    checkForUpdate().then((update) => { if (active && update) setAvailableUpdate(update); }).catch(() => undefined);
    return () => { active = false; };
  }, []);

  // Padrão macOS: o botão fechar ESCONDE a janela — o app continua vivo na
  // Dock (clique no ícone reabre, ver Reopen no Rust) e ⌘Q encerra de
  // verdade. Fora do macOS, fechar continua encerrando a janela normalmente.
  useEffect(() => {
    if (!isTauriRuntime() || !navigator.userAgent.includes("Mac")) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    import("@tauri-apps/api/window").then(async ({ getCurrentWindow }) => {
      if (disposed) return;
      const win = getCurrentWindow();
      unlisten = await win.onCloseRequested(async (event) => {
        event.preventDefault();
        await win.hide().catch(() => undefined);
      });
    });
    return () => { disposed = true; unlisten?.(); };
  }, []);

  // Sincronização automática: ao trocar de projeto, envia o que ficou para
  // trás e traz o que há de mais novo do que foi aberto. Nunca sobrescreve
  // silenciosamente em caso de conflito — ver docs/decisions/005 e 007.
  // Na abertura do app tudo é silencioso (best-effort): erro de nuvem não
  // pode ser o primeiro greeting do usuário a cada lançamento.
  const initialSyncRef = useRef(false);
  useEffect(() => {
    const previous = previousProjectRef.current;
    previousProjectRef.current = selectedProjectId;
    if (!isTauriRuntime() || !cloudSyncEnabled) return;
    const isStartup = !initialSyncRef.current;
    initialSyncRef.current = true;
    let active = true;
    (async () => {
      if (previous && previous !== selectedProjectId) {
        await syncProjectNow(previous).catch(() => undefined);
      }
      if (!selectedProjectId) return;
      try {
        const outcome = await syncProjectNow(selectedProjectId);
        if (!active || isStartup) return;
        if (outcome.status === "conflict") setSyncConflict({ projectId: selectedProjectId, message: outcome.message, files: outcome.conflictFiles ?? [] });
        else if (outcome.status !== "disabled") setSyncStatus(outcome.message);
      } catch (error) { if (active && !isStartup) setSyncStatus(String(error)); }
    })();
    return () => { active = false; };
  }, [selectedProjectId, cloudSyncEnabled]);

  // Fechamento nativo, sem interceptação: interceptar o close-requested
  // exigia preventDefault + destroy() via IPC (latência perceptível e,
  // na prática, o botão parecia morto). A sincronização continua automática
  // ao trocar de projeto — nenhum dado fica preso no fechamento.

  async function resolveSyncConflict(prefer: "local" | "remote") {
    if (!syncConflict) return;
    try {
      // Conflito de dados usa `prefer`; conflito de arquivos (lista não
      // vazia sem conflito de dados) usa `filesPrefer`.
      const isFilesOnly = syncConflict.files.length > 0 && !syncConflict.message.includes("duas máquinas");
      const outcome = isFilesOnly
        ? await syncProjectNow(syncConflict.projectId, undefined, prefer)
        : await syncProjectNow(syncConflict.projectId, prefer);
      setSyncStatus(outcome.message);
    }
    catch (error) { setSyncStatus(String(error)); }
    setSyncConflict(null);
  }

  async function toggleTerminalHistorySync(project: Project) {
    const enabled = !project.syncTerminalHistory;
    try { await setProjectTerminalSync(project.id, enabled); setProjects((current) => current.map((item) => item.id === project.id ? { ...item, syncTerminalHistory: enabled } : item)); }
    catch (error) { setProjectError(String(error)); }
  }

  async function toggleFilesSync(project: Project) {
    const enabled = !project.filesSyncEnabled;
    try { await setProjectFilesSync(project.id, enabled); setProjects((current) => current.map((item) => item.id === project.id ? { ...item, filesSyncEnabled: enabled } : item)); }
    catch (error) { setProjectError(String(error)); }
  }

  function onSettingsSaved(settings: AppSettings) { applyCloudSettings(settings); }

  async function addProject() {
    try { const project = await chooseAndAddProject(); if (!project) return; setProjects((current) => [project, ...current.filter((item) => item.id !== project.id)]); setSelectedProjectId(project.id); setProjectError(null); }
    catch (error) { setProjectError(String(error)); }
  }
  async function toggleFavorite(project: Project) {
    const favorite = !project.isFavorite;
    try { await setProjectFavorite(project.id, favorite); setProjects((current) => current.map((item) => item.id === project.id ? { ...item, isFavorite: favorite } : item).toSorted((a, b) => Number(b.isFavorite) - Number(a.isFavorite) || b.lastOpenedAt - a.lastOpenedAt)); }
    catch (error) { setProjectError(String(error)); }
  }
  async function archiveProject(project: Project) {
    try { await setProjectArchived(project.id, true); setProjects((current) => current.filter((item) => item.id !== project.id)); if (selectedProjectId === project.id) setSelectedProjectId(null); }
    catch (error) { setProjectError(String(error)); }
  }

  return <div className="app-shell">
    <SplashScreen />
    <header className="titlebar" data-tauri-drag-region>
      <WindowControls />
      <img src={resolvedTheme === "light" ? logoClaro : logoEscuro} alt="Eureka42" className="titlebar-logo" />
      <div className="titlebar-status">
        <span className="titlebar-cloud"><StatusDot />{cloudLabel}</span>
        <span className="titlebar-agents">{connectedAgents.length > 0 ? `${connectedAgents.length} ${connectedAgents.length === 1 ? "agente conectado" : "agentes conectados"}` : `${agents.filter((agent) => agent.available).length} agentes instalados`}</span>
        <button className="titlebar-button" onClick={() => setAboutOpen(true)}>Sobre</button>
        <button className="icon-button" aria-label="Configurações" title="Configurações (⌘,)" onClick={() => setSettingsOpen(true)}>
          <svg width="17" height="17" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M6.5 1.5h3l.5 1.7a5 5 0 0 1 1.5.9l1.7-.6 1.5 2.6-1.3 1.2a5 5 0 0 1 0 1.4l1.3 1.2-1.5 2.6-1.7-.6a5 5 0 0 1-1.5.9l-.5 1.7h-3l-.5-1.7a5 5 0 0 1-1.5-.9l-1.7.6-1.5-2.6L2.6 9.5a5 5 0 0 1 0-1.4L1.3 6.9l1.5-2.6 1.7.6a5 5 0 0 1 1.5-.9z" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinejoin="round" />
            <circle cx="8" cy="8" r="2.1" fill="none" stroke="currentColor" strokeWidth="1.2" />
          </svg>
        </button>
      </div>
    </header>
    <Sidebar agents={connectedAgents} hasProject={selectedProject !== null} projectStats={projectStats} projects={projects} selectedProjectId={selectedProjectId} onProjectSelect={(project) => setSelectedProjectId(project.id)} onAddProject={addProject} onFavorite={toggleFavorite} onArchive={archiveProject} />
    <nav className="chip-rail" aria-label="Ferramentas do projeto">
      {UTILITY_CHIPS.map((chip) => <button
        key={chip.id}
        className={chip.id === utility ? "chip chip--active" : "chip"}
        style={{ "--chip": chip.color, "--chip-ink": chip.ink } as React.CSSProperties}
        aria-pressed={chip.id === utility}
        aria-label={chip.id}
        onClick={() => setUtility(chip.id)}
      >
        <span className="chip-card"><b>{chip.code}</b></span>
        <span className="chip-label">{chip.id.toUpperCase()}</span>
      </button>)}
    </nav>
    <section className="project-header"><div><h1>{selectedProject?.name ?? "Abra um projeto"}</h1><span>{selectedProject?.path ?? "Selecione uma pasta para começar"}</span></div><div className="project-header-actions">{cloudSyncEnabled && selectedProject ? <><button className={selectedProject.filesSyncEnabled ? "sync-history-toggle sync-history-toggle--active" : "sync-history-toggle"} onClick={() => toggleFilesSync(selectedProject)} title="Espelha os arquivos do projeto na sua nuvem (exceto .git, dependências, segredos e arquivos > 4 MiB)">☁ Arquivos: {selectedProject.filesSyncEnabled ? "ligado" : "desligado"}</button><button className={selectedProject.syncTerminalHistory ? "sync-history-toggle sync-history-toggle--active" : "sync-history-toggle"} onClick={() => toggleTerminalHistorySync(selectedProject)} title="Histórico de terminal pode conter segredos colados — desligado por padrão">☁ Histórico: {selectedProject.syncTerminalHistory ? "ligado" : "desligado"}</button></> : null}<button className="handoff-button" disabled={!selectedProject} onClick={() => setHandoffOpen(true)}>⇄ <span>Transferir contexto</span></button></div></section>
    {availableUpdate ? <UpdateBanner update={availableUpdate} onDismiss={() => setAvailableUpdate(null)} /> : null}
    {projectError ? <div className="project-error" role="alert">{projectError}</div> : null}
    {syncConflict ? <div className="project-error sync-conflict" role="alert"><p>{syncConflict.message}</p>{syncConflict.files.length > 0 ? <ul className="sync-conflict-files">{syncConflict.files.slice(0, 8).map((file) => <li key={file}>{file}</li>)}{syncConflict.files.length > 8 ? <li>… e mais {syncConflict.files.length - 8}</li> : null}</ul> : null}<div className="sync-conflict-actions"><button className="secondary-button" onClick={() => resolveSyncConflict("local")}>Usar versão local</button><button className="secondary-button" onClick={() => resolveSyncConflict("remote")}>Usar versão da nuvem</button></div></div> : null}
    {!syncConflict && syncStatus ? <div className="sync-status" role="status" onClick={() => setSyncStatus(null)}>☁ {syncStatus}</div> : null}
    {projects.length === 0 ? <Onboarding onAddProject={addProject} /> : null}
    <div className="content-stack" style={{ "--chip": activeChip.color } as React.CSSProperties}>
      <div className="content-keyline" aria-hidden="true" />
      <TerminalWorkspace agents={agents} connectedAgents={connectedAgents} workingDirectory={selectedProject?.path ?? null} active={utility === "Terminal"} onAgentStarted={refreshConnectedAgents} /><HistoryPane projectId={selectedProject?.id ?? null} active={utility === "Histórico"} /><MemoryPane projectId={selectedProject?.id ?? null} active={utility === "Memória"} /><GitPane projectId={selectedProject?.id ?? null} active={utility === "Git"} /><TasksPane projectId={selectedProject?.id ?? null} active={utility === "Tarefas"} />
    </div>
    <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} onSettingsSaved={onSettingsSaved} />
    <HandoffDialog open={handoffOpen} projectId={selectedProject?.id ?? null} agents={agents} onClose={() => setHandoffOpen(false)} />
    <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} />
  </div>;
}

/** Primeira execução para quem baixou o app: o que fazer em três passos. */
function Onboarding({ onAddProject }: { onAddProject(): void }) {
  return <section className="onboarding" aria-label="Comece aqui">
    <div className="onboarding-card">
      <h2>Bem-vindo ao Eureka42</h2>
      <p className="onboarding-tagline">Um terminal. Inteligência infinita. Qualquer CLI de IA no mesmo projeto, com contexto que atravessa conversas e máquinas.</p>
      <ol className="onboarding-steps">
        <li><strong>Abra um projeto</strong><span>Aponte a pasta do seu trabalho — o histórico, a memória e o contexto passam a ficar ligados a ela.</span></li>
        <li><strong>Conecte um agente</strong><span>No terminal, escolha Codex, Claude, Kimi ou qualquer outra CLI instalada e inicie a sessão — é um PTY real, com o contexto do projeto já carregado.</span></li>
        <li><strong>Continue de qualquer lugar</strong><span>Em Configurações → Nuvem, conecte seu Google Drive, OneDrive ou Nextcloud. Mesma conta em outro computador = mesmo trabalho.</span></li>
      </ol>
      <button className="primary-button" onClick={onAddProject}>Abrir meu primeiro projeto</button>
    </div>
  </section>;
}
