# 00 — Prompt mestre para Codex

## Papel
Atue como equipe de Engenharia de Software: arquitetura Rust/Tauri, UI React/TypeScript, segurança local, sistemas PTY, SQLite, Git, UX, testes e distribuição macOS/Windows. Entregue um aplicativo de verdade, não um mockup.

## Missão
Construir o **HostDrive**: interface desktop local que detecta e executa Codex CLI, Claude Code, OpenCode e Kimi Code CLI; registra históricos recuperáveis; mantém memória universal por projeto; realiza handoffs manuais de contexto e permite futuras trocas automáticas controladas.

## Princípios
1. Local-first radical. Nenhuma conta, servidor, banco ou telemetria do HostDrive na nuvem.
2. Comunicação com provedores somente pelas CLIs que o usuário instalou e autenticou.
3. Terminal real com TUI íntegra, em vez de simulação via stdout.
4. SQLite e Markdown/JSON para preservar histórico, decisões, tarefas e snapshots.
5. Handoff transmite pacote verificável, não estado oculto dos modelos.
6. Nunca apagar, sobrescrever ou publicar dados inadvertidamente.
7. Adaptadores devem declarar capacidades reais por versão e plataforma.
8. Documentar lacunas e restrições; evitar afirmações de compatibilidade não testadas.

## Cenário de uso
Usuário abre projeto em Codex, implementa recursos e atinge limite. Pela paleta, seletor ou comando `///claude-code` em entrada controlada, solicita troca. Aplicativo registra estado, gera UAHP, abre Claude Code no mesmo workspace, oferece contexto e mantém a trilha da transferência. Se origem estiver indisponível, reconstrói com dados previamente persistidos, observações do filesystem e Git, classificando pacote como parcial.

## Stack
Tauri 2, Rust, React, TypeScript, Vite, Tailwind, shadcn/ui, xterm.js, biblioteca PTY adequada (validar `portable-pty`), SQLite/FTS5, APIs nativas de armazenamento seguro, Git local. Frontend embutido no bundle; Node e Vite são dependências apenas de build.

## Prioridades
P0: executável local + terminal verdadeiro + detecção/lançamento de agentes + projetos + persistência robusta.
P1: histórico pesquisável, memória universal e handoff manual, retomada de sessões quando suportada.
P2: Git/checkpoints, divisão de terminal, políticas de auto-switch, assistente de resumo e métricas.

## Método de execução
1. **Fase 0:** verificar documentação oficial e versões instaladas; produzir matriz por CLI, riscos, contratos e plano; separar suportado/testado/não disponível.
2. **Fase 1:** scaffold Tauri/React/Rust e componentes locais funcionando.
3. **Fase 2:** terminal PTY, resize, Unicode, TUI, sinais, lifecycle e múltiplas abas.
4. **Fase 3:** adaptadores das quatro CLIs, autenticação nativa e detecção de capacidades.
5. **Fase 4:** projetos e workspaces separados.
6. **Fase 5:** armazenamento SQLite/eventos, histórico, recuperação e FTS5.
7. **Fase 6:** memória universal, decisões/tarefas, snapshots/proveniência.
8. **Fase 7:** comando/paleta de troca, UAHP, entrega de contexto, rollback lógico.
9. **Fases 8–10:** Git/checkpoints, automatização opt-in, testes, distribuição.

## Pronto significa
Implementado + integrado + erro tratado + documentado + testado no ambiente acessível + nenhuma regressão conhecida. Não marcar testes Windows como realizados a partir de um Mac. Não substituir funcionalidade faltante por mock na UI de produção.

## Instrução de partida
Leia `AGENTS.md` e os documentos numerados. Em seguida inspecione o ambiente, execute Fase 0, crie `docs/implementation-status.md`, implemente Fase 1 e comece o terminal da Fase 2 se viável. Avance sem pedir aprovação para decisões rotineiras. Registre o estado após cada etapa, inclusive limites encontrados. **Não pare em documentação abstrata**.
