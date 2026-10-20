# 01 — Requisitos funcionais e não funcionais

## Rastreabilidade
Identificadores estáveis; estados: `NOT_STARTED`, `IN_PROGRESS`, `IMPLEMENTED`, `TESTED`, `BLOCKED` e `NOT_VERIFIED`. Cada requisito deve apontar testes e arquivos de implementação no `implementation-status.md`.

### Funcionais — essenciais
- **REQ-001:** Executar localmente macOS e Windows com pacote instalável.
- **REQ-002:** Abrir diretório como projeto; listar recentes/favoritos e selecionar agente padrão.
- **REQ-003:** Descobrir executáveis `codex`, `claude`, `opencode`, `kimi`, suas versões e caminhos configuráveis.
- **REQ-004:** Abrir cada CLI real por pseudoterminal, preservando interatividade e recursos próprios.
- **REQ-005:** Suportar múltiplas sessões/abas, resize, Unicode, ANSI, copiar/colar e Ctrl+C.
- **REQ-006:** Persistir eventos de sessões localmente; marcar campos capturados parcialmente.
- **REQ-007:** Retomar sessões nativas apenas quando identificador e comando forem suportados.
- **REQ-008:** Buscar histórico por projeto, agente, data e texto, via FTS5 e paginação.
- **REQ-009:** Manter memória independente: identidade, requisitos, decisões, tarefas, estado e proveniência.
- **REQ-010:** Mudar agentes via paleta e seletor; `///agente` apenas em modo de entrada sob controle do app.
- **REQ-011:** Gerar pacote UAHP versionado antes de handoff e registrar sucesso/falhas.
- **REQ-012:** Entregar contexto usando mecanismo oficialmente suportado ou arquivo explícito.
- **REQ-013:** Preservar sessão existente em caso de falha no agente de destino.
- **REQ-014:** Suportar histórico e consulta de projetos sem conexão com a internet.
- **REQ-015:** Exportar histórico/contexto em Markdown, JSON e texto.
- **REQ-016:** Listar Git branch, HEAD, alterações e diffs sem realizar commits automáticos.
- **REQ-017:** Mostrar estado e diagnóstico local de agentes/processos/armazenamento.
- **REQ-018:** Excluir ou arquivar projeto da lista sem remover diretório do usuário.

### Funcionais — evolução
- **REQ-019:** Terminais divididos e execução paralela com alertas de arquivos sobrepostos.
- **REQ-020:** Checkpoints lógicos e snapshots físicos opt-in com cotas e exclusões.
- **REQ-021:** `///auto` para seleção de destino com autorização explícita.
- **REQ-022:** Identificação de falhas/limites somente por eventos documentados e verificáveis.
- **REQ-023:** Compactação determinística e resumos assistidos por IA opcionais.
- **REQ-024:** Políticas de preferências e aliases por projeto.

### Não funcionais
- **NFR-001:** Sem backend próprio, login, sincronização ou telemetria obrigatória.
- **NFR-002:** Segredos não armazenados em texto puro; respeitar mecanismos dos fornecedores.
- **NFR-003:** Sem comandos destrutivos, stashes/commits ou permissões elevadas automáticos.
- **NFR-004:** Histórico com transações, paginação, limites, retenção explícita e recuperação após falhas.
- **NFR-005:** Backend modular Rust; frontend de apresentação, IPC tipado e permissões mínimas.
- **NFR-006:** Acessibilidade, atalhos, temas claro/escuro e estados de erro compreensíveis.
- **NFR-007:** Compatibilidade declarada por CLI+versão+SO; não mascarar recursos ausentes.
- **NFR-008:** Metas de desempenho a medir em hardware de referência: UI inicial <3s, busca comum <500ms; sem garantias antes de benchmark.
- **NFR-009:** Manter build distribuível sem Node/Vite em execução no dispositivo do usuário.

## Fora do escopo inicial
Execução local dos próprios LLMs, login universal aos quatro fornecedores, conversão fiel de memória oculta, instalação silenciosa das CLIs, edição destrutiva de repositórios, sincronização multiusuário, cobrança/contabilização de tokens sem dados oficiais.
