---
name: Eureka42 — Universal CLI
description: Desktop local-first que reúne qualquer CLI de IA em projetos, com contexto que atravessa conversas e máquinas.
colors:
  teal-luz: "#34D6BC"
  teal-42: "#0E8A77"
  teal-fundo: "#0B7363"
  ambar: "#F2B33D"
  ambar-fundo: "#9A6406"
  violeta: "#8B7CF6"
  coral: "#FF7A6B"
  noite: "#0A0D13"
  superficie-1: "#121722"
  superficie-2: "#1A2130"
  linha: "#2A3242"
  texto: "#E9EDF2"
  texto-muted: "#98A2B3"
  papel: "#F4F6F7"
  tinta: "#13141F"
typography:
  display:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "17px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-0.025em"
  body:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "JetBrains Mono, ui-monospace, monospace"
    fontSize: "9px"
    fontWeight: 500
    letterSpacing: "0.12em"
  numerals:
    fontFamily: "Montserrat, Inter, sans-serif"
    fontWeight: 800
rounded:
  sm: "6px"
  md: "10px"
  lg: "16px"
  chip: "9px"
spacing:
  sm: "8px"
  md: "14px"
  lg: "24px"
components:
  button-primary:
    backgroundColor: "{colors.teal-luz}"
    textColor: "{colors.noite}"
    rounded: "{rounded.sm}"
    padding: "8px 14px"
  chip-tool:
    backgroundColor: "var(--chip)"
    textColor: "var(--chip-ink)"
    rounded: "{rounded.chip}"
    size: "56px 74px"
---

# Design System: Eureka42 — Universal CLI

## Overview

**Creative North Star: "O Baralho de Chips"**

Um aplicativo de trabalho escuro, denso e preciso, organizado como um baralho de fichas de cor indexadas: cada ferramenta do projeto (Terminal, Histórico, Memória, Git, Tarefas) é uma ficha rotulada na paleta funcional da marca, e a ficha ativa sai do baralho — sua cor carimba o filete da área de trabalho. A identidade Eureka42 (teal sobre noite, Inter/JetBrains Mono, cantos 6/10/16) é a autoridade; as fichas são a assinatura do app dentro dela. Recusa o visual default da categoria (IDE escura com neon genérico, degradê roxo-azul de "IA").

**Key Characteristics:** escuro por padrão (a marca); cor sempre faz trabalho de estado/seção, nunca decoração; legendas técnicas em JetBrains Mono com tracking largo; fichas cantos vivos de 9px; movimento só com `cubic-bezier(.23,1,.32,1)` e micro-feedback de pressão (scale .97 em 120ms); zero glow, zero degradê.

## Colors

Paleta de marca sobre noite: teal na intenção, âmbar no momento eureka, violeta como terceira série. As fichas usam exatamente os funcionais da marca — nenhuma cor fora do brandbook.

### Primary
- **Teal Luz** (`{colors.teal-luz}`): cor da marca sobre escuro (10,6:1). É a cor da ficha Terminal e do acento global (foco, seleção, links de ação) no tema escuro.
- **Teal 42** (`{colors.teal-42}`): logo e destaques sobre fundo claro (4,3:1). Ficha Git e acento no tema claro via Teal Profundo.

### Secondary
- **Âmbar Eureka** (`{colors.ambar}`): "o momento da ideia" — só sobre escuro e pontual. Ficha Memória, aviso de atualização e estados de espera no histórico.
- **Âmbar Profundo** (`{colors.ambar-fundo}`): versão de trabalho sobre claro; ficha Tarefas.

### Tertiary
- **Violeta Série-3** (`{colors.violeta}`): terceira série de dados da marca (5,8:1 em noite). Ficha Histórico.

### Neutral
- **Noite** (`{colors.noite}`): fundo do app e do terminal (xterm `#060910` no recuo do terminal).
- **Superfície 1 / 2** (`{colors.superficie-1}`, `{colors.superficie-2}`): painéis elevados e hover.
- **Linha** (`{colors.linha}`): divisores 1px; o app não usa sombra em repouso, usa linha.
- **Texto / Texto Muted** (`{colors.texto}`, `{colors.texto-muted}`): 7,6:1 de contraste sobre noite; secundário nunca é cinza puro — vem do token.
- **Papel / Tinta** (`{colors.papel}`, `{colors.tinta}`): papeis do tema claro (`[data-theme="light"]`).

**The Brand-Only Rule.** Teal é a única cor de marca; as cinco cores de ficha são exatamente os funcionais do brandbook (`tokens.css`). Nenhuma cor nova entra no sistema sem passar pela identidade.

**The State-Color Rule.** Coral (`{colors.coral}`) é reservado a erro/destruição (encerrar sessão, conflito, falha). Âmbar reservado a atenção (atualização, sessão interrompida). Verde/teal a sucesso e "ativo".

## Typography

**Display Font:** Inter 600 (com system-ui) — títulos de seção e cabeçalhos, tracking −2,5% a −3,5%.
**Body Font:** Inter 400/500 — UI e texto corrido.
**Label/Mono Font:** JetBrains Mono 400/500 — legendas de ficha, códigos (`T-01…K-05`), status de sessão, campos do orquestrador, metadados de sync.
**Numerals:** Montserrat ExtraBold 800 — **somente** números de destaque: o "42", marcações de perfil.

