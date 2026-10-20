import { useEffect, useRef, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { StatusDot } from "./components/StatusDot";
import { TerminalWorkspace } from "./components/TerminalWorkspace";
import { HistoryPane } from "./components/HistoryPane";
import { SettingsDialog } from "./components/SettingsDialog";
import { MemoryPane } from "./components/MemoryPane";
import { TasksPane } from "./components/TasksPane";
import { GitPane } from "./components/GitPane";
import { HandoffDialog } from "./components/HandoffDialog";
import hostdriveIcon from "./assets/hostdrive-icon.png";
import { agents as fallbackAgents } from "./data/workspace";
import { detectInstalledAgents } from "./services/agents";
import { chooseAndAddProject, listProjects, setProjectArchived, setProjectFavorite, setProjectTerminalSync, type Project } from "./services/projects";
import { getAppSettings, type AppSettings, type ThemePreference } from "./services/settings";
import { syncProjectNow } from "./services/sync";
import { isTauriRuntime } from "./services/terminal";

/** NFR-006 — aplica a preferência de tema ao documento, inclusive quando "Sistema" muda em tempo real. */
function useAppliedTheme(preference: ThemePreference) {
  useEffect(() => {
    const root = document.documentElement;
    if (preference !== "system") { root.dataset.theme = preference; return; }
    const media = window.matchMedia("(prefers-color-scheme: light)");
    const apply = () => { root.dataset.theme = media.matches ? "light" : "dark"; };
    apply();
    media.addEventListener("change", apply);
    return () => { delete root.dataset.theme; media.removeEventListener("change", apply); };
  }, [preference]);
}

const SYNC_TIMEOUT_MS = 8000;

export function App() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [utility, setUtility] = useState("Terminal");
  const [agents, setAgents] = useState(fallbackAgents);
  const [projectError, setProjectError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [handoffOpen, setHandoffOpen] = useState(false);
  const [theme, setTheme] = useState<ThemePreference>("dark");
  const [nextcloudSyncEnabled, setNextcloudSyncEnabled] = useState(false);
  const [syncStatus, setSyncStatus] = useState<string | null>(null);
  const [syncConflict, setSyncConflict] = useState<{ projectId: string; message: string } | null>(null);
  const selectedProject = projects.find((project) => project.id === selectedProjectId) ?? null;
  useAppliedTheme(theme);

  const previousProjectRef = useRef<string | null>(null);
  const selectedProjectIdRef = useRef<string | null>(null);
  const nextcloudSyncEnabledRef = useRef(false);
  selectedProjectIdRef.current = selectedProjectId;
  nextcloudSyncEnabledRef.current = nextcloudSyncEnabled;

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
      if (settings) { setTheme(settings.theme); setNextcloudSyncEnabled(settings.nextcloudSyncEnabled); }
    }).catch((error) => { if (active) setProjectError(String(error)); });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key === ",") { event.preventDefault(); setSettingsOpen(true); }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  // Sincronização automática: ao trocar de projeto, envia o que ficou para
  // trás e traz o que há de mais novo do que foi aberto. Nunca sobrescreve
  // silenciosamente em caso de conflito — ver docs/decisions/005-nextcloud-sync.md.
  useEffect(() => {
    const previous = previousProjectRef.current;
    previousProjectRef.current = selectedProjectId;
    if (!isTauriRuntime() || !nextcloudSyncEnabled) return;
    let active = true;
    (async () => {
      if (previous && previous !== selectedProjectId) {
        await syncProjectNow(previous).catch(() => undefined);
      }
      if (!selectedProjectId) return;
      try {
        const outcome = await syncProjectNow(selectedProjectId);
        if (!active) return;
        if (outcome.status === "conflict") setSyncConflict({ projectId: selectedProjectId, message: outcome.message });
        else if (outcome.status !== "disabled") setSyncStatus(outcome.message);
      } catch (error) { if (active) setSyncStatus(String(error)); }
    })();
    return () => { active = false; };
  }, [selectedProjectId, nextcloudSyncEnabled]);

  // Ao fechar a janela, dá tempo (limitado) para enviar o projeto aberto ao
  // Nextcloud antes de realmente encerrar — não é garantido se o SO matar o
  // processo com força, mas cobre o fechamento normal.
  useEffect(() => {
    if (!isTauriRuntime()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      if (disposed) return;
      const win = getCurrentWindow();
      win.onCloseRequested(async (event) => {
        event.preventDefault();
        const projectId = selectedProjectIdRef.current;
        if (projectId && nextcloudSyncEnabledRef.current) {
          await Promise.race([
            syncProjectNow(projectId).catch(() => undefined),
            new Promise((resolve) => setTimeout(resolve, SYNC_TIMEOUT_MS))
          ]);
        }
        await win.destroy();
      }).then((fn) => { if (disposed) fn(); else unlisten = fn; });
    });
    return () => { disposed = true; unlisten?.(); };
  }, []);

  async function resolveSyncConflict(prefer: "local" | "remote") {
    if (!syncConflict) return;
    try { const outcome = await syncProjectNow(syncConflict.projectId, prefer); setSyncStatus(outcome.message); }
    catch (error) { setSyncStatus(String(error)); }
    setSyncConflict(null);
  }

  async function toggleTerminalHistorySync(project: Project) {
    const enabled = !project.syncTerminalHistory;
    try { await setProjectTerminalSync(project.id, enabled); setProjects((current) => current.map((item) => item.id === project.id ? { ...item, syncTerminalHistory: enabled } : item)); }
    catch (error) { setProjectError(String(error)); }
  }

  function onSettingsSaved(settings: AppSettings) { setTheme(settings.theme); setNextcloudSyncEnabled(settings.nextcloudSyncEnabled); }

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
    <header className="titlebar" data-tauri-drag-region><img src={hostdriveIcon} alt="" className="app-icon" aria-hidden="true" /><strong>HostDrive</strong><div className="titlebar-status"><span><StatusDot />Local</span><span>{agents.filter((agent) => agent.available).length} agentes detectados</span><button className="icon-button" aria-label="Configurações" onClick={() => setSettingsOpen(true)}>⚙</button></div></header>
    <Sidebar agents={agents} projects={projects} selectedProjectId={selectedProjectId} onProjectSelect={(project) => setSelectedProjectId(project.id)} onAddProject={addProject} onFavorite={toggleFavorite} onArchive={archiveProject} />
    <section className="project-header"><div><h1>{selectedProject?.name ?? "Abra um projeto"}</h1><span>{selectedProject?.path ?? "Selecione uma pasta para começar"}</span></div><div className="project-header-actions">{nextcloudSyncEnabled && selectedProject ? <button className={selectedProject.syncTerminalHistory ? "sync-history-toggle sync-history-toggle--active" : "sync-history-toggle"} onClick={() => toggleTerminalHistorySync(selectedProject)} title="Histórico de terminal pode conter segredos colados — desligado por padrão">☁ Histórico no Nextcloud: {selectedProject.syncTerminalHistory ? "ligado" : "desligado"}</button> : null}<button className="handoff-button" disabled={!selectedProject} onClick={() => setHandoffOpen(true)}>⇄ <span>Transferir contexto</span></button></div></section>
    {projectError ? <div className="project-error" role="alert">{projectError}</div> : null}
    {syncConflict ? <div className="project-error sync-conflict" role="alert"><p>{syncConflict.message}</p><div className="sync-conflict-actions"><button className="secondary-button" onClick={() => resolveSyncConflict("local")}>Usar versão local</button><button className="secondary-button" onClick={() => resolveSyncConflict("remote")}>Usar versão do Nextcloud</button></div></div> : null}
    {!syncConflict && syncStatus ? <div className="sync-status" role="status" onClick={() => setSyncStatus(null)}>☁ {syncStatus}</div> : null}
    <div className="content-stack"><TerminalWorkspace agents={agents} workingDirectory={selectedProject?.path ?? null} active={utility === "Terminal"} /><HistoryPane projectId={selectedProject?.id ?? null} active={utility === "Histórico"} /><MemoryPane projectId={selectedProject?.id ?? null} active={utility === "Memória"} /><GitPane projectId={selectedProject?.id ?? null} active={utility === "Git"} /><TasksPane projectId={selectedProject?.id ?? null} active={utility === "Tarefas"} /></div>
    <nav className="utility-tabs" aria-label="Ferramentas do projeto">{["Terminal", "Histórico", "Memória", "Git", "Tarefas"].map((item) => <button className={item === utility ? "utility-tab utility-tab--active" : "utility-tab"} onClick={() => setUtility(item)} key={item}>{item}</button>)}</nav>
    <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} onSettingsSaved={onSettingsSaved} />
    <HandoffDialog open={handoffOpen} projectId={selectedProject?.id ?? null} agents={agents} onClose={() => setHandoffOpen(false)} />
  </div>;
}
