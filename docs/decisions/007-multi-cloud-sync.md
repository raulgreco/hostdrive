# 007 — Sync multi-provedor (Google Drive, OneDrive, Nextcloud) + arquivos de projeto

Data: 2026-11-10. Pedido do usuário: "conectar seja um OneDrive, Google Drive ou Nextcloud da pessoa… quando a pessoa entra em outro computador com o mesmo e-mail e sincroniza o Cloud, ele baixa o histórico, o contexto e os arquivos do projeto".

## Decisões

- **Um provedor ativo por vez**, escolhido em Configurações → Nuvem (`app_settings.cloud_provider`: `nextcloud` | `google` | `onedrive`). A coluna `nextcloud_sync_enabled` passa a significar "sincronização cloud ativa" para o provedor escolhido (nome antigo mantido para compatibilidade de schema).
- **Identidade = conta do provedor.** O e-mail da conta conectada (`app_settings.cloud_account`) é a identidade entre máquinas — sem cadastro HostDrive, sem servidor nosso (NFR-001/002). Quem troca de máquina e conecta a mesma conta passa a ver os mesmos projetos (via `nextcloud_sync_id` estável, agora provider-agnóstico).
- **OAuth com PKCE e redirect de loopback** (`src-tauri/src/oauth.rs`) para Google e OneDrive: o navegador abre no login do provedor, o código chega num listener local (`127.0.0.1:porta/callback`) e a troca por tokens acontece em Rust. **Client IDs são do usuário** (open-source pattern): gerados em console.cloud.google.com / portal Microsoft e digitados nas Configurações — não são segredo e vivem no SQLite; tokens e refresh_tokens vivem só no keyring do SO (`google-tokens`, `onedrive-tokens`, JSON com `expires_at`; renovação automática 60 s antes da expiração).
- **Escopos de mínimo privilégio**: Google `drive.appdata` (pasta oculta do app) + `openid email`; Microsoft `Files.ReadWrite.AppFolder` (pasta de apps do OneDrive) + `offline_access` + `User.Read`. O HostDrive não vê nada além do próprio espaço de app no Drive/OneDrive do usuário.
- **Criptografia inalterada** (AES-256-GCM + Argon2, decisão 005): tudo continua cifrado no cliente antes de sair da máquina, incluindo manifestos e arquivos de projeto.
- **Pacote de nível de app** (`app.uaisync`): preferências + agentes personalizados (tabela `custom_agents`), com união por id (local ganha em colisão; `INSERT OR IGNORE`) e preferência para o lado mais recente nas preferências.
- **Sync de arquivos do projeto** (`projects.files_sync_enabled`, opt-in por projeto, nunca por padrão):
  - Espelho two-way com manifesto: estado combinado da última sync gravado localmente em `<app_data>/file_manifests/<sync_id>.json` e na nuvem como `<sync_id>.manifest` (cifrado). Comparação por (size, mtime, sha256).
  - Arquivos remotos como objetos flat `<sync_id>.files.<percent-encoding-do-caminho>` — funciona nos três provedores (o Drive não aceita `/` em nome).
  - **Nunca apaga arquivo** em nenhuma ponta. Conflito (ambas mudaram) nunca é decidido silenciosamente: aplica o que não conflita, devolve a lista de conflitos para a UI escolher "versão local" ou "versão da nuvem" (`files_prefer`), e só publica o novo manifesto quando não há pendência.
  - Exclusões defensivas: diretórios de build/dependências (`.git`, `node_modules`, `target`, …, ver `EXCLUDED_DIRS` em `sync.rs`); arquivos de segredo por nome (`.env*`, `*.pem`, `*.key`, …, `is_excluded_file`); arquivos > 4 MiB são pulados com aviso (teto do upload simples do OneDrive/Google).
  - `.hostdrive/` (inclui `contexto.md`) entra no sync — é como o contexto universal viaja com os arquivos.
- **Conflito de dados** (memória/tarefas/histórico) mantém a regra da decisão 005: prompt ao usuário, `prefer` local/remote.

## Verificação

- 31 testes Rust passando (inclui novos: exclusões do walk, manifesto unido, round-trip do pacote de app, PKCE/percent-encoding do OAuth).
- Chamadas reais contra Google Drive / OneDrive / Nextcloud: **NOT_VERIFIED** — não há conta/credencial disponível neste ambiente. Fluxos de token, upload/download/listagem e sync OAuth devem ser exercitados com a conta do usuário antes de marcar IMPLEMENTED.
