# 04 — Memória universal, histórico e busca

## Objetivo
Manter **memória do projeto independente do fornecedor**. Armazenar identidade, objetivos, stack, requisitos, ADRs, decisões do usuário, estado verificado, tarefas pendentes, histórico de sessões e pacote de continuidade. Não prometer clonagem da memória interna de modelos.

## Entidades SQLite
`projects`, `workspaces`, `agents`, `agent_installations`, `agent_sessions`, `terminal_sessions`, `terminal_events`, `conversation_entries`, `context_snapshots`, `architectural_decisions`, `project_tasks`, `handoffs`, `file_changes`, `checkpoints`, `application_settings`, `audit_events`. UUIDs, chaves estrangeiras, migrações e índices. FTS5 para texto pesquisável, com política de ingestão. Usar WAL se adequado e backups consistentes.

## Proveniência
Cada fato/registro pode ter origem `USER_CONFIRMED`, `AGENT_REPORTED`, `FILE_VERIFIED`, `GIT_VERIFIED`, `SYSTEM_OBSERVED` ou `INFERRED`, timestamp e referências. Relato de agente de que 'testes passaram' não substitui resultado observado de execução.

## Estrutura sugerida
Dados globais em pasta de aplicação do SO:
```
config/
database/universal-ai.sqlite
projects/<uuid>/metadata.json
projects/<uuid>/memory/PROJECT.md
projects/<uuid>/memory/REQUIREMENTS.md
projects/<uuid>/memory/ARCHITECTURE.md
projects/<uuid>/memory/DECISIONS.md
projects/<uuid>/memory/TASKS.md
projects/<uuid>/memory/CURRENT_STATE.md
projects/<uuid>/sessions/
projects/<uuid>/handoffs/
projects/<uuid>/checkpoints/
backups/
logs/
```
Somente mediante consentimento criar `<repo>/.universal-ai/` com subconjunto público do contexto. Nunca gravar segredos ou transcrições privadas automaticamente no repo.

## Registro bruto e estruturado
Armazenar stream/eventos de terminal com limites (não inferir semântica integral); armazenar conversa estruturada exclusivamente quando API/saída suportar e parser for confiável. Marcar lacunas, inacessibilidade de conteúdo e erro de coleta.

## Memória incremental
Snapshot base + deltas de sessões; reconstrução determinística. Resumo opcional por IA deve indicar envio de dados ao provedor e possível consumo de cota; nunca eliminar original automaticamente. Usar limites por bytes/tokens apenas como estimativas, sem confundir com janela real do provedor.

## Busca e exportação
FTS5 com filtros por projeto, data, agente, sessão e tipo; resultados paginados e virtualizados. Exportar Markdown, JSON e TXT com revisão/redação de segredos. Estratégia de retenção configurável e relatórios de uso de disco. Excluir dados apenas por ação explícita separada da remoção do projeto da lista.

## Integridade e recuperação
Migrations transacionais, checksums quando úteis, journal, estados de sessão interrompida, backups e testes de queda durante escrita. Não declarar sessão antiga como ativa sem verificar PID e identidade do processo.
