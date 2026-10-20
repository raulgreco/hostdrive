# 02 — Arquitetura local-first

## Visão
```
Tauri Window / React + xterm.js
        │ IPC tipado + eventos controlados
Rust Orchestrator
 ├── Terminal Manager (PTY / ConPTY)
 ├── Process/Session Manager
 ├── Agent Registry + adapters
 ├── Context/Handoff Engine
 ├── Project and Git Services
 ├── SQLite Repository + FTS5
 ├── Filesystem (Markdown/JSON/snapshots)
 └── Security/Diagnostics
          │ subprocessos do usuário
 codex | claude | opencode | kimi
          │ conexões oficiais, quando exigidas
     Provedores de modelos
```

A aplicação NÃO possui serviços de nuvem próprios. Vite/React são empacotados na WebView nativa; nunca exigir servidor Node em produção.

## Delimitação das camadas
- **UI:** telas, terminal renderer, atalhos e estados; nunca detém política de segurança ou credenciais.
- **Rust Core:** ciclo de vida de processos, comando/IPC, orquestração, acesso a arquivos e validações.
- **Agent Adapters:** comandos oficiais por ferramenta/versão; não acessar formatos privados sem justificativa.
- **Storage:** migrations transacionais e abstrações testáveis; arquivos de memória versionados.
- **Handoff:** máquina de estados persistida; idempotência e integridade.

## Contratos / IPC
Exemplos tipados: `list_projects`, `open_project`, `detect_agents`, `launch_agent`, `resize_terminal`, `send_terminal_input`, `close_terminal`, `list_sessions`, `search_history`, `prepare_handoff`, `commit_handoff`, `read_context`. Os handlers devem validar projeto, path, IDs, escopo e permissões. Não expor shell arbitrário via IPC genérico.

## Machine states
Process: `DETECTED -> STARTING -> RUNNING -> EXITING -> EXITED | FAILED`; app reiniciado não pode afirmar que processo antigo continua ativo sem checagem. Handoff: `REQUESTED -> PREPARING -> READY -> STARTING_DESTINATION -> CONTEXT_DELIVERED -> COMPLETED` com `PARTIAL`, `FAILED`, `CANCELLED` e ações de recuperação.

## Dados e diretórios
Dados globais em diretório por SO fornecido pela API Tauri; projetos referenciados por caminho canonizado e identificador estável. No repositório, criar `.universal-ai` apenas se usuário consentir; nunca inserir tokens ou logs privados nele.

## Design extensível
Interfaces Rust por agente e repositório; capabilities opcionais. Evitar abstrações prematuras e microsserviços. Preferir crates consolidados; registrar ADRs antes de mudanças arquiteturais relevantes.

## Compatibilidade
Mac PTY Unix e Windows ConPTY não são idênticos; isolar no Terminal Manager. WSL deve ser ambiente separado, com seu executável, caminhos e credenciais específicos. Avaliar arquitetura arm64/x64 nas builds.
