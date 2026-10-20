# 08 — Plano de implementação e marcos

## Governança
Desenvolver incrementalmente. Cada fase termina com demonstração reproduzível, testes, documentação e atualização em `docs/implementation-status.md`. Não afirmar certificação de SO não disponível.

### Fase 0 — Viabilidade
Inspecionar sistemas, versões, docs oficiais das CLIs, capacidades de PTY, formato de sessão, Windows/WSL, segurança Tauri. Documentar `feasibility.md`, `agent-compatibility.md`, `architecture.md`, `risks.md` e ADRs. **Aceite:** contratos e limitações explícitos.

### Fase 1 — Scaffold
Tauri 2, Rust, React TS, Vite, Tailwind, navegação, tema, IPC tipado, organização por features, suíte de teste. **Aceite:** app abre sem servidor externo e persiste config.

### Fase 2 — Terminal
PTY/ConPTY, xterm.js, lifecycle, ANSI, resize, Unicode, TUI, buffers, abas. **Aceite:** processo interativo real funciona e não congela UI.

### Fase 3 — Agentes
Detectar as quatro CLIs; caminhos, versões, suporte de login e retomada, wrappers adaptadores. **Aceite:** cada ferramenta instalada executa independentemente; ausentes exibem mensagem clara.

### Fase 4 — Projetos
Cadastro, pasta de trabalho, recentes, favoritos, sessões independentes, escolha de agente. **Aceite:** múltiplos workspaces sem contaminação de estado.

### Fase 5 — Persistência
SQLite migrations, sessões, eventos, busca FTS5, exportações, recuperação de queda, retenção. **Aceite:** dados sobrevivem reinício e podem ser filtrados.

### Fase 6 — Memória
Markdown/JSON, decisões, tarefas, snapshots, provenance, materialização incremental. **Aceite:** contexto reconstruído com fontes identificadas.

### Fase 7 — Handoff manual
UAHP, seleção de destino, pacote, prévia, entrega, estados transacionais, falha reversível. **Aceite:** Codex→Claude e outros pares testados quando executáveis disponíveis; sem perda de estado original.

### Fase 8 — Git e checkpoints
Diff, branch/HEAD, alertas dirty state, snapshots lógicos. **Aceite:** arquivos alterados visíveis e nenhuma ação destrutiva automática.

### Fase 9 — Automação opt-in
`///auto`, política de ordem, erro confirmado e troca autorizada. **Aceite:** nenhuma troca por falsa suposição de esgotamento.

### Fase 10 — Distribuição
Build macOS/Windows, assinaturas quando configuradas, QA, documentação, instaladores e testes de upgrade. **Aceite:** app instalável sem Node/Vite de desenvolvimento e sem backend próprio.

## Status obrigatório
Criar `docs/implementation-status.md` com tabela `ID | Estado | Evidências | Testes | Observações` e itens por fase. Reportar SO testado, comandos usados, falhas e próximos passos.

## Riscos prioritários
Interfaces/flags mudam entre versões das CLIs; TUI e entrada `///`; extração incompleta de histórico; contaminação de contexto/segredos; Windows ConPTY/WSL; controle de processos órfãos; armazenamento volumoso; conflitos de edições paralelas. Mitigar por adaptadores, capabilities, testes end-to-end, UX transparente e fallback manual.
