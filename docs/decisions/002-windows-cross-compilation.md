# ADR 002 — Cross-compilação Windows a partir de macOS

## Estado

Aceito. Toolchain de cross-compilação instalada e validada neste host em 2026-10-13.

## Contexto

A Fase 10 exige build Windows além de macOS. Este host é Apple Silicon sem acesso a uma máquina ou VM Windows. `rustup` já trazia o target `x86_64-pc-windows-msvc`, mas faltavam `clang-cl`, `lld-link`, `llvm-rc` e o Windows SDK/CRT (headers e `.lib` de import), impossíveis de obter via Xcode. A documentação anterior (`feasibility.md`, 2026-10-13) registrava o build Windows como `NOT_VERIFIED` por esse motivo.

## Decisão

- Instalar `llvm@20`/`llvm` (Homebrew) por `clang-cl`, `llvm-rc`, `llvm-lib`, `llvm-dlltool`.
- Instalar `lld` (Homebrew) por `lld-link`, ausente no `llvm@20` padrão.
- Instalar `makensis` (Homebrew) para gerar o instalador NSIS sem depender de Windows/WiX (WiX não tem build para macOS; MSI portanto permanece indisponível neste host).
- Usar `xwin` para baixar o Windows SDK e o CRT redistribuíveis da Microsoft (licença de cross-compilação) e montar um sysroot local (`~/.cache/cargo-xwin`, ~1,7 GB).
- Usar `cargo-xwin` como wrapper de build: ele aponta `CC`/`CXX`/`AR`/linker para `clang-cl`/`lld-link` e injeta os paths do sysroot automaticamente. Comando: `cargo xwin build --target x86_64-pc-windows-msvc --release`.
- Invocar o bundler do Tauri com `--runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis` a partir da raiz do repositório (`npm run tauri -- build ...`).
- Nenhuma dessas ferramentas é copiada para o repositório nem versionada; é setup de ambiente local, reproduzível por qualquer desenvolvedor macOS com Homebrew.

## Consequências

- O `.exe` principal, a `universal_ai_cli_lib.dll` e o instalador NSIS (`HostDrive_0.1.0_x64-setup.exe`) agora são gerados e verificados como PE32+/PE32 válidos para Windows x86_64, a partir deste Mac.
- Assinatura de código Windows continua indisponível (requer host Windows ou certificado + `signtool`); o bundler já reporta isso e pula a assinatura.
- MSI (WiX) não é produzido neste host; apenas NSIS. Caso MSI seja exigido, requer Windows real ou CI com runner Windows.
- Execução real do `.exe` (abrir janela, WebView2, PTY/ConPTY interativo com as quatro CLIs) continua `NOT_VERIFIED`: não há Windows nem WebView2 neste host, e Wine não resolve WebView2 (componente Edge). Build e link bem-sucedidos não substituem teste funcional em Windows real; isso é reportado como tal em `implementation-status.md`, não como "testado".
- Corrigido en passant: faltava `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` em `src-tauri/src/main.rs`, o que faria o `.exe` de release abrir com uma janela de console atrás da GUI em qualquer Windows real, inclusive numa build nativa futura. Validado comparando o campo PE `Subsystem` antes (3, CONSOLE) e depois (2, WINDOWS) da correção.
