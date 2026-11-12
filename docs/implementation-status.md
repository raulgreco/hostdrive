# Estado da implementação

Atualizado em 2026-11-11. **Correções e melhorias de uso real** (relatadas pelo
usuário ao testar o app no Mac): botão Fechar nativo quebrado, fluxo de agentes
confuso, fontes pequenas, splash mais longo, histórico por projeto na sidebar.

**2026-11-11 — botão Fechar do macOS (root cause encontrado)**: o app testado
hoje de manhã era um binário **obsoleto** de `npm run tauri -- build` (00:15),
com o código antigo — o handler `onCloseRequested` fazia `preventDefault()` e
chamava `win.destroy()` **sem a permissão** `core:window:allow-destroy`, o
invoke era rejeitado e a janela nunca fechava. Correção: handler removido
(fechamento 100% nativo, instantâneo, verificado por AXPress em binário novo:
fecha em ~1s) e permissões de janela adicionadas (`allow-close/destroy/
minimize/toggle-maximize/is-decorated/start-dragging`) para uso futuro.
Aprendizado operacional: `cargo build` gera `target/release/universal-ai-cli`;
o binário `Eureka42` só é atualizado pelo `npm run tauri -- build` — testar o
caminho certo. Builds de cargo puro precisam da feature `custom-protocol`
(documentada em `Cargo.toml`), senão o app abre com webview branco.

**2026-11-11 — sync Nextcloud**: erro "HTTP 400" ao preparar a pasta agora ecoa
o corpo da resposta do servidor na mensagem (diagnóstico real, não apenas o
código). Causa raiz no servidor do usuário ainda em aberto — aguardando o
texto exato do erro.

