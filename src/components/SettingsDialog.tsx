import { useEffect, useState } from "react";
import { getAppSettings, updateAppSettings, type AppSettings, type ThemePreference } from "../services/settings";
import { clearNextcloudCredentials, connectCloud, disconnectCloud, getCloudStatus, hasNextcloudCredentials, setNextcloudCredentials, testCloudConnection, type CloudStatus } from "../services/sync";
import { isTauriRuntime } from "../services/terminal";

const defaults: AppSettings = { captureTerminalEvents: true, retentionDays: 90, theme: "dark", nextcloudUrl: "", nextcloudUsername: "", nextcloudSyncEnabled: false, contextCarryoverEnabled: true, cloudProvider: "", googleClientId: "", onedriveClientId: "", cloudAccount: "" };

const SHORTCUTS: Array<{ keys: string; description: string }> = [
  { keys: "⌘/Ctrl + K", description: "Focar o comando do orquestrador (///agente, ///retomar) no terminal ativo" },
  { keys: "⌘/Ctrl + N", description: "Abrir um novo terminal (até 6 por projeto)" },
  { keys: "⌘/Ctrl + ,", description: "Abrir Configurações" },
  { keys: "Esc", description: "Fechar a janela em foco (Configurações, Transferir contexto, detalhes de sessão)" }
];

const PROVIDERS = [
  { id: "nextcloud", label: "Nextcloud" },
  { id: "google", label: "Google Drive" },
  { id: "onedrive", label: "OneDrive" }
] as const;

