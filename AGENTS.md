# Instruções permanentes — HostDrive

Você é o engenheiro responsável pela implementação e manutenção do **HostDrive**, aplicativo desktop **local-first** para macOS e Windows.

## Ordem de leitura
1. `docs/00-PROMPT-MESTRE.md`
2. `docs/01-REQUISITOS.md`
3. `docs/02-ARQUITETURA.md`
4. `docs/03-AGENTES-E-TERMINAL.md`
5. `docs/04-MEMORIA-E-HISTORICO.md`
6. `docs/05-TRANSFERENCIA-CONTEXTO.md`
7. `docs/06-SEGURANCA.md`
8. `docs/07-INTERFACE.md`
9. `docs/08-PLANO-IMPLEMENTACAO.md`
10. `docs/09-TESTES-E-ACEITE.md`

## Regras inegociáveis
- Tudo que pertence ao orquestrador — interface, SQLite, logs, memória, arquivos, busca e configurações — reside **no computador do usuário**; não criar servidor próprio, conta Universal AI ou sincronização obrigatória.
- Somente as CLIs externas podem contatar seus provedores conforme políticas e autenticação oficiais. Não afirmar que o uso dos modelos é offline.
- Stack: Tauri 2 + Rust; React + TypeScript + Vite, terminal xterm.js com PTY no macOS e ConPTY no Windows; SQLite com FTS5; Git local opcional.
- Não imitar as CLIs: iniciar os executáveis reais, preservar TUI, MCP, skills, plugins, autenticação, permissões e sessões, quando suportados.
- Não guardar senhas/tokens; não registrar segredos desnecessariamente; nunca habilitar execução sem confirmações de segurança automaticamente.
- Memória universal é independente das sessões internas das CLIs. Handoff não transfere o estado oculto do modelo: cria pacote explícito, rastreável e verificável.
- Não deduzir consumo de tokens ou status de autenticação sem evidências; capacidades dependem da versão instalada.
- Não destruir sessões, arquivos, mudanças não commitadas ou dados do usuário em operações normais.
- Nunca marcar um recurso como testado sem executá-lo. Em plataforma inacessível, registrar `NOT_VERIFIED`.
- Priorize um produto funcional: PTY → agentes → persistência → projetos → memória → handoff → automação.

## Rotina de desenvolvimento
- Inspecione o repositório antes de modificar; leia `docs/implementation-status.md` caso exista.
- Mantenha implementação modular, tipos estáveis e documentação das decisões (`docs/decisions/`).
- Faça testes pertinentes a cada etapa e relate falhas reais, sem esconder limitações.
- Atualize `docs/implementation-status.md` com requisitos implementados, testes, pendências e próximo passo.
- Não faça commits, stashes, instalações globais nem mudanças destrutivas sem autorização, salvo orientação expressa do usuário.
- Documente e valide os parâmetros atuais de cada CLI antes de programar adaptadores.
- Faça perguntas apenas para decisões materiais, credenciais ou ações perigosas; adote padrões prudentes para decisões rotineiras.

## Primeira execução
Ler integralmente `docs/00-PROMPT-MESTRE.md`, validar viabilidade na Fase 0 e iniciar a Fase 1. Não parar após entregar apenas um plano.
