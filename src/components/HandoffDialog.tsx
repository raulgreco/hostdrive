import { useEffect, useState } from "react";
import { prepareHandoff, type HandoffPackage } from "../services/handoff";
import type { Agent } from "../data/workspace";

export function HandoffDialog({ open, projectId, agents, onClose }: { open: boolean; projectId: string | null; agents: Agent[]; onClose(): void }) {
  const [source, setSource] = useState("codex"); const [target, setTarget] = useState("claude"); const [result, setResult] = useState<HandoffPackage | null>(null); const [status, setStatus] = useState("");
  useEffect(() => {
    if (!open) return;
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === "Escape") onClose(); };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [open, onClose]);
  if (!open) return null;
  async function create() { if (!projectId) return; setStatus("Preparando pacote…"); setResult(null); try { const value = await prepareHandoff(projectId, source, target); setResult(value); setStatus("Pacote pronto para revisão e entrega explícita."); } catch (error) { setStatus(String(error)); } }
  return <div className="settings-backdrop" role="presentation"><section className="handoff-dialog" role="dialog" aria-modal="true" aria-labelledby="handoff-title"><header><div><span>UAHP 1.0</span><h2 id="handoff-title">Transferir contexto</h2></div><button className="settings-close" onClick={onClose} aria-label="Fechar transferência">×</button></header><div className="handoff-content"><p>Cria um pacote local e verificável. A sessão atual e os arquivos do projeto não são alterados.</p><div className="handoff-route"><label><span>Origem</span><select value={source} onChange={(event) => setSource(event.target.value)}>{agents.filter((agent) => agent.available).map((agent) => <option value={agent.id} key={agent.id}>{agent.label}</option>)}</select></label><b>→</b><label><span>Destino</span><select value={target} onChange={(event) => setTarget(event.target.value)}>{agents.filter((agent) => agent.available).map((agent) => <option value={agent.id} key={agent.id}>{agent.label}</option>)}</select></label></div><div className="handoff-scope"><strong>Conteúdo incluído</strong><ul><li>Manifesto versionado e diretório do projeto</li><li>Memória confirmada e tarefas locais</li><li>Prompt de continuidade com limites explícitos</li></ul><small>Arquivos do projeto, `.env`, chaves e credenciais não são coletados.</small></div>{result ? <div className="handoff-result"><strong>Pacote {result.status}</strong><code>{result.packagePath}</code>{result.warnings.map((warning) => <p key={warning}>⚠ {warning}</p>)}</div> : null}{status ? <p className="handoff-status">{status}</p> : null}</div><footer><button className="secondary-button" onClick={onClose}>Fechar</button><button className="primary-button" onClick={create} disabled={!projectId || source === target}>Criar pacote</button></footer></section></div>;
}