export function SettingsDialog({ open, onClose, onSettingsSaved }: { open: boolean; onClose(): void; onSettingsSaved?(settings: AppSettings): void }) {
  const [settings, setSettings] = useState(defaults);
  const [section, setSection] = useState<"account" | "privacy" | "general" | "cloud">("account");
  const [status, setStatus] = useState<"idle" | "loading" | "saving" | "saved" | "error">("idle");
  const [cloudStatus, setCloudStatus] = useState<CloudStatus | null>(null);
  const [credentialsConfigured, setCredentialsConfigured] = useState<boolean | null>(null);
  const [appPassword, setAppPassword] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [passphraseConfirm, setPassphraseConfirm] = useState("");
  const [credentialStatus, setCredentialStatus] = useState("");
  const [connectionStatus, setConnectionStatus] = useState("");
  const [oauthBusy, setOauthBusy] = useState(false);

  useEffect(() => {
    if (!open || !isTauriRuntime()) return;
    let current = true;
    setStatus("loading");
    Promise.all([getAppSettings(), hasNextcloudCredentials(), getCloudStatus().catch(() => null)]).then(([value, configured, cloud]) => {
      if (current) { setSettings(value); setCredentialsConfigured(configured); setCloudStatus(cloud); setStatus("idle"); }
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

  async function saveCloudSection() {
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
      setCredentialStatus("Configuração da nuvem salva.");
      setCloudStatus(await getCloudStatus());
    } catch (error) { setCredentialStatus(String(error)); }
  }

  async function connectAccount() {
    if (!isTauriRuntime()) return;
    setOauthBusy(true);
    setCredentialStatus("Abrindo o navegador para login…");
    try {
      // Salva o Client ID escolhido antes de abrir o OAuth.
      const saved = await updateAppSettings(settings);
      setSettings(saved);
      const account = await connectCloud();
      setCredentialStatus(`Conta conectada: ${account}.`);
      setCloudStatus(await getCloudStatus());
      setSettings((current) => ({ ...current, cloudAccount: account }));
    } catch (error) { setCredentialStatus(String(error)); }
    finally { setOauthBusy(false); }
  }

  async function disconnectAccount() {
    setCredentialStatus("Desconectando…");
    try {
      await disconnectCloud();
      setCredentialStatus("Conta desconectada — os tokens foram removidos do chaveiro.");
      setCloudStatus(await getCloudStatus());
      setSettings((current) => ({ ...current, cloudAccount: "" }));
    } catch (error) { setCredentialStatus(String(error)); }
  }

  async function removeCredentials() {
    setCredentialStatus("Removendo…");
    try { await clearNextcloudCredentials(); setCredentialsConfigured(false); setCloudStatus(await getCloudStatus()); setCredentialStatus("Credenciais do Nextcloud removidas do chaveiro do sistema."); }
    catch (error) { setCredentialStatus(String(error)); }
  }

  async function testConnection() {
    setConnectionStatus("Testando…");
    try { setConnectionStatus(await testCloudConnection()); }
    catch (error) { setConnectionStatus(String(error)); }
  }

  if (!open) return null;
  const usingNextcloud = settings.cloudProvider === "nextcloud";
  const oauthProvider = settings.cloudProvider === "google"
    ? { clientIdKey: "googleClientId" as const, name: "Google Drive", guide: "console.cloud.google.com → APIs e serviços → Tela de permissão OAuth (Externo, app próprio) → Credenciais → Criar ID do cliente → App para computador. Escopos não precisam ser configurados manualmente — o Eureka42 pede drive.appdata + e-mail no login." }
    : settings.cloudProvider === "onedrive"
      ? { clientIdKey: "onedriveClientId" as const, name: "OneDrive", guide: "entra.microsoft.com → Entra ID (ou Registros de aplicativo) → Novo registro → Plataforma → Aplicativo móvel e desktop → URI de redirecionamento http://127.0.0.1 → copie o ID do aplicativo (cliente). Escopos não precisam ser configurados manualmente — o Eureka42 pede Files.ReadWrite.AppFolder + e-mail no login." }
      : null;

  return <div className="settings-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <section className="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <header><div><span>Eureka42</span><h2 id="settings-title">Configurações</h2></div><button className="settings-close" onClick={onClose} aria-label="Fechar configurações">×</button></header>
      <div className="settings-body">
        <nav aria-label="Seções das configurações">
          <button className={section === "account" ? "active" : ""} onClick={() => setSection("account")}>Conta</button>
          <button className={section === "privacy" ? "active" : ""} onClick={() => setSection("privacy")}>Privacidade</button>
          <button className={section === "cloud" ? "active" : ""} onClick={() => setSection("cloud")}>Nuvem</button>
          <button className={section === "general" ? "active" : ""} onClick={() => setSection("general")}>Geral</button>
        </nav>
        <div className="settings-content">
          {section === "account" ? <>
            <div className="local-profile"><div className="profile-mark">L</div><div><strong>Modo local</strong><p>Nenhuma conta é necessária. Seus agentes e dados permanecem neste computador.</p></div><span>ATIVO</span></div>
            <div className="settings-card"><div><strong>Continuar em outro computador</strong><p>Memória, tarefas, contexto, histórico e arquivos do projeto podem sincronizar pela sua própria nuvem — Google Drive, OneDrive ou Nextcloud. Mesma conta em outra máquina = mesmo trabalho, sem servidor nosso.</p></div><button className="secondary-button" onClick={() => setSection("cloud")}>Configurar nuvem</button></div>
            <p className="settings-note">O código continua sendo sincronizado pelo seu provedor Git. Credenciais dos agentes nunca são tocadas pelo HostDrive.</p>
          </> : null}
          {section === "privacy" ? <>
            <h3>Contexto entre conversas</h3>
            <label className="settings-toggle"><div><strong>Carregar contexto de uma conversa para outra</strong><p>Mantém <code>.eureka42/contexto.md</code> no projeto atualizado (memória + tarefas + conversas recentes) e injeta esse contexto nas novas sessões do Claude Code — validado via <code>--append-system-prompt</code>.</p></div><input type="checkbox" checked={settings.contextCarryoverEnabled} onChange={(event) => setSettings((current) => ({ ...current, contextCarryoverEnabled: event.target.checked }))} /></label>
            <p className="settings-note">Nos demais agentes, o contexto fica disponível no arquivo — aponte ou cole no início da conversa. Nenhum conteúdo capturado do terminal entra no contexto.</p>
            <h3 className="settings-section-gap">Histórico local</h3>
            <label className="settings-toggle"><div><strong>Capturar eventos do terminal</strong><p>Salva entrada e saída brutas no SQLite local para histórico e retomada.</p></div><input type="checkbox" checked={settings.captureTerminalEvents} onChange={(event) => setSettings((current) => ({ ...current, captureTerminalEvents: event.target.checked }))} /></label>
            <label className="settings-field"><span>Retenção planejada</span><select value={settings.retentionDays} onChange={(event) => setSettings((current) => ({ ...current, retentionDays: Number(event.target.value) }))}><option value={30}>30 dias</option><option value={90}>90 dias</option><option value={365}>1 ano</option><option value={3650}>Sem limpeza automática</option></select></label>
            <p className="settings-note">A retenção já é salva como preferência; a limpeza automática será ativada em uma etapa posterior.</p>
          </> : null}
          {section === "cloud" ? <>
            <h3>Sincronização via nuvem</h3>
            <p className="settings-note">Escolha <em>uma</em> nuvem — a conta que você já tem é a identidade: ao abrir o Eureka42 em outro computador com a mesma conta e sincronizar, você recupera histórico, contexto e arquivos dos projetos. Tudo é cifrado antes de sair desta máquina; nem o provedor vê o conteúdo. Histórico de terminal e arquivos de projeto ficam de fora por padrão (ative por projeto). <strong>Nenhuma fica pré-selecionada</strong> — a escolha é sempre sua, e o app funciona 100% local sem nenhuma.</p>
            <div className="theme-picker" role="radiogroup" aria-label="Provedor de nuvem">
              {PROVIDERS.map((provider) => <button key={provider.id} role="radio" aria-checked={settings.cloudProvider === provider.id} className={settings.cloudProvider === provider.id ? "theme-option theme-option--active" : "theme-option"} onClick={() => setSettings((current) => ({ ...current, cloudProvider: provider.id }))}>{provider.label}</button>)}
            </div>
            {settings.cloudProvider === "" ? <p className="settings-note">Nenhuma nuvem selecionada — modo local puro. Escolha um provedor acima para configurar a conexão quando quiser.</p> : <>
            <label className="settings-toggle"><div><strong>Ativar sincronização</strong><p>Ao abrir um projeto, traz dados mais novos da nuvem; ao trocar de projeto ou fechar o app, envia o que mudou aqui.</p></div><input type="checkbox" checked={settings.nextcloudSyncEnabled} onChange={(event) => setSettings((current) => ({ ...current, nextcloudSyncEnabled: event.target.checked }))} /></label>
            {usingNextcloud ? <>
              <label className="settings-field"><span>Endereço do servidor</span><input className="settings-text-input" value={settings.nextcloudUrl} onChange={(event) => setSettings((current) => ({ ...current, nextcloudUrl: event.target.value }))} placeholder="https://nuvem.exemplo.com" /></label>
              <label className="settings-field"><span>Usuário</span><input className="settings-text-input" value={settings.nextcloudUsername} onChange={(event) => setSettings((current) => ({ ...current, nextcloudUsername: event.target.value }))} placeholder="raul" /></label>
            </> : oauthProvider ? <>
              <p className="settings-note"><strong>Como obter o Client ID ({oauthProvider.name}):</strong> {oauthProvider.guide}</p>
              <label className="settings-field"><span>Client ID</span><input className="settings-text-input" value={settings[oauthProvider.clientIdKey]} onChange={(event) => setSettings((current) => ({ ...current, [oauthProvider.clientIdKey]: event.target.value }))} placeholder={settings.cloudProvider === "google" ? "xxxx.apps.googleusercontent.com" : "00000000-0000-0000-0000-000000000000"} /></label>
              <div className="settings-card">
                <div><strong>{cloudStatus?.configured && settings.cloudAccount ? `Conta conectada: ${settings.cloudAccount}` : "Conta não conectada nesta máquina."}</strong><p>O login abre no navegador; os tokens ficam só no chaveiro do sistema.</p></div>
                <div className="settings-card-actions">
                  {cloudStatus?.configured ? <button className="secondary-button" onClick={() => void disconnectAccount()}>Desconectar</button> : null}
                  <button className="primary-button" onClick={() => void connectAccount()} disabled={oauthBusy || !settings[oauthProvider.clientIdKey].trim()}>{oauthBusy ? "Aguardando login…" : "Conectar conta"}</button>
                </div>
              </div>
            </> : null}
            </>}
            <h3 className="settings-section-gap">Credenciais (ficam só no chaveiro do sistema)</h3>
            <p className="settings-note">{(usingNextcloud ? credentialsConfigured : true) ? "Frase de criptografia já configurada nesta máquina." : "Ainda não configuradas nesta máquina."} A frase de criptografia é diferente de qualquer senha — ela nunca é enviada a lugar nenhum, só usada aqui para cifrar/decifrar. Deixe os campos em branco se só quiser atualizar a configuração sem trocar as credenciais já salvas.</p>
            {usingNextcloud ? <label className="settings-field"><span>Senha de app (App Password) do Nextcloud</span><input className="settings-text-input" type="password" value={appPassword} onChange={(event) => setAppPassword(event.target.value)} placeholder="Gerada em Configurações → Segurança no Nextcloud" /></label> : null}
            <label className="settings-field"><span>Frase de criptografia</span><input className="settings-text-input" type="password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} placeholder="Mínimo 8 caracteres" /></label>
            <label className="settings-field"><span>Confirmar frase de criptografia</span><input className="settings-text-input" type="password" value={passphraseConfirm} onChange={(event) => setPassphraseConfirm(event.target.value)} /></label>
            <div className="settings-card">
              <div><strong>{credentialStatus || "Um único clique salva provedor, Client ID, toggle e credenciais — nunca só parte da configuração."}</strong></div>
              <div className="settings-card-actions">
                {usingNextcloud && credentialsConfigured ? <button className="secondary-button" onClick={removeCredentials}>Remover credenciais</button> : null}
                <button className="primary-button" onClick={() => void saveCloudSection()}>Salvar configuração da nuvem</button>
              </div>
            </div>
            <div className="settings-card">
              <div><strong>{connectionStatus || "Testa se o app consegue falar com a nuvem configurada (use depois de salvar)."}</strong></div>
              <button className="secondary-button" onClick={() => void testConnection()}>Testar conexão</button>
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
