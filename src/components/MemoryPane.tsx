import { useEffect, useState } from "react";
import { listProjectMemory, saveProjectMemory } from "../services/workspace";

const sections = [{ id: "IDENTITY", label: "Identidade" }, { id: "REQUIREMENTS", label: "Requisitos" }, { id: "ARCHITECTURE", label: "Arquitetura" }, { id: "DECISIONS", label: "Decisões" }, { id: "CURRENT_STATE", label: "Estado atual" }];

export function MemoryPane({ projectId, active }: { projectId: string | null; active: boolean }) {
  const [section, setSection] = useState("IDENTITY"); const [values, setValues] = useState<Record<string, string>>({}); const [status, setStatus] = useState("");
  useEffect(() => { if (!active || !projectId) return; let current = true; listProjectMemory(projectId).then((items) => { if (current) setValues(Object.fromEntries(items.map((item) => [item.section, item.content]))); }).catch((error) => { if (current) setStatus(String(error)); }); return () => { current = false; }; }, [active, projectId]);
  async function save() { if (!projectId) return; setStatus("Salvando…"); try { await saveProjectMemory(projectId, section, values[section] ?? ""); setStatus("Salvo localmente"); } catch (error) { setStatus(String(error)); } }
  if (!active) return null;
  return <section className="tool-pane"><header><div><h2>Memória do projeto</h2><p>Contexto independente dos agentes, com origem confirmada pelo usuário.</p></div><span>LOCAL</span></header>{!projectId ? <p className="history-empty">Abra um projeto para editar a memória.</p> : <div className="memory-editor"><nav>{sections.map((item) => <button className={section === item.id ? "active" : ""} onClick={() => setSection(item.id)} key={item.id}>{item.label}</button>)}</nav><div><textarea value={values[section] ?? ""} onChange={(event) => setValues((current) => ({ ...current, [section]: event.target.value }))} placeholder="Registre apenas informações verificadas e úteis para continuidade…" /><footer><span>{status}</span><button onClick={save}>Salvar seção</button></footer></div></div>}</section>;
}
