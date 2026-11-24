# CrOptix Patcher

Aplicativo Windows x64 em Rust com interface [GPUI](https://gpui.rs/) para aplicar ajustes ao Katamari da extensão CrOptix.

## Uso

1. Abra `CrOptix Patcher.exe` e escolha a pasta que contém `katamari.js` e `manifest.json`.
2. Clique em **Aplicar patch** e confira o relatório.
3. Recarregue a extensão e atualize a página da Crunchyroll.

O patch inclui PiP com legendas em navegadores compatíveis, botão de próximo episódio, avanço por teclado sem espera, ajustes de foco e controles, e desativação do [CC] selecionado automaticamente em áudio dublado. A seleção manual de [CC] continua disponível.

O aplicativo embute o conteúdo de `assets/runtime.js` no arquivo da extensão. O navegador executa esse conteúdo ao carregar o player. A seleção de qualidade do Katamari é preservada.

A aplicação não cria backups. Em caso de falha durante a gravação, o motor tenta restaurar os arquivos que já substituiu usando os dados que manteve em memória. **Restaurar backup antigo** lê snapshots preexistentes em `.croptix-rust-backups` e restaura somente os que correspondem aos arquivos atuais.

## Compilação

Use Rust estável com alvo Windows MSVC, Visual Studio Build Tools com suporte a C++ e Windows SDK. O projeto usa o compilador e o vinculador desse ambiente para gerar o binário Windows.

```text
cargo fmt --check
cargo test --locked
cargo build --release --locked
```

O executável fica em `target/release/croptix-patcher.exe` ou no diretório de compilação definido em `CARGO_TARGET_DIR`.
