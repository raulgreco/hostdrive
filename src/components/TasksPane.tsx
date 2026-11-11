import { useEffect, useState } from "react";
import { addProjectTask, listProjectTasks, setProjectTaskCompleted, type ProjectTask } from "../services/workspace";

export function TasksPane({ projectId, active }: { projectId: string | null; active: boolean }) {
  const [tasks, setTasks] = useState<ProjectTask[]>([]); const [title, setTitle] = useState(""); const [error, setError] = useState("");
  useEffect(() => { if (!active || !projectId) return; let current = true; listProjectTasks(projectId).then((items) => { if (current) setTasks(items); }).catch((reason) => { if (current) setError(String(reason)); }); return () => { current = false; }; }, [active, projectId]);
  async function add() { if (!projectId || !title.trim()) return; try { const task = await addProjectTask(projectId, title); setTasks((current) => [task, ...current]); setTitle(""); } catch (reason) { setError(String(reason)); } }
  async function toggle(task: ProjectTask) { try { await setProjectTaskCompleted(task.id, !task.completed); setTasks((current) => current.map((item) => item.id === task.id ? { ...item, completed: !item.completed } : item)); } catch (reason) { setError(String(reason)); } }
  if (!active) return null;
  return <section className="tool-pane"><header><div><h2>Tarefas</h2><p>Próximos passos locais usados na continuidade entre agentes.</p></div><span>{tasks.filter((task) => !task.completed).length} PENDENTES</span></header>{!projectId ? <p className="history-empty">Abra um projeto para gerenciar tarefas.</p> : <><div className="task-create"><input value={title} onChange={(event) => setTitle(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") void add(); }} placeholder="Adicionar próximo passo…" /><button onClick={add}>Adicionar</button></div>{error ? <p className="history-error">{error}</p> : null}<div className="task-list">{tasks.map((task) => <label className={task.completed ? "task-row task-row--done" : "task-row"} key={task.id}><input type="checkbox" checked={task.completed} onChange={() => toggle(task)} /><span>{task.title}</span><small>{task.provenance}</small></label>)}</div></>}</section>;
}
