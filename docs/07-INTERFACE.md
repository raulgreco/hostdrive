# 07 — Interface de usuário (UX/UI)

## Produto
Uma aplicação de produtividade para desenvolvedores, com organização visual semelhante a IDEs, porém **não** IDE completa. Interface nativa via Tauri WebView, com React/TypeScript e xterm.js, sem servidor web na execução.

## Layout conceitual
```
┌ UNIVERSAL AI CLI ───── Projeto: Sistema CPA ──────── ⚙ ┐
│ Projetos         │ [Codex][Claude][OpenCode][Kimi][+] │
│ • Sistema CPA    ├───────────────────────────────────┤
│ • Portal         │ Terminal PTY interativo           │
│ • Finance        │ ...                               │
│ Agentes          │                                   │
│ ● Codex          │                                   │
│ ● Claude         │                                   │
├──────────────────┴───────────────────────────────────┤
│ Terminal | Histórico | Memória | Git | Tarefas        │
└──────────────────────────────────────────────────────┘
```

## Telas
1. Boas-vindas/onboarding: descobrir CLIs sem obrigar instalação de todas.
2. Projetos: lista, recentes, favoritos, abrir pasta, arquivar.
3. Workspace: abas PTY, paleta, troca de agentes, painel de contexto.
4. Agentes: executável, versão, evidência de autenticação, configuração.
5. Histórico: linha do tempo, busca e filtros; eventos parciais identificados.
6. Memória: projeto, arquitetura, decisões, tarefas e origem das informações.
7. Handoffs: pacote, destino, advertências, estado e tentativas.
8. Configurações: tema, atalhos, diretórios, retenção, privacidade, preferências.
9. Diagnóstico local: processos, armazenamento, logs sanitizados e falhas.

## Interações essenciais
Paleta global `Cmd/Ctrl+Shift+P`, seleção gráfica de agente disponível enquanto TUI usa entrada própria. Comandos `///` somente no prompt do orquestrador. Não interceptar bytes arbitrários da TUI para detectar texto digitado. Permitir Ctrl+C para processo, copiar/colar e shortcuts sem colisão.

## Acessibilidade
Navegação por teclado, foco visível, contraste, labels acessíveis, temas claro/escuro, suportar redimensionamento e zoom, estado de carregamento significativo, mensagens acionáveis. Disponibilizar avisos claros ao transferir contexto para outro provedor.

## Estados importantes
`not installed`, `unknown auth`, `running`, `interrupted`, `history partial`, `handoff failed`, `context incomplete`, `offline`. Não mostrar status enganoso.

## Entrega por fases
UI ligada ao backend verdadeiro; mocks só em Storybook/testes. Começar simples, priorizando terminal e tarefas centrais.