**Character:** uma voz grotesk neutra e de trabalho (Inter) com uma voz de máquina precisa (JetBrains Mono) para tudo que é medida/código. Serifa não existe no sistema — o manifesto do Sobre é Inter, não documento literário.

### Hierarchy
- **Display** (600, 17px, −0.02em): nome do projeto no cabeçalho; título do onboarding (24px) e do Sobre (26px com o 42 em Montserrat 800).
- **Title** (600, 18px): títulos dos painéis (Histórico, Memória…).
- **Body** (400, 13px/1.5): texto de UI; manifesto do Sobre em 13px/1.7, medida máxima 62ch.
- **Label** (JetBrains Mono 500, 9px, +0.12em, caixa alta): legendas de ficha, seções da sidebar, captions técnicas.

**The One Mono Rule.** Mono é para código, dados e medida — nunca para "fazer gracinha técnica" em título ou botão.

## Layout

Grid fixa de três colunas (sidebar 264px | trilho de fichas 96px | área de trabalho) sobre linhas de 52px (titlebar) / 62px (cabeçalho do projeto) / 1fr. Áreas de trabalho trocam por visibilidade (não desmontam). Densidade alta de ferramenta de trabalho: padding de painel 24px, linhas de lista em 42–46px. Em ≤900px a sidebar e o trilho de fichas se escondem e a área ocupa tudo — o app é desktop-first, o estreito é degradação, não alvo.

## Elevation & Depth

Sistema de linha, não de sombra: superfícies em repouso separam por `border: 1px solid var(--line)`. Sombra só em camadas flutuantes (diálogos, paletas, picker de agente, banner): `0 12px 32px -12px rgb(0 0 0 / .6)`, sempre com offset e blur suave — nunca halo colorido de offset zero. Não existe ghost card (borda larga + sombra ao mesmo tempo): uma superfície escolhe borda OU sombra.

**The Flat-At-Rest Rule.** Profundidade é resposta, não estado padrão: ela aparece só quando algo se sobrepõe ao trabalho.

## Shapes

Cantos de 6px (controles pequenos), 10px (botões, cartões de formulário), 16px (diálogos, onboarding); a ficha tem canto vivo próprio de 9px. Bordas 1px. Nada de pill exceto indicadores pequenos. O filete de cor da área de trabalho tem 3px de altura e transiciona de cor com a ficha ativa.

## Components

### Fichas (signature)
- **Shape:** cartão 56×74px, raio 9px, cor da ficha no fundo, código mono (`T-01`) no canto superior esquerdo com a cor `--chip-ink`, legenda caixa-alta mono 8px abaixo do cartão.
- **State:** ativa translada +10px para fora do trilho, anel de 2px na cor do texto do tema (`outline: 2px solid var(--text)`), sombra suave na cor da ficha; pressão scale .97.

### Buttons
- **Primary:** Teal Luz com texto Noite, raio 6px, padding 8×14; hover leve brilho; active scale .97.
- **Secondary:** Superfície 2 com borda Linha, texto do tema.
- **Destruição** (encerrar sessão): borda e texto Coral, fundo `coral/12`.

### Chips de agente / abas
- Abas de agente: fundo transparente, ativa com `accent-soft` e filete inferior 2px no acento; ponto verde de "sessão em andamento".

### Cards / Containers
- Painéis de formulário (settings-card, handoff-scope): Superfície 2, borda Linha, raio 10px, padding 12–14px. Onboarding: Superfície 1, raio 16px.

### Inputs / Fields
- Fundo afundado (`--bg-sunken`), borda Linha, raio 6px; foco muda a borda para o acento (sem glow). Orquestrador usa mono com o caret `///` em teal.

### Navigation
- **Trilho de fichas** (assinatura, acima). Sidebar: lista com hover em Superfície 2 e selecionado em `accent-soft` com título na cor do acento. Abas de terminal: raio 6px no topo, ativa fundo do terminal.

### Terminal
- xterm com fundo Noite (`#0A0D13`), texto `#E9EDF2`, cursor Teal Luz, seleção Superfície 2. Mensagens do app em teal (`38;2;52;214;188`) e âmbar para encerramento.

## Do's and Don'ts

### Do:
- **Do** usar as cores de ficha exatas (teal-luz, violeta, âmbar, teal-42, âmbar-fundo) como identidade de seção — e deixar a cor da ficha ativa marcar o filete da área de trabalho.
- **Do** respeitar `prefers-reduced-motion` (todas as transições somem) e dar feedback de pressão de 120ms nos controles.
- **Do** usar Montserrat 800 apenas para números de destaque e o "42".
- **Do** trocar o logo por tema: `logo-escuro.svg` no tema escuro, `logo-claro.svg` no claro.

### Don't:
- **Don't** colocar glow, sombra projetada ou degradê no logo da marca (regra do brandbook).
- **Don't** usar coral para qualquer coisa que não seja erro/destruição, nem âmbar como cor genérica de destaque.
- **Don't** criar cores fora do brandbook ou o degradê roxo-azul típico de produto de "IA".
- **Don't** usar serifa, ícones emoji como sistema de ícones, sombra de offset duro, ou borda colorida grossa em cartões e alertas.
- **Don't** usar easing elástico/bounce: a curva da marca é `cubic-bezier(.23,1,.32,1)`.
