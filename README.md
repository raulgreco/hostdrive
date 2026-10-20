# HostDrive

**Site oficial:** [www.hostdrive.inf.br](https://www.hostdrive.inf.br)

Aplicativo desktop local-first para organizar **Codex CLI, Claude Code, OpenCode e Kimi Code CLI** em projetos, terminais PTY reais e um histórico compartilhado independente dos fornecedores.

## Versão 0.1.0

- Detecção local das quatro CLIs e versões instaladas.
- Até seis terminais independentes por projeto.
- Projetos recentes, busca, favoritos e arquivamento lógico.
- Histórico SQLite com FTS5, filtros, detalhes e exportação Markdown/JSON/TXT.
- Memória estruturada e tarefas por projeto com proveniência.
- Estado Git somente leitura, sem commits, stash ou reset automáticos.
- Pacotes UAHP 1.0 para continuidade explícita entre agentes.
- Configurações locais de privacidade, captura e tema (sistema/claro/escuro).
- Retomada de sessão nativa quando o agente expõe o comando (`codex resume --last`, `--continue` em Claude/OpenCode/Kimi).
- Comando do orquestrador `///agente <id>` / `///retomar` / `///ajuda` com paleta de sugestões, sem nunca interferir no stream do terminal.
- Atalhos: `Cmd/Ctrl+K` (comando do orquestrador), `Cmd/Ctrl+N` (novo terminal), `Cmd/Ctrl+,` (Configurações), `Esc` (fechar diálogo).
- Sincronização opcional via **Nextcloud** (WebDAV): memória, tarefas, pacotes UAHP e preferências, cifrados no cliente antes de saírem da máquina; histórico de terminal é opt-in por projeto (pode conter segredos). Ver `docs/decisions/005-nextcloud-sync.md`.
- Funcionamento sem conta própria; a sincronização usa a conta Nextcloud que o usuário já tem, nunca um backend do HostDrive.

## Privacidade

O núcleo, o SQLite, a memória e os pacotes ficam no computador. As CLIs oficiais podem acessar seus próprios provedores conforme a autenticação do usuário. Credenciais não são copiadas entre agentes. A redação de exportações é básica e toda exportação deve ser revisada antes de compartilhamento.

## Desenvolvimento

```bash
npm install
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

## Distribuição

### macOS

A primeira build validada é macOS Apple Silicon (`arm64`). O pacote atual usa **assinatura ad-hoc** (sem certificado Apple Developer nem notarização).

**Requisitos para instalar/testar no macOS:**
- macOS com Apple Silicon (M1/M2/M3/M4) — a build atual não é `universal` nem `x86_64`, só `arm64`.
- Como o app não é notarizado, o Gatekeeper vai bloquear a abertura direta (duplo clique mostra "não pode ser aberto"). Para abrir: **clique com o botão direito (ou Control+clique) no app → Abrir**, e confirme no diálogo. É preciso fazer isso apenas na primeira execução.
- Sem dependências externas: não precisa de Node, Rust ou Xcode instalados para *rodar* o app — só para compilar.
- As quatro CLIs (Codex, Claude Code, OpenCode, Kimi) são detectadas automaticamente se estiverem no `PATH`, `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/.kimi-code/bin` ou `~/.cargo/bin`; nenhuma é obrigatória para abrir o app.

Build/empacotamento:

```bash
npm run tauri -- build --bundles app,dmg   # ou só "app" se o DMG falhar (ver abaixo)
```

Neste ambiente de desenvolvimento específico, a geração do **DMG** falha porque o passo de "deixar bonito" do `create-dmg` pede ao Finder para posicionar ícones via AppleScript, e esse processo sandbox não tem permissão de Acessibilidade concedível sem uma sessão gráfica interativa (erro `-1712`, timeout de AppleEvent). Isso **não é um bug do app** — é uma limitação deste ambiente de automação. Alternativa usada: compactar o `.app` em ZIP, que se instala do mesmo jeito (descompactar e arrastar para `/Applications`):

```bash
cd src-tauri/target/release/bundle/macos
ditto -c -k --sequesterRsrc --keepParent "HostDrive.app" "HostDrive_0.1.0_macOS-arm64.zip"
```

Em uma máquina com sessão de usuário normal (não este sandbox), `--bundles app,dmg` deve gerar o DMG sem esse problema.

### Windows

A build Windows x64 (`.exe`, `.dll` e instalador NSIS) é gerada por **cross-compilação a partir deste Mac** (`docs/decisions/002-windows-cross-compilation.md`):

```bash
npm run tauri -- build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
```

Requer `rustup target add x86_64-pc-windows-msvc`, `brew install llvm lld makensis` e `cargo install xwin cargo-xwin` previamente.

O instalador (`HostDrive_0.1.0_x64-setup.exe`) foi configurado para ser amigável a quem vai testar (`src-tauri/tauri.conf.json`):
- Seletor de idioma na abertura (Português do Brasil / Inglês).
- Escolha entre instalar só para o usuário atual ou para todos (não força administrador a menos que a pessoa escolha "todos os usuários").
- Bootstrapper do WebView2 embutido no instalador — na maioria das máquinas Windows 10/11 atuais o WebView2 já vem pré-instalado, então esse passo não faz nada perceptível; se faltar, o instalador busca e instala automaticamente (precisa de internet nesse caso).
- Ícone próprio do app (antes o bundle não tinha nenhum ícone configurado — corrigido).

Os binários são PE32+ válidos e o instalador é um NSIS PE32 GUI válido (tudo verificado neste Mac), mas a execução real em Windows (abrir janela, WebView2, PTY/ConPTY com as quatro CLIs) permanece `NOT_VERIFIED` — não há host ou VM Windows disponível para testar aqui. Assinatura de código e instalador MSI (WiX) também não estão disponíveis neste host; só NSIS.

## Sincronização via Nextcloud

Em Configurações → Nextcloud: informe o endereço do servidor, o usuário, uma **senha de app** (gerada em Configurações → Segurança no próprio Nextcloud, nunca a senha principal da conta) e uma **frase de criptografia** própria (cifra os dados antes de saírem da máquina — nem o Nextcloud vê o conteúdo). As duas credenciais ficam só no chaveiro do sistema operacional. Com a sincronização ativada, abrir ou trocar de projeto traz o que há de mais novo; fechar a janela ou trocar de projeto envia o que mudou. Em caso de mudanças nas duas pontas ao mesmo tempo, o app pergunta qual versão manter — nunca decide sozinho. Histórico de terminal fica de fora por padrão; ative por projeto se quiser incluí-lo. Detalhes completos e o que fica **de fora** da sincronização em `docs/decisions/005-nextcloud-sync.md`.

## Atualização automática

O app consulta `https://github.com/raulgreco/hostdrive/releases/latest/download/latest.json` ao abrir; se houver versão mais nova, mostra um aviso — **nada é baixado ou instalado sem o usuário clicar "Atualizar agora"**. Toda atualização é assinada; o app recusa instalar qualquer pacote sem a assinatura correta. Detalhes em `docs/decisions/006-auto-update.md`.

**Para publicar uma versão nova:**
1. Atualize `"version"` em `src-tauri/tauri.conf.json`.
2. `git commit` e `git push`.
3. `git tag v0.1.1 && git push origin v0.1.1` (troque pelo número real).
4. O GitHub Actions builda macOS e Windows automaticamente e cria uma **Release em rascunho** com os instaladores e o `latest.json` assinados.
5. Revise o rascunho em https://github.com/raulgreco/hostdrive/releases e clique em **Publish release** — só a partir daí os usuários recebem o aviso de atualização.

A chave privada de assinatura vive em `~/.hostdrive-keys/hostdrive-updater.key` (fora do repositório) e como secret `TAURI_SIGNING_PRIVATE_KEY` no GitHub — **faça backup desse arquivo**; perdê-lo significa não poder mais assinar atualizações para quem já instalou o app.

Consulte `docs/implementation-status.md` para evidências e limitações verificadas, `docs/decisions/003-resume-palette-theme.md` para as decisões de retomada de sessão/paleta/tema, `docs/decisions/004-windows-executable-detection-fix.md` para a correção de detecção de agentes no Windows, `docs/decisions/005-nextcloud-sync.md` para a sincronização, e `docs/decisions/006-auto-update.md` para a atualização automática.
