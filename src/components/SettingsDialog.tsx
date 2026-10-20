import { useEffect, useState } from "react";
import { getAppSettings, updateAppSettings, type AppSettings, type ThemePreference } from "../services/settings";
import { clearNextcloudCredentials, hasNextcloudCredentials, setNextcloudCredentials, testNextcloudConnection } from "../services/sync";
import { isTauriRuntime } from "../services/terminal";

const defaults: AppSettings = { captureTerminalEvents: true, retentionDays: 90, theme: "dark", nextcloudUrl: "", nextcloudUsername: "", nextcloudSyncEnabled: false };

const SHORTCUTS: Array<{ keys: string; description: string }> = [
  { keys: "⌘/Ctrl + K", description: "Focar o comando do orquestrador (///agente, ///retomar) no terminal ativo" },
  { keys: "⌘/Ctrl + N", description: "Abrir um novo terminal (até 6 por projeto)" },
  { keys: "⌘/Ctrl + ,", description: "Abrir Configurações" },
  { keys: "Esc", description: "Fechar a janela em foco (Configurações, Transferir contexto, detalhes de sessão)" }
];

export function SettingsDialog({ open, onClose, onSettingsSaved }: { open: boolean; onClose(): void; onSettingsSaved?(settings: AppSettings): void }) {
  const [settings, setSettings] = useState(defaults);
  const [section, setSection] = useState<"account" | "privacy" | "general" | "nextcloud">("account");
  const [status, setStatus] = useState<"idle" | "loading" | "saving" | "saved" | "error">("idle");
  const [credentialsConfigured, setCredentialsConfigured] = useState<boolean | null>(null);
  const [appPassword, setAppPassword] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [passphraseConfirm, setPassphraseConfirm] = useState("");
  const [credentialStatus, setCredentialStatus] = useState("");
  const [connectionStatus, setConnectionStatus] = useState("");

  useEffect(() => {
    if (!open || !isTauriRuntime()) return;
    let current = true;
    setStatus("loading");
    Promise.all([getAppSettings(), hasNextcloudCredentials()]).then(([value, configured]) => {
      if (current) { setSettings(value); setCredentialsConfigured(configured); setStatus("idle"); }
    }).catch(() => { if (current) setStatus("error"); });
    return () => { current = false; };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === "Escape") onClose(); };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [open, onClose]);

  async function save() {
    if (!isTauriRuntime()) return;
    setStatus("saving");
    try { const saved = await updateAppSettings(settings); setSettings(saved); setStatus("saved"); onSettingsSaved?.(saved); }
    catch { setStatus("error"); }
  }

  function setTheme(theme: ThemePreference) { setSettings((current) => ({ ...current, theme })); }

  // Salva endereço/usuário/toggle (SQLite) e credenciais (chaveiro) juntos,
  // num único clique — antes eram dois botões separados e era fácil salvar
  // só um dos dois, deixando "Nextcloud não configurado" mesmo depois de
  // preencher a tela inteira.
  async function saveNextcloudSection() {
    if (!isTauriRuntime()) return;
    if (passphrase && passphrase !== passphraseConfirm) { setCredentialStatus("As duas frases de criptografia digitadas são diferentes."); return; }
    setCredentialStatus("Salvando…");
    try {
      const saved = await updateAppSettings(settings);
      setSettings(saved);
      onSettingsSaved?.(saved);
      if (appPassword || passphrase) {
        await setNextcloudCredentials(appPassword, passphrase);
        setAppPassword(""); setPassphrase(""); setPassphraseConfirm("");
        setCredentialsConfigured(true);
      }
      setCredentialStatus("Configuração do Nextcloud salva: endereço, usuário" + (appPassword || passphrase ? " e credenciais" : "") + ".");
    } catch (error) { setCredentialStatus(String(error)); }
  }

  async function removeCredentials() {
    setCredentialStatus("Removendo…");
    try { await clearNextcloudCredentials(); setCredentialsConfigured(false); setCredentialStatus("Credenciais removidas do chaveiro do sistema."); }
    catch (error) { setCredentialStatus(String(error)); }
  }

  async function testConnection() {
    setConnectionStatus("Testando…");
    try { setConnectionStatus(await testNextcloudConnection()); }
    catch (error) { setConnectionStatus(String(error)); }
  }

  if (!open) return null;
  return <div className="settings-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <section className="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <header><div><span>HostDrive</span><h2 id="settings-title">Configurações</h2></div><button className="settings-close" onClick={onClose} aria-label="Fechar configurações">×</button></header>
      <div className="settings-body">
        <nav aria-label="Seções das configurações">
          <button className={section === "account" ? "active" : ""} onClick={() => setSection("account")}>Conta</button>
          <button className={section === "privacy" ? "active" : ""} onClick={() => setSection("privacy")}>Privacidade</button>
          <button className={section === "nextcloud" ? "active" : ""} onClick={() => setSection("nextcloud")}>Nextcloud</button>
          <button className={section === "general" ? "active" : ""} onClick={() => setSection("general")}>Geral</button>
        </nav>
        <div className="settings-content">
          {section === "account" ? <>
            <div className="local-profile"><div className="profile-mark">L</div><div><strong>Modo local</strong><p>Nenhuma conta é necessária. Seus agentes e dados permanecem neste computador.</p></div><span>ATIVO</span></div>
            <div className="settings-card"><div><strong>Continuar em outro computador</strong><p>Memória, tarefas, pacotes de transferência e preferências podem sincronizar via o seu próprio Nextcloud — sem conta nossa, sem servidor nosso.</p></div><button className="secondary-button" onClick={() => setSection("nextcloud")}>Configurar Nextcloud</button></div>
            <p className="settings-note">O código continua sendo sincronizado pelo seu provedor Git. Credenciais dos agentes (Codex, Claude, OpenCode, Kimi) nunca são tocadas pelo HostDrive.</p>
          </> : null}
          {section === "privacy" ? <>
            <h3>Histórico local</h3>
            <label className="settings-toggle"><div><strong>Capturar eventos do terminal</strong><p>Salva entrada e saída brutas no SQLite local para histórico e retomada.</p></div><input type="checkbox" checked={settings.captureTerminalEvents} onChange={(event) => setSettings((current) => ({ ...current, captureTerminalEvents: event.target.checked }))} /></label>
            <label className="settings-field"><span>Retenção planejada</span><select value={settings.retentionDays} onChange={(event) => setSettings((current) => ({ ...current, retentionDays: Number(event.target.value) }))}><option value={30}>30 dias</option><option value={90}>90 dias</option><option value={365}>1 ano</option><option value={3650}>Sem limpeza automática</option></select></label>
            <p className="settings-note">A retenção já é salva como preferência; a limpeza automática será ativada em uma etapa posterior.</p>
          </> : null}
          {section === "nextcloud" ? <>
            <h3>Sincronização via Nextcloud</h3>
            <p className="settings-note">Envia memória, tarefas, pacotes de transferência (UAHP) e preferências para uma pasta no seu Nextcloud, cifrados antes de saírem deste computador — nem quem administra o servidor vê o conteúdo. Histórico de terminal fica de fora por padrão (pode conter segredos); ative por projeto, individualmente, se quiser.</p>
            <label className="settings-toggle"><div><strong>Ativar sincronização</strong><p>Ao abrir um projeto, traz dados mais novos do Nextcloud; ao trocar de projeto ou fechar o app, envia o que mudou aqui.</p></div><input type="checkbox" checked={settings.nextcloudSyncEnabled} onChange={(event) => setSettings((current) => ({ ...current, nextcloudSyncEnabled: event.target.checked }))} /></label>
            <label className="settings-field"><span>Endereço do servidor</span><input className="settings-text-input" value={settings.nextcloudUrl} onChange={(event) => setSettings((current) => ({ ...current, nextcloudUrl: event.target.value }))} placeholder="https://nuvem.exemplo.com" /></label>
            <label className="settings-field"><span>Usuário</span><input className="settings-text-input" value={settings.nextcloudUsername} onChange={(event) => setSettings((current) => ({ ...current, nextcloudUsername: event.target.value }))} placeholder="raul" /></label>
            <h3 className="settings-section-gap">Credenciais (ficam só no chaveiro do sistema)</h3>
            <p className="settings-note">{credentialsConfigured ? "Senha de app e frase de criptografia já configuradas nesta máquina." : "Ainda não configuradas nesta máquina."} A frase de criptografia é diferente da senha do Nextcloud — ela nunca é enviada a lugar nenhum, só usada aqui para cifrar/decifrar. Deixe os três campos abaixo em branco se só quiser atualizar endereço/usuário sem trocar as credenciais já salvas.</p>
            <label className="settings-field"><span>Senha de app (App Password) do Nextcloud</span><input className="settings-text-input" type="password" value={appPassword} onChange={(event) => setAppPassword(event.target.value)} placeholder="Gerada em Configurações → Segurança no Nextcloud" /></label>
            <label className="settings-field"><span>Frase de criptografia</span><input className="settings-text-input" type="password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} placeholder="Mínimo 8 caracteres" /></label>
            <label className="settings-field"><span>Confirmar frase de criptografia</span><input className="settings-text-input" type="password" value={passphraseConfirm} onChange={(event) => setPassphraseConfirm(event.target.value)} /></label>
            <div className="settings-card">
              <div><strong>{credentialStatus || "Um único clique salva endereço, usuário, toggle e credenciais — nunca só parte da configuração."}</strong></div>
              <div className="settings-card-actions">
                {credentialsConfigured ? <button className="secondary-button" onClick={removeCredentials}>Remover credenciais</button> : null}
                <button className="primary-button" onClick={saveNextcloudSection}>Salvar configuração do Nextcloud</button>
              </div>
            </div>
            <div className="settings-card">
              <div><strong>{connectionStatus || "Testa se o app consegue falar com o seu Nextcloud (use depois de salvar)."}</strong></div>
              <button className="secondary-button" onClick={testConnection}>Testar conexão</button>
            </div>
          </> : null}
          {section === "general" ? <>
            <h3>Aparência</h3>
            <div className="theme-picker" role="radiogroup" aria-label="Tema">
              {([["system", "Sistema"], ["light", "Claro"], ["dark", "Escuro"]] as const).map(([value, label]) => <button key={value} role="radio" aria-checked={settings.theme === value} className={settings.theme === value ? "theme-option theme-option--active" : "theme-option"} onClick={() => setTheme(value)}>{label}</button>)}
            </div>
            <p className="settings-note">"Sistema" segue o modo claro/escuro do computador. O terminal continua com fundo escuro nos três modos para preservar o contraste das CLIs.</p>
            <h3 className="settings-section-gap">Atalhos de teclado</h3>
            <ul className="shortcut-list">{SHORTCUTS.map((shortcut) => <li key={shortcut.keys}><kbd>{shortcut.keys}</kbd><span>{shortcut.description}</span></li>)}</ul>
            <div className="settings-row"><div><strong>Processamento</strong><p>Os agentes são executados localmente por suas CLIs oficiais.</p></div><span>Local</span></div>
          </> : null}
        </div>
      </div>
      <footer><span>{status === "saved" ? "Configurações salvas" : status === "error" ? "Não foi possível salvar" : status === "loading" ? "Carregando…" : "Alterações permanecem neste dispositivo"}</span><div><button className="secondary-button" onClick={onClose}>Cancelar</button><button className="primary-button" onClick={save} disabled={status === "saving" || status === "loading"}>{status === "saving" ? "Salvando…" : "Salvar"}</button></div></footer>
    </section>
  </div>;
}
