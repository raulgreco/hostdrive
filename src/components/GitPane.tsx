import { useEffect, useState } from "react";
import { getGitStatus, type GitStatus } from "../services/workspace";

export function GitPane({ projectId, active }: { projectId: string | null; active: boolean }) {
  const [state, setState] = useState<GitStatus | null>(null); const [error, setError] = useState("");
  useEffect(() => { if (!active || !projectId) return; let current = true; getGitStatus(projectId).then((value) => { if (current) setState(value); }).catch((reason) => { if (current) setError(String(reason)); }); return () => { current = false; }; }, [active, projectId]);
  if (!active) return null;
  return <section className="tool-pane"><header><div><h2>Git</h2><p>Leitura local; o aplicativo não faz commit, stash ou reset.</p></div><span>{state?.dirty ? "ALTERAÇÕES" : "LIMPO"}</span></header>{error ? <p className="history-error">{error}</p> : !projectId ? <p className="history-empty">Abra um projeto para consultar o Git.</p> : state && !state.available ? <p className="history-empty">{state.error}</p> : state ? <><div className="git-summary"><div><small>BRANCH</small><strong>{state.branch ?? "detached"}</strong></div><div><small>HEAD</small><code>{state.head ?? "—"}</code></div><div><small>ESTADO</small><strong>{state.dirty ? "Com alterações" : "Limpo"}</strong></div></div><pre className="git-changes">{state.changes.length ? state.changes.join("\n") : "Nenhuma alteração local."}</pre></> : <p className="history-empty">Consultando…</p>}</section>;
}
