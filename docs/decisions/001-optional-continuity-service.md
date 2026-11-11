# ADR 001 — Serviço opcional de continuidade

## Estado

Aceito. **Concretizado em 2026-10-18** via sincronização opcional por Nextcloud/WebDAV — ver `docs/decisions/005-nextcloud-sync.md` para o desenho e a implementação. A ideia de "servidor próprio de continuidade" descrita abaixo acabou não sendo necessária: o Nextcloud que o próprio usuário já controla cumpre o mesmo papel de "transporte e armazenamento de continuidade" sem o HostDrive precisar operar backend algum.

## Contexto

O produto deve permitir que um usuário comece um projeto em um computador e continue em outro. Os agentes e o processamento permanecem locais. O código-fonte deve continuar usando Git; o HostDrive sincronizaria apenas contexto explícito, memória, tarefas e preferências.

## Decisão

- A primeira versão continua totalmente funcional sem conta, backend ou internet.
- Uma conta futura será opcional e separada das autenticações das CLIs.
- A sincronização nunca enviará credenciais dos agentes.
- O código não será sincronizado pelo serviço; Git continua sendo o mecanismo recomendado.
- Conteúdo sincronizado deverá ser minimizado, criptografado no cliente e selecionável pelo usuário.
- O servidor atuará como transporte e armazenamento de continuidade, não como executor de IA.
- O site público poderá ser hospedado separadamente da API de sincronização.

## Consequências

O aplicativo deve manter contratos locais estáveis e não depender de disponibilidade da nuvem. Conta, dispositivos, planos e sincronização podem ser adicionados posteriormente sem alterar o núcleo PTY/SQLite. A interface pode antecipar essa direção, desde que identifique recursos indisponíveis e preserve claramente o modo local.
