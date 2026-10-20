# Viabilidade — levantamento inicial

Data: 2026-10-13. Ambiente observado: macOS 27.0.1, Apple Silicon.

## Ferramentas locais

| Ferramenta | Estado | Evidência |
|---|---|---|
| Node.js | TESTED | `node --version` → `v26.8.1` |
| npm | TESTED | `npm --version` → `11.19.0` |
| Xcode CLI | DETECTED | `/usr/bin/xcodebuild` |
| Rust/Cargo | TESTED | Rust `1.99.0`; `cargo check` e testes concluídos |
| Codex | DETECTED | `0.154.0`; ajuda local expõe `resume` |
| Claude Code | DETECTED | `2.1.273`; ajuda local expõe `--continue` e `--resume` |
| OpenCode | DETECTED | `1.18.31`; ajuda local expõe `--continue` e `--session` |
| Kimi Code | DETECTED | `0.43.1`; ajuda local expõe `--continue` e `--session` |

## Conclusão

O frontend React e o núcleo Tauri compilam neste Mac. `.app`, ZIP e DMG arm64 foram gerados; a build release abriu e executou as quatro TUIs.

**Atualização 2026-10-13 (tarde):** a cross-compilação Windows foi viabilizada neste mesmo host. `clang-cl`, `llvm-rc`, `llvm-lib`/`llvm-dlltool` vieram do Homebrew (`llvm`), `lld-link` do Homebrew (`lld`), o Windows SDK/CRT via `xwin` e a orquestração via `cargo-xwin` (ver `docs/decisions/002-windows-cross-compilation.md`). `cargo xwin build --target x86_64-pc-windows-msvc --release` e `npm run tauri -- build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis` produziram `universal-ai-cli.exe`, `universal_ai_cli_lib.dll` (ambos PE32+ válidos, confirmados via `file`) e o instalador `HostDrive_0.1.0_x64-setup.exe` (NSIS, 2,85 MiB, via `makensis` do Homebrew). MSI (WiX) continua indisponível neste host. Execução real em Windows (abrir janela, WebView2, PTY/ConPTY com as quatro CLIs) permanece `NOT_VERIFIED`: não há máquina ou VM Windows disponível, e Wine não resolve o WebView2 (dependência do Edge). Nenhuma autenticação foi inferida a partir da presença dos executáveis.

Fontes oficiais consultadas: documentação de pré-requisitos do Tauri 2 e referência CLI do Claude Code. As capacidades das quatro ferramentas também foram confrontadas com a ajuda das versões instaladas.
