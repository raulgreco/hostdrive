import { useState } from "react";
import type { ConnectedAgent } from "../services/agents";
import type { Project } from "../services/projects";
import { StatusDot } from "./StatusDot";

/** Histórico agregado de um projeto (sessões + última atividade). */
export type ProjectHistoryStats = { sessions: number; lastLabel: string; lastUsedAt: number };

type Props = { agents: ConnectedAgent[]; hasProject: boolean; projectStats: Record<string, ProjectHistoryStats>; projects: Project[]; selectedProjectId: string | null; onProjectSelect: (project: Project) => void; onAddProject: () => void; onFavorite: (project: Project) => void; onArchive: (project: Project) => void };

function relativeTime(timestamp: number): string {
  if (!timestamp) return "";
  const elapsed = Date.now() / 1000 - timestamp;
  if (elapsed < 90) return "agora";
  const minutes = Math.floor(elapsed / 60);
  if (minutes < 60) return `há ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `há ${hours} h`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `há ${days} d`;
  return `há ${Math.floor(days / 30)} m`;
}

function historyLine(stats: ProjectHistoryStats | undefined): string {
  if (!stats || stats.sessions === 0) return "Sem sessões ainda";
  const when = relativeTime(stats.lastUsedAt);
  return `${stats.sessions} ${stats.sessions === 1 ? "sessão" : "sessões"}${stats.lastLabel ? ` · última: ${stats.lastLabel}` : ""}${when ? ` ${when}` : ""}`;
}

export function Sidebar({ agents, hasProject, projectStats, projects, selectedProjectId, onProjectSelect, onAddProject, onFavorite, onArchive }: Props) {
  const [query, setQuery] = useState("");
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const visibleProjects = normalizedQuery ? projects.filter((project) => `${project.name} ${project.path}`.toLocaleLowerCase().includes(normalizedQuery)) : projects;
  return <aside className="sidebar" aria-label="Navegação do projeto">
    <section><div className="section-heading"><h2>Projetos</h2><button className="add-project-button" onClick={onAddProject} title="Abrir uma pasta como projeto">
      <svg width="11" height="11" viewBox="0 0 11 11" aria-hidden="true"><path d="M5.5 1v9M1 5.5h9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" /></svg>
      Novo projeto
    </button></div>
      <label className="search"><span aria-hidden="true">⌕</span><input value={query} onChange={(event) => setQuery(event.target.value)} aria-label="Buscar projetos" placeholder="Buscar projetos…" /></label>
      <nav className="project-list">{visibleProjects.length === 0 ? <p className="empty-projects">Nenhum projeto aberto.</p> : visibleProjects.map((project) => <div className={project.id === selectedProjectId ? "project-row project-row--selected" : "project-row"} key={project.id}>
        <button className="project-main" onClick={() => onProjectSelect(project)}><span className="folder" aria-hidden="true" /><span><strong>{project.name}</strong><small>{project.path}</small><small className="project-history">{historyLine(projectStats[project.id])}</small></span></button>
        <button className="project-action" aria-label={project.isFavorite ? `Desfavoritar ${project.name}` : `Favoritar ${project.name}`} onClick={() => onFavorite(project)}>{project.isFavorite ? "★" : "☆"}</button>
        <button className="project-action" aria-label={`Arquivar ${project.name}`} onClick={() => onArchive(project)}>×</button>
      </div>)}</nav></section>
    <section className="agents-panel"><h2>Agentes</h2>
      {!hasProject ? <p className="empty-projects">Abra um projeto para conectar agentes.</p> :
        agents.length === 0 ? <p className="empty-projects">Nenhum agente conectado ainda — clique em <strong>Escolha seu agente</strong> no terminal.</p> :
        agents.map((agent) => <div className="agent-row" key={agent.id} title={agent.executable}>
          <StatusDot active={agent.running} />
          <span><strong>{agent.label}</strong><small>{agent.sessionCount === 1 ? "1 sessão" : `${agent.sessionCount} sessões`} · {relativeTime(agent.lastUsedAt)}</small></span>
        </div>)}
    </section>
  </aside>;
}
