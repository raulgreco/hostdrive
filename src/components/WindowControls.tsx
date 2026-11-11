import { useEffect, useState } from "react";

/** Controles de janela desenhados pelo próprio app (titlebar do Baralho
    de Chips). Só aparecem quando a janela está sem decoração nativa: no
    macOS os semáforos nativos não entregam o clique de fechar neste
    ambiente (wry 2.12), então o app assume o controle; no Windows a barra
    nativa segue e estes controles ficam ocultos. */
export function WindowControls() {
  const [decorated, setDecorated] = useState<boolean | null>(null);

  useEffect(() => {
    let disposed = false;
    import("@tauri-apps/api/window").then(async ({ getCurrentWindow }) => {
      if (disposed) return;
      try {
        setDecorated(await getCurrentWindow().isDecorated());
      } catch {
        setDecorated(true);
      }
    });
    return () => { disposed = true; };
  }, []);

  if (decorated !== false) return null;

  async function close() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    // destroy() força o encerramento sem passar pelo evento nativo de
    // fechamento, que não é entregue neste ambiente.
    await getCurrentWindow().destroy();
  }

  async function minimize() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }

  async function toggleMaximize() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().toggleMaximize();
  }

  return <div className="window-controls" aria-label="Controles da janela">
    <button className="window-control window-control--close" aria-label="Fechar janela" onClick={() => void close()}>
      <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true"><path d="M1.6 1.6l4.8 4.8M6.4 1.6L1.6 6.4" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" /></svg>
    </button>
    <button className="window-control window-control--minimize" aria-label="Minimizar janela" onClick={() => void minimize()}>
      <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true"><path d="M1.2 4h5.6" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" /></svg>
    </button>
    <button className="window-control window-control--zoom" aria-label="Ampliar ou restaurar janela" onClick={() => void toggleMaximize()}>
      <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true"><path d="M1.5 4.8V1.5h3.3M6.5 3.2v3.3H3.2" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" strokeLinejoin="round" /></svg>
    </button>
  </div>;
}
