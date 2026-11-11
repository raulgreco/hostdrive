import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { exportTerminalSession, getTerminalSessionEvents, listTerminalSessions, type SessionSummary, type TerminalEvent } from "../services/history";

export function HistoryPane({ projectId, active }: { projectId: string | null; active: boolean }) {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [agentId, setAgentId] = useState("");
  const [status, setStatus] = useState("");
  const [periodDays, setPeriodDays] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [selected, setSelected] = useState<SessionSummary | null>(null);
  const [events, setEvents] = useState<TerminalEvent[]>([]);
  const [detailStatus, setDetailStatus] = useState("");
  const deferredQuery = useDeferredValue(query);
  const startedAfter = useMemo(() => periodDays ? Math.floor(Date.now() / 1000) - periodDays * 86400 : 0, [periodDays]);

  useEffect(() => {
    if (!selected) return;
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === "Escape") setSelected(null); };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [selected]);

  useEffect(() => {
    if (!active || !projectId) return;
    let current = true;
    listTerminalSessions(projectId, { query: deferredQuery, agentId, status, startedAfter })
      .then((items) => { if (current) { setSessions(items); setHasMore(items.length === 50); setError(null); } })
      .catch((reason) => { if (current) setError(String(reason)); });
    return () => { current = false; };
  }, [active, projectId, deferredQuery, agentId, status, startedAfter]);

  async function loadMore() {
    if (!projectId || loadingMore) return;
    setLoadingMore(true);
    try {
      const items = await listTerminalSessions(projectId, { query: deferredQuery, agentId, status, startedAfter }, 50, sessions.length);
      setSessions((current) => [...current, ...items]);
      setHasMore(items.length === 50);
      setError(null);
    } catch (reason) { setError(String(reason)); }
    finally { setLoadingMore(false); }
  }
  async function inspect(session: SessionSummary) { setSelected(session); setEvents([]); setDetailStatus("Carregando eventos…"); try { const items = await getTerminalSessionEvents(session.id); setEvents(items); setDetailStatus(items.length === 200 ? "Mostrando os primeiros 200 eventos" : ""); } catch (reason) { setDetailStatus(String(reason)); } }
  async function exportAs(format: "md" | "json" | "txt") { if (!selected) return; setDetailStatus("Exportando com redação básica…"); try { const result = await exportTerminalSession(selected.id, format); setDetailStatus(`Exportado em ${result.path}`); } catch (reason) { setDetailStatus(String(reason)); } }

  if (!active) return null;
  return <section className="history-pane" aria-label="Histórico de sessões"><header><div><h2>Histórico</h2><p>Eventos brutos capturados e pesquisados localmente.</p></div><span>{sessions.length} sessões</span></header>
    <div className="history-filters"><label><span>Buscar no terminal</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Comando, arquivo ou resposta…" /></label><label><span>Agente</span><select value={agentId} onChange={(event) => setAgentId(event.target.value)}><option value="">Todos</option><option value="codex">Codex</option><option value="claude">Claude</option><option value="opencode">OpenCode</option><option value="kimi">Kimi</option></select></label><label><span>Estado</span><select value={status} onChange={(event) => setStatus(event.target.value)}><option value="">Todos</option><option value="EXITED">Concluída</option><option value="TERMINATED">Encerrada</option><option value="INTERRUPTED">Interrompida</option></select></label><label><span>Período</span><select value={periodDays} onChange={(event) => setPeriodDays(Number(event.target.value))}><option value={0}>Todo período</option><option value={1}>Últimas 24 horas</option><option value={7}>Últimos 7 dias</option><option value={30}>Últimos 30 dias</option></select></label></div>
    {error ? <p className="history-error" role="alert">{error}</p> : null}
    {!projectId ? <p className="history-empty">Abra um projeto para consultar o histórico.</p> : sessions.length === 0 ? <p className="history-empty">Nenhuma sessão encontrada com estes filtros.</p> : <><div className="session-list">{sessions.map((session) => <button className="session-row" key={session.id} onClick={() => inspect(session)}><span className={`session-state session-state--${session.status.toLowerCase()}`} /><div><strong>{session.agentId}</strong><small>{new Date(session.startedAt * 1000).toLocaleString()}</small></div><code>{session.id.slice(0, 8)}</code><span>{session.eventCount} eventos</span><span>{session.capturedBytes.toLocaleString()} bytes</span><b>{session.status}</b></button>)}</div>{hasMore ? <button className="history-more" onClick={loadMore} disabled={loadingMore}>{loadingMore ? "Carregando…" : "Carregar mais"}</button> : null}</>}
    {selected ? <div className="settings-backdrop"><section className="session-detail" role="dialog" aria-modal="true" aria-label="Detalhes da sessão"><header><div><span>{selected.agentId} · {selected.id.slice(0, 8)}</span><h2>Eventos da sessão</h2></div><button className="settings-close" onClick={() => setSelected(null)} aria-label="Fechar detalhes">×</button></header><div className="event-stream">{events.map((event) => <article key={event.id}><small>{event.direction} · {new Date(event.createdAt * 1000).toLocaleTimeString()}</small><pre>{event.text}</pre></article>)}</div><footer><span>{detailStatus || "Exportações aplicam redação básica; revise antes de compartilhar."}</span><div><button className="secondary-button" onClick={() => exportAs("txt")}>TXT</button><button className="secondary-button" onClick={() => exportAs("json")}>JSON</button><button className="primary-button" onClick={() => exportAs("md")}>Markdown</button></div></footer></section></div> : null}
  </section>;
}
