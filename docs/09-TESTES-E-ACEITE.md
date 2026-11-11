# 09 — Estratégia de testes e aceite

## Pirâmide de testes
Rust unit para domínio, estados e schema; frontend Vitest para UI/stores; integração SQLite+PTY com executáveis fake; end-to-end quando suportado; smoke tests com CLIs reais; testes de build separados para macOS e Windows.

## Cenários mandatórios
- **T-001:** Sem CLI instalada: aplicativo abre, permite projetos/históricos e orientação de instalação.
- **T-002:** CLI detectada: processo inicia no cwd correto com ambiente seguro.
- **T-003:** PTY: menu interativo, cores ANSI, Unicode, Ctrl+C, resize e encerramento.
- **T-004:** Persistência: capturar sessão, reiniciar app e pesquisar registro salvo.
- **T-005:** Handoff: origem Codex para destino Claude sem apagar arquivos/sessão.
- **T-006:** Destino ausente: não destrói origem, oferece erro e recuperação.
- **T-007:** Handoff interrompido: conserva pacote e marca `PARTIAL/FAILED`.
- **T-008:** `.env` e chaves fictícias não entram no pacote automaticamente.
- **T-009:** Reinício após kill abrupto: banco consistente; sessões antigas não aparecem ativas sem validação.
- **T-010:** Dois agentes no mesmo projeto: alterações sobrepostas geram aviso verificável.
- **T-011:** Sem internet: projetos, histórico, busca, memória e export funcionam.
- **T-012:** Projeto sem Git funciona; projeto dirty não sofre stash/reset/commit automático.
- **T-013:** Versões divergentes de CLI: capacidades ausentes exibidas como não suportadas.
- **T-014:** Terminal contém segredo fictício: log/export com política de privacidade definida; limitações documentadas.
- **T-015:** Falha de disco/SQLite bloqueado: erro visível sem perda silenciosa além do último commit.
- **T-016:** Caminhos com Unicode/espaços no macOS e Windows.
- **T-017:** Migrações entre versões preservam projetos e histórico.
- **T-018:** Comando `///` funciona no modo orquestrador e não interfere em comandos da TUI.

## Critérios de conclusão
Código funcional, testes executados e documentados, casos de erro exercitados, sem mocks em produção, instalação testada em máquina compatível e privacidade validada. Quando a plataforma não está acessível, registrar `NOT_VERIFIED` com procedimento reproduzível. Não afirmar suporte de flags ou sessões sem testar/documentar por versão.

## Medição local
Metas experimentais: UI <3s e busca comum <500ms em hardware de referência definido; carga com histórico grande e uso de memória controlado. Publicar condições de benchmark, não inventar resultados.

## Evidência
Para cada etapa, registrar comando, SO, versão da CLI, status do teste, saída resumida e logs sanitizados. Corrigir regressões antes da conclusão.
