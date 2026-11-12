# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Desenvolvedores em geral que baixam o app público no GitHub (github.com/raulgreco/hostdrive). Situação: usam mais de uma CLI de IA (Codex, Claude Code, OpenCode, Kimi Code e outras) em projetos reais, frequentemente em mais de uma máquina, e querem continuar de onde pararam sem depender de nenhum fornecedor específico. Confirmado pelo usuário: "qualquer dev que baixar" — first-run cuidadoso, exemplos e textos explicativos fazem parte do produto.

## Product Purpose

Aplicativo desktop local-first (macOS/Windows, Tauri) que organiza **qualquer** CLI de IA em projetos com terminais PTY reais, mantém histórico/memória/contexto universais independentes dos fornecedores e sincroniza tudo (histórico, contexto e arquivos de projeto) pela **conta cloud do próprio usuário** — Google Drive ou OneDrive — para que, ao abrir o app em outra máquina com a mesma conta e sincronizar, a pessoa recupere histórico, contexto e arquivos e continue de onde parou. Sucesso = trocar de máquina e estar exatamente onde estava, sem fricção e sem backend do HostDrive.

## Positioning

A camada de contexto é do usuário, não do fornecedor: histórico, memória e contexto vivem na máquina (local-first) e, opcionalmente, na nuvem que a pessoa já tem — nunca num serviço HostDrive. Nenhum produto vizinho (as próprias CLIs, com seus resumes próprios) consegue copiar com verdade a continuidade cruzada entre agentes e entre máquinas.

## Operating Context

- Desktop macOS Apple Silicon e Windows x64; janela Tauri webview; frontend web (React + TS + Vite).
- Uso offline-first: tudo funciona sem cloud; a sincronização é opcional e explícita.
- As CLIs são TUIs reais que devem ser preservadas intactas (stream, MCP, skills, auth); o app nunca as imita nem interfere no stream do terminal.
- Identidade = conta cloud do usuário. Mesmo e-mail/servidor em outra máquina = mesmo espaço de trabalho. (Inferido do pedido do usuário; confirmar na prática.)
- Workflows com Git locais; estado Git é somente leitura.
- Idioma primário PT-BR; instalador Windows já tem seletor PT-BR/EN.

## Capabilities and Constraints

Confirmado (funciona hoje): detecção das 4 CLIs, até 6 terminais PTY, projetos com busca/favoritos/arquivo, histórico SQLite FTS5 com exportação, memória e tarefas com proveniência, pacotes UAHP de handoff, sync Google Drive/OneDrive (memória/tarefas/UAHP/preferências, cifrado no cliente), atualização automática por GitHub Releases, temas, atalhos, acessibilidade básica.

Pedido nesta rodada (2026-11-10), tudo confirmado pelo usuário:
- **Agentes dinâmicos**: suporte a todas as CLIs, sem lista fixa; um agente aparece quando conectado/iniciado, os não conectados não aparecem.
- **Contexto universal entre conversas**: o contexto precisa necessariamente fluir de uma conversa para outra dentro de um projeto.
- **Sync multi-provedor**: Google Drive e OneDrive (usuário fornece Client IDs OAuth próprios; guia de registro embutido). Sync cobre histórico, contexto e arquivos de projeto.
- **UI/UX simples e objetiva, visual de alto nível.**

Restrições duras (do AGENTS.md, inegociáveis): tudo no computador do usuário; sem servidor próprio, conta HostDrive ou sync obrigatório; sem guardar senhas/tokens fora do chaveiro do SO; sem operações Git destrutivas; nunca marcar como testado o que não foi executado (em plataforma inacessível, registrar NOT_VERIFIED); execução perigosa só com confirmação.

## Brand Commitments

Nome do produto: **Eureka42 — Universal CLI** (definido pelo usuário em 2026-11-10; substitui o nome anterior "HostDrive"). Identidade visual oficial (cores, tipografia, regras de uso do ⌘): `~/Desktop/EUREKA42/Identidade Visual/brandbook/` (tokens.css + DESIGN.md v1.0) — cópias de trabalho em `src/assets/brand/`. Ativos: logo horizontal escuro/claro (SVG oficial), `eureka42-logo.png`, `splash-screen.png` (abertura do app). Marca escura por padrão; teal (#34D6BC luz / #0E8A77) é a única cor de marca; âmbar #F2B33D é pontual ("o momento eureka"); Inter (títulos 600) + Montserrat 800 (só números) + JetBrains Mono (código/terminal/atalhos). Proibições da marca: glow/sombra/degradê no logo, degradê roxo-azul de "IA", cérebros/robôs. Tom: direto, honesto, sem alegação além da evidência (a documentação histórica registra NOT_VERIFIED explicitamente). Idioma primário PT-BR.

Nomes internos legados preservados de propósito (invisíveis ao usuário, evitam perda de dados/credenciais): identificador do bundle `app.hostdrive.desktop`, banco `hostdrive.sqlite`, serviço de chaveiro `hostdrive-nextcloud`. Pasta raiz na nuvem: `Eureka42`. Repositório/atualizador: `github.com/raulgreco/hostdrive` (renomear o repo é decisão do usuário). Site oficial: `eureka42.org` (definido pelo usuário em 2026-11-10).

## Evidence on Hand

docs/00–09 + decisions/001–006 (requisitos, arquitetura, decisões); docs/implementation-status.md (estado verificado); release v0.1.0 publicada no GitHub com CI; builds locais em ../universal-ai-cli-builds; 19 testes Rust + 6 Vitest aprovados. Não existem: testes de execução real em Windows, chamadas WebDAV contra instância real, fluxo ponta-a-ponta de auto-update (todos registrados como NOT_VERIFIED nas docs).

## Product Principles

1. Local-first, sempre: a máquina é a fonte primária; a nuvem do usuário é espelho opcional, nunca a dona dos dados.
2. Universal, não preso a fornecedor: qualquer CLI, qualquer agente, qualquer máquina — o contexto pertence ao projeto e à pessoa.
3. Honestidade verificável: nada de "testado" sem execução real; limitações aparecem na documentação, não escondidas.
4. Simplicidade operacional: tarefas comuns em um clique/atalho; a UI some e deixa a pessoa trabalhar.
5. A identidade é a conta que a pessoa já tem: sem cadastro HostDrive, sem novo login, sem nova senha.

## Accessibility & Inclusion

Teclado primeiro (atalhos documentados), aria-label/role nos componentes interativos, temas claro/escuro, contraste legível em terminais longos. Sem norma formal estabelecida pelo usuário.
