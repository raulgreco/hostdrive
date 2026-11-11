# Handoff da sessão — 2026-11-10/11

> Nota de continuidade da conversa entre o usuário (Raul) e o agente (Kimi Code).
> Ao retomar: leia este arquivo, depois `docs/implementation-status.md` (seção
> "Próximos passos (2026-11-11)"), `PRODUCT.md` e `DESIGN.md`.

## Quem é o projeto

**Eureka42 — Universal CLI** (antigo "HostDrive") — app desktop local-first
(Tauri 2 + Rust + React 19 + Vite + xterm.js + SQLite FTS5) para organizar
**qualquer** CLI de IA em projetos com PTY real, contexto universal entre
conversas e continuidade entre máquinas via nuvem do próprio usuário.
Pasta: `~/Desktop/UNIVERSAL/universal-ai-cli`. Site de marca: `eureka42.org`.
Identidade visual oficial: `~/Desktop/EUREKA42/Identidade Visual/brandbook/`.

## O que esta sessão entregou (tudo verificado: 31 testes Rust + 6 Vitest + builds)

1. **Agentes dinâmicos** — catálogo de 10 CLIs + agentes personalizados
   (`custom_agents`; qualquer executável; ids slug). UI só mostra agentes
   **conectados** ao projeto (`list_project_agents`). Retomada nativa só nas
   4 CLIs validadas (codex/claude/opencode/kimi).
2. **Contexto universal** (`src-tauri/src/context.rs`) — `.eureka42/contexto.md`
   agregado (memória + tarefas + 10 conversas recentes, nunca bytes do
   terminal); injetado em novas sessões do Claude Code via
   `--append-system-prompt` (única CLI com flag validada); toggle em
   Configurações → Privacidade; visão "Contexto" no painel Memória.
3. **Sync multi-provedor** (`oauth.rs`, `sync.rs`) — Google Drive, OneDrive
   (OAuth PKCE com redirect loopback; Client ID do usuário; tokens só no
   keyring; renovação automática) e Nextcloud (WebDAV); identidade = e-mail da
   conta; pacote de nível de app (preferências + agentes personalizados);
   **sync de arquivos do projeto** opt-in (manifesto local+remoto, nunca apaga,
   conflito sempre pergunta, exclusões de build/segredo, teto 4 MiB).
   Decisão: `docs/decisions/007-multi-cloud-sync.md`. Pasta na nuvem: `Eureka42`.
4. **Rebrand + redesign "Baralho de Chips"** — identidade Eureka42 (teal
   `#34D6BC` sobre noite `#0A0D13`, Inter/Montserrat 800/JetBrains Mono
   empacotadas em `src/assets/fonts/`); trilho de 5 fichas de cor
   (Terminal teal-luz, Histórico violeta, Memória âmbar, Git teal-42,
   Tarefas âmbar-fundo); splash 4s em meia escala; onboarding; tela **Sobre**
   com o manifesto oficial; ícones do app regenerados da marca.
   Direção escolhida pelo usuário na rodada Impeccable (seed `fa31b05b207211b6`;
   contrato em `.impeccable/surfaces/src-app-tsx.md`); sistema documentado em
   `DESIGN.md` + `.impeccable/design.json`.
5. **Site apontado para `eureka42.org`** (README + PRODUCT.md).

## Bugs corrigidos nesta sessão (importante — não regredir)

- **Logos não apareciam**: Vite embute SVG < 4KB como `data:` URI e o CSP do
  Tauri bloqueava `data:` em img. Corrigido em `tauri.conf.json`
  (`img-src ... data:`).
- **Seletor de agente cortado / botões mortos**: `.agent-tabs` tem
  `overflow-x: auto` que clipava o dropdown. Picker agora usa
  `position: fixed` ancorado no botão ＋ (`pickerPos` no TerminalPane).
- **Cache de build stale** (projeto foi movido de `~/Desktop/universal-ai-cli`
  para `~/Desktop/UNIVERSAL/universal-ai-cli`): se `cargo`/`tauri build`
  falharem com caminho antigo em `target/`, apagar `target/debug` e
  `target/release/build`.

## Decisões do usuário registradas

- Suporte a **todas** as CLIs; não-conectadas ficam invisíveis.
- Contexto **precisa** fluir entre conversas (ligado por padrão).
- Os **3 provedores** de nuvem com Client IDs próprios do usuário.
- Público-alvo: **qualquer dev que baixar** (first-run cuidadoso).
- Nome: **Eureka42 — Universal CLI**; splash 4s em 50%; botão Sobre com o
  manifesto; site `eureka42.org`; visual = Baralho de Chips.

## Pendente / próximo (não esquecer)

1. Testar OAuth/sync reais com contas (Google/OneDrive/Nextcloud) — `NOT_VERIFIED`.
2. Decidir Client IDs do projeto (login direto para usuários finais).
3. Teste real no Windows.
4. Renomear repo GitHub (decisão do usuário; afeta endpoint do updater).
5. Release v0.2.0 via tag (CI publica rascunho com macOS+Windows).

## Comandos úteis nesta máquina

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # cargo NÃO está no PATH padrão do shell
cd ~/Desktop/UNIVERSAL/universal-ai-cli
npm run build                           # tsc + vite
npm test                                # vitest
~/.cargo/bin/cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run tauri -- build                  # gera Eureka42.app + DMG
# abrir: open src-tauri/target/release/bundle/macos/Eureka42.app
# assinar updater localmente: export TAURI_SIGNING_PRIVATE_KEY=$(cat ~/.hostdrive-keys/hostdrive-updater.key)
```

Assinatura do updater: chave pública já em `tauri.conf.json`; privada em
`~/.hostdrive-keys/hostdrive-updater.key` (e secret no GitHub). CI assina
automaticamente; localmente o build termina com um "Error" de assinatura que
não afeta o `.app`/DMG.
