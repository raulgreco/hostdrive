# ADR 006 — Atualização automática via GitHub Releases

## Estado

Aceito e implementado em 2026-10-20.

## Contexto

O usuário perguntou se era possível ter um sistema de atualização: instalar o app e depois receber aviso de versão nova. Isso implica três coisas que não existiam no projeto: um lugar público pra publicar versões, um jeito do app descobrir que existe versão nova, e um jeito de instalar com segurança (sem abrir brecha pra alguém disfarçar uma atualização maliciosa). Escolhido, com o usuário: GitHub Releases como hospedagem, repositório público.

## Decisão

### Repositório
Criado **https://github.com/raulgreco/hostdrive** (público). Primeiro commit em 2026-10-20 com todo o histórico de decisões já tomado (`docs/decisions/001` a `005`). Arquivos deliberadamente fora do repositório: `.xwin-cache/` (cache de download do Windows SDK, ~1 GB, puramente local), `fundo.png`/`fundo1.jpg` (assets soltos sem uso no código, aguardando decisão do usuário).

### Mecanismo — `tauri-plugin-updater`
Plugin oficial do Tauri 2 (`tauri-plugin-updater` no Rust, `@tauri-apps/plugin-updater` no frontend), registrado em `src-tauri/src/lib.rs`. Fluxo:
1. No início do app, `checkForUpdate()` (`src/services/updater.ts`) consulta o manifesto `latest.json`.
2. Se existir versão mais nova, `UpdateBanner` aparece com a versão e o changelog (texto da release).
3. **Nada é baixado nem instalado sem o usuário clicar "Atualizar agora"** — nunca automático, nunca silencioso.
4. Clicando, baixa, verifica a assinatura, instala e reinicia o app (`@tauri-apps/plugin-process`).

### Assinatura — por que é obrigatória
O updater do Tauri recusa instalar qualquer pacote que não esteja assinado com a chave privada correspondente à chave pública embutida em `tauri.conf.json` (`plugins.updater.pubkey`). Isso impede que alguém interceptando a rede, ou comprometendo o servidor de hospedagem, consiga empurrar uma atualização maliciosa — o app só aceita o que foi assinado por quem tem a chave privada.

- Par de chaves gerado com `tauri signer generate`, **sem senha** (decisão deliberada para permitir automação em CI sem precisar guardar uma segunda credencial — a proteção nesse caso é o GitHub Secret em si, que só é decifrado dentro do runner efêmero).
- **Chave privada**: vive só em `~/.hostdrive-keys/hostdrive-updater.key` (fora do repositório, nunca versionada) e como o secret `TAURI_SIGNING_PRIVATE_KEY` no repositório GitHub. **Se esse arquivo local for perdido e o secret do GitHub for apagado, não há como recuperar — seria preciso gerar um par novo e todo mundo que já instalou o app pararia de receber atualizações até reinstalar com a chave pública nova.** Fazer backup desse arquivo é responsabilidade do usuário a partir de agora.
- **Chave pública**: embutida em `tauri.conf.json`, versionada normalmente (não é segredo).

### Publicação — automática, mas com um portão manual
`.github/workflows/release.yml`: ao empurrar uma tag `vX.Y.Z`, builda em runners reais do GitHub (`macos-latest` e `windows-latest` — **não** usa a cross-compilação deste Mac, que só existe para desenvolvimento local; em CI o Windows builda nativo, sem `cargo-xwin`/`xwin`), assina com a chave do secret, e publica os instaladores + `latest.json` numa Release. **A Release é criada como rascunho (`releaseDraft: true`)** — fica invisível para o updater e para quem navega pelas Releases até alguém publicar manualmente no GitHub. Isso é intencional: nenhuma versão vaza pros usuários sem revisão humana antes.

## Consequências

- A partir de agora, toda versão nova precisa de uma tag `vX.Y.Z` empurrada ao repositório pra sair uma Release automaticamente (em rascunho, revisar e publicar manualmente).
- `docs/decisions/002-windows-cross-compilation.md` continua válido para desenvolvimento/teste local neste Mac, mas deixa de ser o caminho de distribuição real — releases oficiais agora compilam nativamente em cada SO via CI.
- Testado nesta rodada: plugins compilam e o app abre normalmente nos dois SOs (cross-compile Windows + build nativo macOS) com os plugins registrados; o bundle `.app.tar.gz` (formato exigido pelo updater no macOS) é gerado corretamente quando `createUpdaterArtifacts: true`.
- **Não testado**: o fluxo completo de atualização de ponta a ponta (publicar uma Release de verdade, abrir uma versão mais antiga do app e confirmar que o aviso aparece e a instalação funciona) — depende de pelo menos duas versões publicadas para comparar, o que só acontece depois do primeiro release real.
