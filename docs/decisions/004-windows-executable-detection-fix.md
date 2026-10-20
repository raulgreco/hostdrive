# ADR 004 — Detecção de executáveis no Windows e remoção dos botões de macOS

## Estado

Aceito e implementado em 2026-10-18, a partir de teste real do usuário em máquina Windows.

## Contexto

O usuário reportou dois problemas ao testar o instalador Windows gerado em `docs/decisions/002-windows-cross-compilation.md`:

1. O cabeçalho do app mostrava três pontos vermelho/amarelo/verde no estilo macOS, fora de lugar no Windows.
2. "Precisa deixar logar em cada ferramenta, no Windows não está funcionando."

## Diagnóstico

**Botões de macOS:** `App.tsx` desenhava `.traffic-lights` manualmente dentro do `<header>`, independente de plataforma. A janela já usa decorações nativas do SO (`tauri.conf.json` nunca define `decorations: false`), então esses pontos eram puramente decorativos e duplicavam os controles reais da janela — nunca deveriam ter aparecido em nenhuma plataforma, mas só chamavam atenção fora do macOS.

**Login não funciona no Windows:** a causa raiz não é o fluxo de login em si — é anterior a ele. `agents::find_executable` procurava apenas pelo nome puro do executável (`dir.join("codex").is_file()`). No Windows, `CreateProcess`/`std::process::Command` não resolvem extensão como o `cmd.exe` faz: os binários reais instalados por `npm install -g` ou `cargo install` chamam-se `codex.exe`, `claude.cmd`, `opencode.cmd`, `kimi.cmd` (shims), nunca o nome sem extensão. Resultado: `find_executable` retornava `None` para os quatro agentes no Windows, `detect_agents` reportava todos como não instalados, e `launch_terminal` falhava com "agente não instalado ou não permitido" antes de qualquer processo abrir — não havia PTY, não havia terminal, portanto não havia onde fazer login. `executable_search_dirs()` também só continha caminhos Unix (`/opt/homebrew/bin`, `~/.cargo/bin`, ...), nunca batendo com instalações Windows fora do `PATH`.

Esse diagnóstico explica o sintoma relatado sem precisar reproduzi-lo numa máquina Windows real (que não está disponível neste host) — mas por isso mesmo continua `NOT_VERIFIED` até o usuário confirmar no teste seguinte.

## Decisão

- `candidate_file_names` passa a tentar o nome puro e, no Windows, cada extensão de `PATHEXT` (ou o padrão `.COM;.EXE;.BAT;.CMD` se a variável não existir), replicando a mesma resolução que o `cmd.exe` faz.
- `executable_search_dirs` ganha diretórios equivalentes no Windows: `%USERPROFILE%\.cargo\bin`, `%USERPROFILE%\.local\bin`, `%USERPROFILE%\.kimi-code\bin`, `%APPDATA%\npm`, `%LOCALAPPDATA%\Programs` — além do `PATH`, que continua sendo a fonte principal.
- `.traffic-lights` removido de `App.tsx`/`styles.css`; o cabeçalho ganhou `data-tauri-drag-region` para continuar arrastável mesmo sem os pontos falsos.

## Consequências

- Isso deveria destravar não só o login, mas qualquer uso das quatro CLIs no Windows — a falha anterior era "a ferramenta nunca abre", não algo específico do fluxo de autenticação.
- Se o usuário testar de novo e o problema persistir (ou for diferente — ex.: o terminal abre mas o login em si trava), é sinal de uma segunda causa (ConPTY, variáveis de ambiente do processo filho, ou o próprio fluxo OAuth/browser de alguma CLI) que precisa de diagnóstico específico com a mensagem de erro real.
- Testado nesta rodada: compilação cruzada limpa (`cargo xwin build`), teste unitário `#[cfg(windows)]` cobrindo a lista de candidatos (não executa neste host, só type-checa), e teste `#[cfg(not(windows))]` confirmando que o comportamento em Unix não muda. Execução real num Windows segue pendente de confirmação do usuário.
