# Fluxo do projeto Rust

- O aplicativo Windows e seus testes são escritos em Rust. A interface usa GPUI.
- Preserve os scripts em `assets/` como conteúdo incorporado para a extensão. Eles não são executados pelo aplicativo.
- Execute `cargo fmt --check`, `cargo test --locked` e `cargo build --release --locked`.
- No ambiente local, mantenha o cache de compilação em `%LOCALAPPDATA%/CrOptixPatcher/gpui-target`.
- Use fixtures para testar o patcher. Preserve alterações do usuário e backups legados.
- Não capture nem renderize a interface para testes, salvo pedido explícito.
- Se o exe estiver aberto, informe o bloqueio e deixe o build no cache. Não encerre o aplicativo à força.
