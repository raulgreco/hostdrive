# ADR 003 — Retomada de sessão, paleta `///agente` e tema claro/escuro

## Estado

Aceito e implementado em 2026-10-13, fechando as lacunas de REQ-007, REQ-010 e NFR-006 registradas em `docs/implementation-status.md`.

## Contexto

Uma auditoria dos documentos `01-REQUISITOS.md`/`09-TESTES-E-ACEITE.md` contra o código mostrou três requisitos essenciais sem implementação e sem rastro na tabela de status:

- **REQ-007** — retomar sessões nativas apenas quando "identificador e comando forem suportados".
- **REQ-010** — trocar agentes via paleta/seletor, com `///agente` funcionando só em modo de entrada controlado pelo app (T-018 exige que não interfira na TUI).
- **NFR-006** — temas claro/escuro, atalhos de teclado, estados compreensíveis.

## Decisão

### REQ-007 — retomada
Em vez de o app rastrear identificadores de sessão nativos (o que exigiria integrações específicas e frágeis por versão), usamos exatamente os comandos/flags que cada CLI já expõe para continuar a sessão mais recente *no diretório de trabalho atual*, validados lendo `--help` dos binários instalados em 2026-10-13:

| Agente | Comando |
|---|---|
| Codex | `codex resume --last` |
| Claude | `claude --continue` |
| OpenCode | `opencode --continue` |
| Kimi | `kimi --continue` |

`agents::resume_args` é a única fonte dessas flags (`src-tauri/src/agents.rs`); `launch_terminal` rejeita a retomada com erro visível se o agente não estiver nessa lista — nunca finge suporte. O backend persiste `resumed` em `terminal_sessions` (migração `ensure_column`, retrocompatível com bancos existentes) para o histórico mostrar quais sessões foram continuações.

### REQ-010 — paleta e `///agente`
A troca de agente já existia como seletor (abas em `TerminalPane`). Adicionamos o comando textual em `OrchestratorBar` (`src/components/OrchestratorBar.tsx`), um campo de texto **separado do xterm** — nunca lê o stream do PTY nem escreve nele — que reconhece `///agente <id>`, `///retomar` e `///ajuda`. Qualquer texto que não comece com `///` é ignorado pela barra (mensagem explicando que deve ser digitado no terminal), satisfazendo T-018 por construção, não por filtro heurístico. `Cmd/Ctrl+K` foca essa barra; digitar `///agente` sem completar abre uma lista (paleta) dos agentes disponíveis diferentes do atual.

Trocar de agente com uma sessão em andamento é bloqueado com mensagem clara — não há stop automático do processo (mantém NFR-003: nada de encerramento automático sem confirmação).

### NFR-006 — tema e atalhos
Tokens de cor (`src/styles.css`, bloco `:root`) substituem os literais hexadecimais espalhados pelo CSS. Três modos: `system` (segue `prefers-color-scheme`, inclusive em tempo real via `matchMedia` listener), `light` e `dark`, persistidos em `app_settings.theme` (migração igual à de `resumed`). **Decisão deliberada:** o terminal (xterm e a seção `.terminal`) permanece escuro nos três modos — é convenção comum de emuladores de terminal e evita perder contraste das TUIs das quatro CLIs; só o chrome do app (painéis, abas, diálogos) muda. Atalhos adicionados: `Cmd/Ctrl+K` (paleta), `Cmd/Ctrl+N` (novo terminal), `Cmd/Ctrl+,` (Configurações), `Esc` (fechar diálogo em foco — estendido a Handoff e detalhe de sessão, que não tinham). Lista visível em Configurações → Geral.

## Consequências

- Se uma CLI mudar a flag de retomada em versão futura, o comando falha de forma visível (erro do processo), não silenciosa — é preciso atualizar `resume_args` e revalidar via `--help` antes de religar o suporte.
- `SessionSummary.resumed` e `AppSettings.theme` são campos novos na API Tauri; o frontend já foi atualizado (`src/services/terminal.ts`, `src/services/settings.ts`).
- Cobertura de teste: `agents::tests::resume_is_only_claimed_for_verified_agents_and_unknown_ids_are_rejected`, round-trip de `resumed`/`theme` em `projects.rs`, e `OrchestratorBar.test.ts` garantindo que texto comum nunca é interpretado como comando.
- NFR-008 (performance) deixou de ser alegado sem medição: `projects::tests::fts5_search_stays_within_budget_on_synthetic_dataset` mede a busca FTS5 contra um dataset sintético (400 sessões / 4.800 eventos) nesta máquina de referência (Apple M4, Mac mini, 24 GB) — resultado real: **~40 ms**, bem abaixo da meta de 500 ms. Não é garantia para todo hardware, é a evidência que o projeto exige antes de marcar a meta como atingida.
