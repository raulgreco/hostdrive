import { useEffect, useState } from "react";
import type { Agent } from "../data/workspace";
import { TerminalPane } from "./TerminalPane";

type TerminalTab = { id: number; label: string };

export function TerminalWorkspace({ agents, workingDirectory, active }: { agents: Agent[]; workingDirectory: string | null; active: boolean }) {
  const [tabs, setTabs] = useState<TerminalTab[]>([{ id: 1, label: "Terminal 1" }]);
  const [selected, setSelected] = useState(1);
  function addTab() { setTabs((current) => { if (current.length >= 6) return current; const id = Math.max(...current.map((tab) => tab.id), 0) + 1; setSelected(id); return [...current, { id, label: `Terminal ${id}` }]; }); }
  function closeTab(id: number) { setTabs((current) => { if (current.length === 1) return current; const next = current.filter((tab) => tab.id !== id); if (selected === id) setSelected(next[0].id); return next; }); }

  useEffect(() => {
    if (!active) return;
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLocaleLowerCase() === "n") { event.preventDefault(); addTab(); }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active]);
  return <section className={active ? "terminal-stack" : "terminal-stack terminal-stack--inactive"} aria-hidden={!active}><nav className="terminal-tabs" aria-label="Terminais">{tabs.map((tab) => <div className={tab.id === selected ? "terminal-tab terminal-tab--active" : "terminal-tab"} key={tab.id}><button onClick={() => setSelected(tab.id)}>{tab.label}</button>{tabs.length > 1 ? <button aria-label={`Fechar ${tab.label}`} onClick={() => closeTab(tab.id)}>×</button> : null}</div>)}<button className="terminal-tab-add" onClick={addTab} disabled={tabs.length >= 6} aria-label="Novo terminal">＋</button></nav><div className="terminal-tab-content">{tabs.map((tab) => <TerminalPane key={tab.id} agents={agents} workingDirectory={workingDirectory} active={tab.id === selected} />)}</div></section>;
}
