# ADR 005 — Sincronização de continuidade via Nextcloud

## Estado

Aceito e implementado em 2026-10-18, substituindo a entrada `CONTINUIDADE_CLOUD: PLANNED` de `docs/decisions/001-optional-continuity-service.md` por uma implementação concreta que usa o Nextcloud do próprio usuário em vez de um serviço próprio.

## Contexto

O usuário pediu, em testes reais: "ele conectar num nextcloud e levar os dados para lá e depois quando abrir em outra máquina, ele trazer de volta". O ADR 001 já previa exatamente esse cenário ("conta futura será opcional", "sincronização nunca enviará credenciais dos agentes", "servidor atuará como transporte... não executor de IA") — a decisão aqui é concretizar isso usando WebDAV contra o Nextcloud que o usuário já tem, em vez de construir um backend próprio (o que violaria NFR-001).

## Decisão

### O que sincroniza, e o que nunca sincroniza
- **Sincroniza por padrão** (quando a sincronização está ligada em Configurações): memória do projeto (`project_memory`), tarefas (`project_tasks`), pacotes de transferência UAHP (`handoffs`, arquivos incluídos) e preferências (tema, retenção, captura de eventos).
- **Nunca sincroniza**: credenciais de nenhuma CLI (Codex/Claude/OpenCode/Kimi) — o app nunca as leu nem as guarda, então não há o que enviar. Código-fonte do projeto continua por conta do Git do usuário.
- **Histórico de terminal é opt-in por projeto**, desligado por padrão (`projects.sync_terminal_history`) — pode conter segredos colados durante a sessão; a decisão de incluir é do usuário, projeto a projeto, nunca uma configuração global que acidentalmente abrange um projeto sensível.

### Autenticação — duas credenciais diferentes, propositalmente separadas
1. **Senha de app do Nextcloud** (App Password, gerada em Configurações → Segurança no próprio Nextcloud): autentica as chamadas WebDAV. Nunca a senha principal da conta.
2. **Frase de criptografia**: cifra os dados antes de saírem da máquina. É independente da senha do Nextcloud — quem administra o servidor Nextcloud nunca vê o conteúdo em claro, mesmo tendo acesso total aos arquivos armazenados.

Ambas ficam **só no chaveiro do sistema operacional** (`keyring` 4, backend nativo: Keychain no macOS, Credential Manager no Windows) — nunca no SQLite, nunca devolvidas ao frontend depois de salvas (os comandos Tauri só confirmam presença/ausência, nunca o valor).

### Protocolo e formato
- WebDAV puro contra `https://<servidor>/remote.php/dav/files/<usuário>/HostDrive/<id>.hostdrive-sync` — sem SDK proprietário do Nextcloud, qualquer servidor WebDAV compatível funciona na prática.
- Cada projeto sincronizado tem um `nextcloud_sync_id` (UUID) estável, gerado na primeira sincronização — independente do caminho local, que muda de máquina para máquina. Ao abrir o app numa máquina nova, o usuário reabre o projeto pela mesma pasta (ou cria o projeto apontando pra ela) e a sincronização localiza o blob remoto pelo `nextcloud_sync_id` já gravado no banco local, se existir, ou cria um novo na primeira vez.
- Payload é um único JSON cifrado com AES-256-GCM; a chave deriva da frase de criptografia via Argon2 (salt aleatório por envio, nunca reaproveitado). Formato do blob: `salt(16B) || nonce(12B) || ciphertext`.

### Gatilho: automático, não manual
Por decisão do usuário (ver conversa), a sincronização dispara **automaticamente**:
- **Ao abrir/trocar para um projeto**: traz (`pull`) o que há de mais novo no Nextcloud.
- **Ao trocar para outro projeto, ou fechar a janela**: envia (`push`) o que mudou no projeto que estava aberto. O fechamento da janela é interceptado (`onCloseRequested`) para dar até 8 segundos para esse envio antes de encerrar de fato — não é garantido se o processo for morto à força pelo SO.

### Conflito — nunca decidido silenciosamente (NFR-003)
Cada lado (local e remoto) guarda um `updated_at` derivado do conteúdo real (máximo entre as datas de atualização de memória/tarefas/handoffs/histórico incluído). A cada sincronização:
- Se o remoto não mudou desde a última sincronização deste projeto → envia o local.
- Se só o remoto mudou → traz o remoto.
- Se **os dois** mudaram desde a última sincronização → não decide: devolve `status: "conflict"` com as duas datas, e a interface mostra duas opções explícitas ("Usar versão local" / "Usar versão do Nextcloud"). Nunca há merge automático de campo a campo nesta primeira versão.

## Consequências

- Nenhum servidor próprio foi criado; o "backend" é inteiramente o Nextcloud que o usuário já controla (NFR-001 preservado).
- Dependências novas no `src-tauri/Cargo.toml`: `ureq` (WebDAV, TLS via `rustls`+`ring`), `keyring` (chaveiro nativo), `aes-gcm`+`argon2` (criptografia cliente), `base64`. Todas puramente Rust (sem OpenSSL/CMake), validadas nesta mesma sessão tanto na compilação nativa macOS quanto na cross-compilação Windows (`cargo xwin build`) — compilaram sem ajuste algum nas duas.
- Testado nesta rodada: round-trip de cifra/decifra com AES-GCM+Argon2 (`sync::tests::encrypt_then_decrypt_round_trips_with_the_right_passphrase`), rejeição de senha errada e de blob corrompido, validação das credenciais, e a codificação percentual usada nas URLs WebDAV.
- **Não testado**: chamadas WebDAV reais contra um servidor Nextcloud — este ambiente não tem acesso a uma instância para testar contra. A lógica de decisão de conflito, upload/download e criação da pasta remota (`MKCOL`) está implementada e compila, mas o caminho de rede em si é `NOT_VERIFIED` até o usuário testar com suas próprias credenciais.
- Migração de schema (`projects.nextcloud_sync_id`/`last_synced_at`/`sync_terminal_history`, `app_settings.nextcloud_url`/`nextcloud_username`/`nextcloud_sync_enabled`) usa o mesmo padrão `ensure_column` já estabelecido para `resumed`/`theme` — retrocompatível com bancos existentes, confirmado relançando o `.app` contra o banco SQLite já criado em sessões anteriores sem erro.
