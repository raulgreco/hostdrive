import { useState } from "react";
import type { Agent } from "../data/workspace";
import type { Project } from "../services/projects";
import { StatusDot } from "./StatusDot";

type Props = { agents: Agent[]; projects: Project[]; selectedProjectId: string | null; onProjectSelect: (project: Project) => void; onAddProject: () => void; onFavorite: (project: Project) => void; onArchive: (project: Project) => void };

export function Sidebar({ agents, projects, selectedProjectId, onProjectSelect, onAddProject, onFavorite, onArchive }: Props) {
  const [query, setQuery] = useState("");
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const visibleProjects = normalizedQuery ? projects.filter((project) => `${project.name} ${project.path}`.toLocaleLowerCase().includes(normalizedQuery)) : projects;
  return <aside className="sidebar" aria-label="Navegação do projeto">
    <section><div className="section-heading"><h2>Projetos</h2><button className="icon-button" aria-label="Adicionar projeto" onClick={onAddProject}>＋</button></div>
      <label className="search"><span aria-hidden="true">⌕</span><input value={query} onChange={(event) => setQuery(event.target.value)} aria-label="Buscar projetos" placeholder="Buscar projetos…" /></label>
      <nav className="project-list">{visibleProjects.length === 0 ? <p className="empty-projects">Nenhum projeto aberto.</p> : visibleProjects.map((project) => <div className={project.id === selectedProjectId ? "project-row project-row--selected" : "project-row"} key={project.id}>
        <button className="project-main" onClick={() => onProjectSelect(project)}><span className="folder" aria-hidden="true" /><span><strong>{project.name}</strong><small>{project.path}</small></span></button>
        <button className="project-action" aria-label={project.isFavorite ? `Desfavoritar ${project.name}` : `Favoritar ${project.name}`} onClick={() => onFavorite(project)}>{project.isFavorite ? "★" : "☆"}</button>
        <button className="project-action" aria-label={`Arquivar ${project.name}`} onClick={() => onArchive(project)}>×</button>
      </div>)}</nav></section>
    <section className="agents-panel"><h2>Agentes</h2>{agents.map((agent) => <div className="agent-row" key={agent.id} title={agent.path ?? undefined}><StatusDot active={agent.available} /><span><strong>{agent.label}</strong><small>{agent.available ? agent.version ?? "versão desconhecida" : "não instalado"}</small></span></div>)}</section>
  </aside>;
}
