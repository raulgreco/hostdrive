import { useEffect, useState } from "react";
import { getProjectContext, listProjectMemory, refreshProjectContext, saveProjectMemory, type ProjectContext } from "../services/workspace";

const sections = [{ id: "IDENTITY", label: "Identidade" }, { id: "REQUIREMENTS", label: "Requisitos" }, { id: "ARCHITECTURE", label: "Arquitetura" }, { id: "DECISIONS", label: "Decisões" }, { id: "CURRENT_STATE", label: "Estado atual" }];
const CONTEXT_SECTION = "CONTEXT";

export function MemoryPane({ projectId, active }: { projectId: string | null; active: boolean }) {
  const [section, setSection] = useState("IDENTITY"); const [values, setValues] = useState<Record<string, string>>({}); const [status, setStatus] = useState("");
  const [context, setContext] = useState<ProjectContext | null>(null);
  const [contextStatus, setContextStatus] = useState("");
  useEffect(() => { if (!active || !projectId) return; let current = true; listProjectMemory(projectId).then((items) => { if (current) setValues(Object.fromEntries(items.map((item) => [item.section, item.content]))); }).catch((error) => { if (current) setStatus(String(error)); }); return () => { current = false; }; }, [active, projectId]);
  useEffect(() => { if (!active || !projectId || section !== CONTEXT_SECTION) return; let current = true; getProjectContext(projectId).then((value) => { if (current) { setContext(value); setContextStatus(""); } }).catch((error) => { if (current) setContextStatus(String(error)); }); return () => { current = false; }; }, [active, projectId, section]);
  async function save() { if (!projectId) return; setStatus("Salvando…"); try { await saveProjectMemory(projectId, section, values[section] ?? ""); setStatus("Salvo localmente"); } catch (error) { setStatus(String(error)); } }
  async function rebuildContext() { if (!projectId) return; setContextStatus("Atualizando…"); try { setContext(await refreshProjectContext(projectId)); setContextStatus("Contexto publicado no projeto."); } catch (error) { setContextStatus(String(error)); } }
  async function copyContext() { if (!context) return; setContextStatus("Copiando…"); try { await navigator.clipboard.writeText(context.markdown); setContextStatus("Contexto copiado — cole no início da conversa."); } catch (error) { setContextStatus(String(error)); } }
  if (!active) return null;
  const viewingContext = section === CONTEXT_SECTION;
  return <section className="tool-pane"><header><div><h2>Memória do projeto</h2><p>Contexto independente dos agentes, com origem confirmada pelo usuário.</p></div><span>LOCAL</span></header>{!projectId ? <p className="history-empty">Abra um projeto para editar a memória.</p> : <div className="memory-editor"><nav>{sections.map((item) => <button className={section === item.id ? "active" : ""} onClick={() => setSection(item.id)} key={item.id}>{item.label}</button>)}<button className={section === CONTEXT_SECTION ? "active" : ""} onClick={() => setSection(CONTEXT_SECTION)}>Contexto</button></nav>
    {viewingContext ? <div><textarea readOnly value={context?.markdown ?? "Carregando contexto…"} aria-label="Contexto agregado do projeto" /><footer><span>{contextStatus || (context ? `${context.memorySections} seções de memória · ${context.openTasks} tarefas em aberto · ${context.recentSessions} conversas recentes${context.filePath ? " · publicado em .eureka42/contexto.md" : ""}` : "")}</span><span className="memory-actions"><button onClick={() => void copyContext()} disabled={!context}>Copiar</button><button onClick={() => void rebuildContext()}>Atualizar agora</button></span></footer></div> :
    <div><textarea value={values[section] ?? ""} onChange={(event) => setValues((current) => ({ ...current, [section]: event.target.value }))} placeholder="Registre apenas informações verificadas e úteis para continuidade…" /><footer><span>{status}</span><button onClick={save}>Salvar seção</button></footer></div>}</div>}</section>;
}
