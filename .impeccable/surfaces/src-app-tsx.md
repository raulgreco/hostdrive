---
version: 1
slug: "src-app-tsx"
primary_target: "src/App.tsx"
related_targets: []
---

# Surface brief — HostDrive app shell (src/App.tsx)

Scope: the whole desktop app shell (titlebar, sidebar, utility navigation, work areas, dialogs). Visitor mode: Operate — the developer completes tasks (open project, start agent session, search history, edit memory, sync).

Audience: qualquer dev que baixar o app público; job = continuar trabalho com CLIs de IA em qualquer máquina. Action: iniciar/retomar sessão com o agente certo, com o contexto carregado. Proof: o próprio estado (agentes conectados, contexto publicado, sync com a conta). Constraints: Tauri webview, React 19, xterm.js escuro por decisão deliberada, PT-BR, temas claro/escuro/sistema, acessibilidade por teclado.

Chosen direction: **Baralho de Chips** — aprovada pelo usuário (2026-11-10, rodada `seed:fa31b05b207211b6`). Memorable moment: a ficha ativa sai do baralho e sua cor assume o chrome da área de trabalho.

## Direction contract

THESIS: o app como um baralho de fichas de cor indexadas — cada ferramenta do projeto (Terminal, Histórico, Memória, Git, Tarefas) é uma ficha rotulada; recusa o arranjo default da categoria (sidebar + abas top + um acento cinza).

OWN-WORLD: ground papel `#f4f2ee` (escuro: `#17181a`), tinta `#141414`, cinco cores de ficha que são papéis reais — Terminal `#1f4fd8`, Histórico `#ff5a1f`, Memória `#009b72`, Git `#6b2b3? ` não: Git `#6b2fb3`, Tarefas `#e8a800`. Fichas cantos vivos (radius 9px) com código mono `T-01…K-05` no canto e legenda caps embaixo; figuras tabulares; legendas em mono 9px com tracking largo. Componentes: cantos 9–12px, borda OU sombra (nunca os dois), sem halo colorido.

STORY: o dev entende em um olhar quais ferramentas o projeto tem (fichas), qual está ativa (ficha fora do baralho + cor no painel), quais agentes já conversaram (painel conectados) e se o trabalho está salvo na nuvem (conta no título). Faz a tarefa principal — nova sessão com contexto — em dois cliques.

FIRST VIEWPORT: colunas [sidebar 264px | trilho de fichas 96px | área de trabalho]. Sidebar: marca, Projetos (busca + lista), painel Agentes conectados. Trilho: cinco fichas 64×88 empilhadas com respiro, legenda mono abaixo de cada; a ativa translada −10px para fora do trilho com anel de tinta 2px. Área: cabeçalho do projeto (nome + nuvem/conta), painel ativo com filete superior 4px na cor da ficha, rodapé com orquestrador `///`. Ações de sessão à direita do trilho de abas de agente.

FORM: direção escolhida = candidate 7 do grounded list (Baralho de Chips); challengers pesados: rw-manual-acetate-tab-board (wins, não escolhido pelo usuário), crouwel-grid-specimen (competitive), demais declined com raises absorvidos. seed:fa31b05b207211b6.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.
