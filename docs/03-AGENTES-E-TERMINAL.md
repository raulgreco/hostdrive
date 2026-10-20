# 03 — Agentes e terminal

## Terminal
Usar xterm.js na WebView conectado a PTY real (`portable-pty` ou alternativa comprovada), com backpressure, fluxo incremental, resize, ANSI, truecolor, UTF-8, TUI, sinais, copiar/colar, seleção e scrollback com limites. Não usar apenas subprocessos com pipe stdout. Tratar cancelamento, fechamento e falhas sem bloquear a UI. Registrar eventos com timestamps sem confundir captura de caracteres com transcrição semântica.

## Detecção
Executáveis iniciais: `codex`, `claude`, `opencode`, `kimi`. Procurar no PATH do processo e em caminhos configuráveis; ambientes GUI no macOS podem herdar PATH diferente do shell interativo. Resolver paths canônicos, validar permissões e identificar versão com timeout. Suportar localização manual.

## Autenticação
O login ocorre pelos fluxos oficiais dentro da CLI. A existência de arquivos de configuração não prova autenticação. Não copiar credenciais entre fornecedores nem interceptar cookies/tokens. Estado: `UNKNOWN`, `REQUIRES_LOGIN`, `AUTHENTICATED` apenas com evidência oficial, `ERROR`.

## Adaptador
Contrato conceitual: `detect`, `version`, `capabilities`, `launch`, `resume?`, `discoverSessions?`, `prepareHandoff`, `getAuthStatus?`. A implementação de produção deve ser em Rust, com contratos tipados para frontend. Declarar compatibilidade individual de `interactivePty`, `resume`, `sessionDiscovery`, `structuredOutput`, `contextInjection`, `MCP`, `skills`, `plugins`, `usageDetection`.

## Retomada
Validar comandos na documentação e na versão **instalada**. Hipóteses a verificar: Codex `codex resume [id]` / `--last`; Claude `claude --continue` ou `--resume <id>`; OpenCode `opencode --continue` ou `--session <id>`; Kimi `kimi --continue` ou `--session <id>`. Não presumir que o mesmo identificador de sessão funciona entre agentes. Registrar comando testado e compatibilidade na matriz.

## Entrada de `///`
TUIs interativas podem ter buffers de edição próprios; não tentar interceptar caracteres arbitrários no PTY. Disponibilizar **sempre** paleta global e seletor visual. Comandos `///codex`, `///claude-code`, `///opencode`, `///kimi`, `///auto`, `///history`, `///context`, `///status`, `///rollback`, `///agents` são processados apenas quando a entrada está explicitamente sob controle do orquestrador. Suportar aliases editáveis e prevenir conflitos.

## Sessões paralelas
Abas por projeto, opcional splits; processos isolados por sessão. Alertar sobre escrita concorrente em caminhos coincidentes, sem prometer resolução automática. No Windows, não misturar WSL com processo nativo e não tratar paths `/mnt/c` como equivalentes sem mapeamento verificado.

## Experimentos
Construir mock CLI PTY com sequências ANSI, menus, readline, interrupção, prompts e falhas; testar ao menos um fluxo real por CLI disponível. Documentar fluxo de login, TUI, resize e resume por sistema operacional, diferenciando `TESTED`, `DOCUMENTED`, `UNKNOWN`, `UNSUPPORTED`.