**2026-11-11 — UX do fluxo de agentes**: o ＋ virou botão rotulado **"Escolha
seu agente"**; escolher um agente no picker **conecta direto** (inicia a sessão
na hora, retomando a sessão nativa quando a CLI validada suporta — botão
"Retomar" removido da UI); cada aba de agente ganhou **× de fechar** (encerra a
sessão ativa se for a aba corrente e esconde a aba — nunca apaga histórico);
mensagens do terminal refletem o estado real ("conectando… / sessão retomada /
agente conectado").

**2026-11-11 — legibilidade e primeira tela**: escala tipográfica global
aumentada (body 13→14px, labels 9→10px, ajustes em todos os painéis — DESIGN.md
atualizado); ícone de Configurações trocado de glifo Unicode para **SVG de
engrenagem** com área de toque 32px; splash de 4s para **7s**; sidebar mostra
**histórico por projeto** (total de sessões + último agente/data) e botão
rotulado "Novo projeto". Controles de janela desenhados pelo app
(`WindowControls`) ficam dormentes — só renderizam se a janela estiver sem
decoração nativa.

**2026-11-11 — padrão macOS de fechamento + silêncio no boot + chaveiro**:

- Fechar (×) agora **esconde** a janela e o app continua na Dock — padrão
  macOS (⌘Q encerra de verdade). Clique no ícone na Dock reabre a janela
  (`RunEvent::Reopen` no Rust). Fechamento-verificado: esconde e o processo
  segue vivo; reabertura via Dock: `NOT_VERIFIED` (não reproduzível via
  automação do bare binary).
- Sync automático **silencioso na abertura** do app: falha na nuvem não gera
  toast de erro nem de sucesso no lançamento (best-effort, console); em
  trocas de projeto feitas pelo usuário o status volta a aparecer.
- Chaveiro com **leituras memoizadas por processo** (`secret_cache`): antes,
  cada ponto do código que lia o keychain disparava um prompt potencial no
  macOS (~15 senhas relatadas); agora o máximo é um prompt por item por
  lançamento — e zero após "Permitir Sempre".

**2026-11-11 — picker de agentes + nuvem sem padrão**: o picker "Instalados nesta
máquina" lista **todos** os instalados — os já conectados aparecem como "Abrir"
(ausência de Claude/Codex na lista era filtro de já-conectados, não falha de
detecção: ambos estão em `~/.local/bin`, que já era pesquisado). Provedor de
nuvem **sem padrão**: `cloud_provider` default vazio no banco (era 'nextcloud');
com provedor vazio o app não toca o chaveiro nem sincroniza; `build_remote` e
`disconnect_cloud` rejeitam provedor vazio/desconhecido com erro claro em vez
de cair no Nextcloud.

**Verificações desta rodada**: 31 testes Rust + 6 Vitest + builds verdes;
app real executado no Mac com capturas de tela: UI nova completa renderizada
(abas com ×, "Escolha seu agente", histórico na sidebar, fontes maiores);
**fechamento nativo verificado (≤1s)**. Conexão de agente com clique real:
`NOT_VERIFIED` (automação não alcança botões do webview) — a ser confirmado
pelo usuário.

**Rodada anterior (2026-11-10) — resumo**: rebrand para **Eureka42 — Universal CLI** (identidade oficial em `~/Desktop/EUREKA42/Identidade Visual/brandbook/`), **agentes dinâmicos** (suporte a qualquer CLI), **contexto universal entre conversas**, **sync multi-provedor** (Google Drive/OneDrive/Nextcloud) com **arquivos de projeto**, e **redesign UI "Baralho de Chips"** com splash e tela Sobre. Ver `docs/decisions/007-multi-cloud-sync.md`.

**2026-11-10 — agentes dinâmicos**: catálogo ampliado (codex, claude, opencode, kimi, gemini, aider, cursor-agent, amp, goose, qwen) + agentes personalizados (`custom_agents`, qualquer executável do PATH; ids slug sem colisão). A UI só lista agentes **conectados** ao projeto (`list_project_agents` — agregação de sessões); os não conectados não aparecem, como pedido. `detect_agents` agora só retorna instalados. Retomada nativa segue restrita às 4 CLIs validadas.

**2026-11-10 — contexto universal** (`src-tauri/src/context.rs`): `.eureka42/contexto.md` agregado (memória + tarefas em aberto + metadata das 10 conversas recentes — nunca bytes do terminal) publicado no projeto ao iniciar/encerrar sessão e ao mudar memória/tarefas; injetado em novas sessões do Claude Code via `--append-system-prompt` (validado via `--help` em 2026-11-10; codex/opencode/kimi não têm flag equivalente para TUI — não alegado). Preferência `context_carryover_enabled` (ligado por padrão). Painel Memória ganhou visão "Contexto" (copiar/atualizar).

**2026-11-10 — sync multi-provedor** (`oauth.rs` + refactor de `sync.rs`): provedor único por vez (`cloud_provider`: nextcloud/google/onedrive); OAuth PKCE com redirect loopback e Client ID do usuário; tokens no keyring com renovação automática; escopos mínimos (drive.appdata / Files.ReadWrite.AppFolder); pacote de nível de app (preferências + agentes personalizados); **sync de arquivos do projeto** opt-in (manifesto local+remoto, nunca apaga, conflitos perguntam, exclusões de build/segredos, teto 4 MiB). Pasta na nuvem renomeada para `Eureka42`.

**2026-11-10 — redesign "Baralho de Chips"**: direção escolhida pelo usuário na rodada Impeccable (seed `fa31b05b207211b6`, contrato em `.impeccable/surfaces/src-app-tsx.md`). Fichas de cor por ferramenta (Terminal teal-luz, Histórico violeta, Memória âmbar, Git teal-42, Tarefas âmbar-fundo) na identidade Eureka42; fontes Inter/Montserrat/JetBrains Mono empacotadas localmente; splash oficial; onboarding de primeiro uso; tela Sobre com o manifesto. Tema escuro por padrão (marca), claro disponível.

**2026-10-18 — correções reportadas pelo usuário ao testar no Windows real** (`docs/decisions/004-windows-executable-detection-fix.md`): a detecção/lançamento de agentes (`agents::find_executable`) procurava só o nome puro do executável (`codex`, `claude`...), que nunca bate com os shims reais do Windows (`codex.exe`, `claude.cmd`, ...) — isso impedia o app de achar e iniciar qualquer CLI no Windows, o que por sua vez impedia qualquer login. Corrigido com resolução de `PATHEXT` e diretórios de instalação equivalentes do Windows. Também removidos os três botões vermelho/amarelo/verde de estilo macOS do cabeçalho — eram puramente decorativos, duplicavam os controles nativos da janela (já fornecidos pelo SO) e não faziam sentido fora do macOS.

**2026-10-18 — rebatizado para HostDrive**: nome do produto, identificador do app (`app.hostdrive.desktop`), binário principal, banco SQLite (`hostdrive.sqlite`) e ícone (extraído de `logo-1.png`, fornecido pelo usuário) atualizados em todo o projeto. Ver também a linha `CONTINUIDADE_CLOUD` abaixo: sincronização opcional via Nextcloud implementada nesta mesma rodada (`docs/decisions/005-nextcloud-sync.md`).

**2026-10-20 — repositório público e atualização automática**: projeto publicado em https://github.com/raulgreco/hostdrive (primeiro commit com todo o histórico de decisões). Atualização automática via `tauri-plugin-updater` contra `latest.json` das GitHub Releases, assinada (chave privada fora do repositório), com publicação automatizada em CI (`.github/workflows/release.yml`) mas Release sempre criada como rascunho — nenhuma versão chega aos usuários sem publicação manual. Detalhes em `docs/decisions/006-auto-update.md`.

| ID | Estado | Evidências | Testes | Observações |
|---|---|---|---|---|
| FASE-0 | IMPLEMENTED | `docs/feasibility.md` | Ferramentas e quatro CLIs inspecionadas | Windows build `NOT_VERIFIED` → ver REQ-001 |
| FASE-1 | IMPLEMENTED | React/Vite + Tauri/Rust empacotados | Build web, `.app`, ZIP e instalador Windows | Assinatura/notarização não configuradas |
| FASE-2 | IMPLEMENTED | PTY real, xterm.js, resize, entrada/saída, encerramento e até 6 abas | Shell e quatro TUIs abertas no release | ConPTY Windows `NOT_VERIFIED` (código compila para o target, execução não) |
| REQ-001 | PARTIAL | Via CI (GitHub Actions, build nativo em `macos-latest`/`windows-latest`, `.github/workflows/release.yml`): `.dmg` real, `.app.tar.gz`, instalador NSIS **e** MSI, todos assinados — ver release `v0.1.0`. Localmente neste Mac: `.app`/ZIP macOS e `.exe`/NSIS Windows cross-compilados (`docs/decisions/002-windows-cross-compilation.md`), usados só para desenvolvimento | DMG baixado da Release e validado com `hdiutil imageinfo` (UDIF válido); os dois jobs de CI terminaram verdes (macOS 6m28s, Windows 8m1s) | Execução real do instalador numa máquina Windows por uma pessoa (não só build) continua `NOT_VERIFIED` — falta confirmação humana, não é mais limitação de ferramental |
| REQ-002 | IMPLEMENTED | Projetos SQLite, seletor nativo, busca, favoritos e arquivo | Persistência após reinício |
| REQ-003 | IMPLEMENTED_MACOS | `agents.rs` | Codex, Claude, OpenCode e Kimi detectados com versão/path no macOS; resolução de `PATHEXT`/diretórios Windows adicionada e cobertura por teste `#[cfg(windows)]` (não executa neste host) | Detecção no Windows real continua `NOT_VERIFIED` — só foi testada no macOS; a correção parte do diagnóstico do usuário, não de execução direta em Windows |
| REQ-004 | IMPLEMENTED_MACOS | PTY allowlist e ambiente `xterm-256color` | Quatro CLIs reais iniciadas; prompts de confiança não aceitos | Código do Terminal Manager (via `portable-pty`, que seleciona ConPTY em Windows) compila para `x86_64-pc-windows-msvc`; execução real de ConPTY `NOT_VERIFIED`. Login interativo de cada CLI depende de REQ-003 achar o executável primeiro — ver correção em `docs/decisions/004-windows-executable-detection-fix.md` |
| REQ-005 | IMPLEMENTED | xterm.js, fit e até seis terminais independentes; atalho `Cmd/Ctrl+N` | Segunda aba criada na build release |
| REQ-006 | IMPLEMENTED | Sessões/eventos SQLite, captura configurável, detalhe de eventos e flag `resumed` | RUNNING/TERMINATED/INTERRUPTED e replay textual verificados |
| REQ-007 | IMPLEMENTED | `agents::resume_args` (codex `resume --last`, claude/opencode/kimi `--continue`, cada um validado via `--help` do binário instalado); botão "Retomar" e comando `///retomar` | `agents::tests::resume_is_only_claimed_for_verified_agents_and_unknown_ids_are_rejected`; round-trip de `resumed` em `projects::tests::records_and_summarizes_terminal_session_events` | Backend rejeita com erro visível se o agente não tiver retomada verificada — nunca finge suporte |
| REQ-008 | IMPLEMENTED | FTS5, projeto/agente/período/estado/texto e paginação | Busca encontrou sessão histórica correta; busca em dataset sintético (400 sessões/4.800 eventos) em ~40 ms — ver NFR-008 |
| REQ-009 | IMPLEMENTED | Memória estruturada, tarefas e proveniência | Painéis compilados e abertos no release |
| REQ-010 | IMPLEMENTED | Seletor por abas (já existia) + `OrchestratorBar` com `///agente <id>` (paleta de sugestões), atalho `Cmd/Ctrl+K` | `OrchestratorBar.test.ts`: comandos reconhecidos, texto comum nunca tratado como comando (T-018) | Troca de agente com sessão ativa é bloqueada com mensagem, não há encerramento automático (NFR-003) |
| REQ-011 | IMPLEMENTED | UAHP 1.0, schema, manifesto, memória, tarefas e contexto | Pacote `READY` gerado e inspecionado |
| REQ-012 | PARTIAL | Arquivo `context.md` explícito no pacote | Pacote disponível para revisão | Injeção automática por CLI não é alegada |
| REQ-013 | IMPLEMENTED | Handoff não encerra nem modifica sessão de origem | Geração ocorreu sem tocar no repositório |
| REQ-014 | IMPLEMENTED | Projetos, memória, busca e histórico locais | Nenhum backend próprio |
| REQ-015 | IMPLEMENTED | Exportação Markdown, JSON e TXT | Markdown exportado na build release |
| REQ-016 | IMPLEMENTED | Branch, HEAD e status Git somente leitura | Projeto sem Git tratado sem erro |
| REQ-017 | IMPLEMENTED | Agentes, versões, paths, sessões e armazenamento visíveis | Diagnóstico básico exercitado |
| REQ-018 | IMPLEMENTED | Arquivamento lógico | Diretório original preservado em teste |
| REQ-019 a REQ-024 | NOT_STARTED | — | — | Evolução explicitamente fora da v0.1.0 pelo próprio `08-PLANO-IMPLEMENTACAO.md` (Fase 9/10): splits paralelos, snapshots físicos, `///auto`, compactação assistida por IA, aliases por projeto. Não implementados nesta rodada para não expandir o escopo além do que foi pedido. |
| NFR-001/2/3 | IMPLEMENTED | Local-first, sem credenciais próprias e sem Git destrutivo | Revisão dos fluxos e QA |
| NFR-004 | IMPLEMENTED_BASE | WAL, FTS5, paginação, limites, recovery e retenção configurada | 13 testes Rust | Limpeza automática fica para evolução |
| NFR-005 | IMPLEMENTED | Backend em módulos Rust separados (`agents`, `terminal`, `projects`, `history`, `handoff`, `workspace`); frontend React só de apresentação, chamando Tauri via `invoke` tipado em `src/services/*` | Build e testes de ambos os lados passam isoladamente | Não havia entrada própria na tabela; adicionada nesta auditoria |
| NFR-006 | IMPLEMENTED | Temas Sistema/Claro/Escuro (`app_settings.theme`, tokens CSS em `src/styles.css`), atalhos `Cmd/Ctrl+K`/`N`/`,` e `Esc` em todos os diálogos, `aria-label`/`role` nos componentes interativos | Build visual verificada por inspeção de código e compilação; **não foi possível tirar screenshot nem usar automação de Acessibilidade neste ambiente** (sem permissão de Gravação de Tela/Acessibilidade concedível sem sessão gráfica) — ver `docs/decisions/003-resume-palette-theme.md` | Terminal (xterm) permanece escuro nos três temas por decisão deliberada (contraste das TUIs) |
| NFR-007 | IMPLEMENTED | `AgentInstallation.supportsResume` e `available`/`version` só refletem o que foi detectado/validado no binário instalado; nenhuma capacidade é assumida por versão | `agents::tests::detection_returns_a_safe_record_for_every_supported_agent` | Não havia entrada própria na tabela; adicionada nesta auditoria |
| NFR-008 | MEASURED | `projects::tests::fts5_search_stays_within_budget_on_synthetic_dataset`: busca FTS5 em 400 sessões / 4.800 eventos sintéticos | **~40 ms** medidos nesta máquina (Apple M4, Mac mini, 24 GB, 2026-10-13), meta de 500 ms | Não é garantia para todo hardware; "UI inicial <3s" continua sem medição automatizada (exigiria cronômetro em sessão gráfica real) — `NOT_VERIFIED` |
| NFR-009 | IMPLEMENTED | `tauri build`/`cargo xwin build` geram binário e bundle standalone; Node/Vite só são necessários em tempo de build, não no app instalado | `.app`/ZIP e instalador Windows não dependem de Node no runtime | Não havia entrada própria na tabela; adicionada nesta auditoria |
| CONFIG | IMPLEMENTED | Conta local, privacidade, captura, tema e retenção planejada | Preferência salva e relida no release |
| CONTINUIDADE_CLOUD | IMPLEMENTED_BASE | Sincronização via Nextcloud/WebDAV opcional (`docs/decisions/005-nextcloud-sync.md`): memória, tarefas, UAHP e preferências, cifrados no cliente (AES-256-GCM + Argon2), credenciais só no chaveiro do SO, histórico de terminal opt-in por projeto, automática ao trocar/fechar projeto, conflito nunca decidido silenciosamente | 19 testes Rust incluindo round-trip de criptografia; cross-compila para Windows sem ajuste | Chamadas WebDAV reais contra um Nextcloud de verdade `NOT_VERIFIED` — sem instância disponível neste ambiente para testar |
| ATUALIZAÇÃO_AUTOMÁTICA | IMPLEMENTED_BASE | `tauri-plugin-updater` + GitHub Releases (`docs/decisions/006-auto-update.md`); aviso não-intrusivo, download/instalação só por ação explícita do usuário, pacotes assinados (chave privada fora do repositório), CI builda e publica como rascunho | App compila e abre normalmente nos dois SOs com os plugins registrados; bundle `.app.tar.gz` do updater gerado corretamente | Fluxo completo ponta a ponta (detectar versão nova de verdade e instalar) `NOT_VERIFIED` — depende do primeiro release real publicado |

## Evidência final

- `cargo test`: **19 aprovados**, 0 falhas (13 anteriores + 1 de detecção de executável Windows (`#[cfg(not(windows))]`, roda neste host) + 5 de `sync.rs`: criptografia, senha errada, blob corrompido, codificação percentual, validação de credenciais).
- `npm test`: **6 aprovados**, 0 falhas (2 anteriores + 4 de `OrchestratorBar.test.ts`).
- `npm run build`: concluído com Vite 8.3.4, sem erros de TypeScript nos componentes novos (`OrchestratorBar`, tema, atalhos).
- macOS 27.0.1 Apple Silicon: binário `.app` relançado após todas as mudanças rodou **3 s em primeiro plano sem panics nem erros no stdout/stderr** (migração de schema incluída: colunas `resumed` e `theme` aplicadas sem falha). Verificação visual por screenshot/Acessibilidade **não foi possível neste ambiente** (sandbox sem permissão de Gravação de Tela nem Acessibilidade, que exigem concessão manual numa sessão gráfica interativa) — reportado como tal, não alegado como testado visualmente.
- Codex: TUI abriu; prompt de confiança/update não foi confirmado.
- Claude: TUI abriu no cwd correto; prompt de confiança não foi confirmado.
- OpenCode: TUI interativa abriu no cwd correto.
- Kimi: TUI abriu no cwd correto; prompt de confiança não foi confirmado.
- Histórico: persistência após reinício, FTS5, filtros, detalhes e exportação Markdown exercitados.
- UAHP: pacote `READY` gerado atomicamente; nenhum `.env`, chave ou arquivo do projeto foi coletado.
- macOS: assinatura **ad-hoc**; Apple Developer/notarização não configuradas. `.app` empacotado e comprimido em ZIP (`HostDrive_0.1.0_macOS-arm64.zip`); DMG não gerado neste ambiente (ver REQ-001).
- **Windows:** toolchain de cross-compilação neste Mac (`clang-cl`, `lld-link`, `llvm-rc` via Homebrew `llvm`/`lld`; Windows SDK/CRT via `xwin`; orquestração via `cargo-xwin`; detalhes em `docs/decisions/002-windows-cross-compilation.md`). `npm run tauri -- build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis` gerou `universal-ai-cli.exe` e `universal_ai_cli_lib.dll` (PE32+, subsystem GUI confirmado) e o instalador `HostDrive_0.1.0_x64-setup.exe` (NSIS, 4,52 MiB, com seletor de idioma PT-BR/EN, modo de instalação "both" e bootstrapper do WebView2 embutido). MSI (WiX) não é produzido neste host. Execução real do `.exe` em Windows permanece `NOT_VERIFIED`.

## Resultado

A primeira versão funcional para macOS está concluída, incluindo os três requisitos (REQ-007, REQ-010, NFR-006) encontrados pendentes numa auditoria desta mesma data. A build Windows é gerada e cross-verificada como binário válido a partir deste Mac, com instalador NSIS amigável (idioma, modo de instalação, WebView2 embutido), mas sua execução funcional depende de um host Windows real para ser `IMPLEMENTED` — isso é reportado como `NOT_VERIFIED`, não alegado como testado. REQ-019 a REQ-024 são evolução explicitamente fora de escopo pelo próprio plano do projeto e não foram implementados. Itens `PARTIAL` indicam limitações de ambiente ou automação deliberadamente não alegada, não pendências do pacote macOS local-first.

## Próximos passos (2026-11-11)

1. **Testar o fluxo de nuvem com contas reais** (Google Drive / OneDrive / Nextcloud): criar os Client IDs OAuth (guia embutido em Configurações → Nuvem) e exercitar login, sync de dados e sync de arquivos. Ainda `NOT_VERIFIED`.
2. **Decisão pendente do usuário**: registrar Client IDs próprios do projeto (Google Cloud + Microsoft Entra) para usuários finais conectarem sem ver código — ou manter o modelo "cada um com seu ID".
3. **Teste real no Windows** (execução do instalador NSIS, ConPTY, detecção de agentes) — CI builda, execução humana pendente.
4. **Site**: apontar para eureka42.org (feito nos arquivos; o domínio precisa existir/apontar).
5. **Repositório**: decidir renomear `github.com/raulgreco/hostdrive` para a marca (afeta o endpoint do auto-update; o updater publica em rascunho, então a troca é segura antes da primeira release pública).
6. Publicar release v0.2.0 (tag) quando os itens 1–3 tiverem evidência — o CI gera macOS + Windows automaticamente.
