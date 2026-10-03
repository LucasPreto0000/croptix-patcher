# CrOptix Patcher

Patcher independente para Windows x64, escrito em Rust, para aplicar ajustes ao `katamari.js` da extensão [CrOptix](https://github.com/stratumadev/croptix). Não é um projeto oficial da Crunchyroll ou do CrOptix.

## Download e uso

Baixe `CrOptix Patcher.exe` na seção [Releases](https://github.com/LucasPreto0000/croptix-patcher/releases).

1. Feche a extensão/página durante a aplicação e abra o patcher.
2. Selecione a pasta que contém `katamari.js` e `manifest.json` (normalmente `dist`).
3. Clique em **Aplicar patch** e consulte o console fixo.
4. Recarregue a extensão no navegador e a página da Crunchyroll.

O executável não precisa de Node.js ou .NET e não importa nem executa `pip1.js`. A lógica dos fixes está incorporada no próprio aplicativo.

## Ajustes incorporados

- Picture-in-Picture com vídeo original e camada de legendas, controles nativos, restauração ao fechar e fallback para PiP nativo.
- Botão de próximo episódio ao lado do volume, com tamanho e alinhamento ajustados.
- Avanço imediato em cada repetição de ←/→ e J/L, sem pausar ou esperar soltar a tecla.
- Barra de controles oculta ao avançar, restaurada ao mover o mouse.
- Correção da bolinha da timeline ao selecionar com o mouse e apertar espaço: preserva o foco visual do mouse, sem remover a indicação de foco para navegação por Tab.

A versão 1.1.0 incorpora a atualização de `pip1.js` de 03/10/2026. Atualiza patches Rust anteriores reconhecidos sem duplicá-los. Patches editados ou desconhecidos são recusados. O navegador precisa oferecer suporte às APIs de PiP; o fallback nativo pode não incluir as legendas.

## Segurança e backups

Valida os arquivos antes de modificar, faz backup em `.croptix-rust-backups`, registra o que mudou e detecta reaplicações. **Restaurar backup** escolhe um snapshot válido correspondente aos arquivos atuais e recusa sobrescrever arquivos atualizados por outro programa.

Não há atualização automática pela internet: para novos ajustes, baixe uma versão nova e aplique novamente. Mudanças internas futuras do player podem exigir adaptação.

## Compilação e testes

Requer Windows x64, Rust MSVC, ferramentas C++/Windows SDK e Node.js apenas para os testes JavaScript. Node.js não é incorporado ao `.exe`.

```powershell
cd rust
cargo fetch --locked  # primeira vez, baixa as dependências
./build.ps1 -Publish
```

O script executa os testes Rust e JavaScript e gera o exe na raiz. Sem `-Publish`, apenas verifica/compila. Se o aplicativo estiver aberto, feche-o e repita a publicação.

O cache é reutilizado em `%LOCALAPPDATA%/CrOptixPatcher/cargo-target`, ou no caminho definido por `CARGO_TARGET_DIR`. A pasta de cache, executáveis, fotos e fontes históricos C# não fazem parte do repositório. Não são geradas capturas da interface.

`rust/runtime.js` é o runtime atual. `rust/runtime-v1.js` é mantido somente para reconhecer com segurança o patch anterior durante a migração. `apply-patch.js` é uma alternativa Node independente para aplicação em uma instalação sem patch Rust; o executável Rust é a opção principal.
