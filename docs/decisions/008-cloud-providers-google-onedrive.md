# 008 — Provedores de nuvem: Google Drive e OneDrive (Nextcloud removido)

Data: 2026-11-11
Status: aceito (decisão do usuário)

## Contexto

O app nasceu com sincronização via Nextcloud/WebDAV (`005-nextcloud-sync.md`)
e ganhou Google Drive e OneDrive em 2026-11-10 (`007-multi-cloud-sync.md`).
No uso real, o Nextcloud se mostrou a fonte dominante de atrito:

- O servidor do usuário respondeu `HTTP 400` ao preparar a pasta remota e o
  erro aparecia em **todo lançamento** (sync automático de abertura), mesmo
  sem o usuário lembrar de ter configurado nada (config legado da sessão de
  testes de 2026-11-10 ficou persistida — comportamento correto de
  local-first, mas ruidoso).
- Cada leitura de credencial Nextcloud no chaveiro do macOS podia disparar
  o prompt de permissão do sistema ("deseja usar as informações
  confidenciais…") — foram ~15 senhas digitadas em uma sessão de testes.
- A senha de app do Nextcloud é um credencial extra que o usuário precisa
  gerar manualmente no servidor; Google e Microsoft usam o login que a
  pessoa já tem.

## Decisão

**Remover o Nextcloud do produto.** Nuvem = Google Drive ou OneDrive,
sempre via OAuth (PKCE, redirect loopback) com Client ID do próprio
usuário. Nenhum provedor fica pré-selecionado; sem provedor escolhido o app
é 100% local e não toca o chaveiro.

## Consequências

- Código: `NextcloudConfig`, o variant `Remote::Nextcloud`, os comandos
  `has/set/clear_nextcloud_credentials` e `test_nextcloud_connection` foram
  removidos; `set_cloud_passphrase` substitui o salvamento conjunto
  (senha de app + frase) — a frase de criptografia continua existindo
  (cifra local dos pacotes) e continua só no chaveiro.
- Banco: colunas renomeadas em novo esquema (`cloud_sync_enabled`,
  `cloud_sync_id`); bancos existentes migram copiando dos nomes legados
  (`nextcloud_sync_enabled`, `nextcloud_sync_id`) — cópia, nunca apaga.
  `VALID_CLOUD_PROVIDERS` aceita "" | "google" | "onedrive".
- Chaveiro: o serviço interno segue chamado `hostdrive-nextcloud` (nome
  legado invisível ao usuário) para não invalidar segredos já gravados.
- Documentos históricos (`005-nextcloud-sync.md`) ficam como registro.

## Alternativas rejeitadas

- Manter Nextcloud como terceira opção: rejeitado pelo usuário — o atrito
  (senha de app, prompts de chaveiro, erros de servidor) superava o valor
  de um terceiro provedor para o público-alvo.
