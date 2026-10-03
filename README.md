# CrOptix Patcher

Patcher independente para Windows x64, escrito em C# (Windows Forms), para aplicar ajustes ao `katamari.js` da extensão [CrOptix](https://github.com/stratumadev/croptix). Não é um projeto oficial da Crunchyroll ou do CrOptix.

## Download e uso

Baixe `CrOptix Patcher.exe` em [Releases](https://github.com/LucasPreto0000/croptix-patcher/releases/latest).

1. Abra o aplicativo e selecione a pasta com `katamari.js` e `manifest.json` (geralmente `dist`).
2. Clique em **Aplicar patch** e consulte o console fixo.
3. Recarregue a extensão no navegador e a página da Crunchyroll.

A versão 1.2.0 torna C# o projeto principal, com console sempre visível, seletor de pasta do Explorer e exe de aproximadamente 91 KB. Usa o .NET Framework 4.x do Windows. O patch é o mesmo da versão Rust 1.1.0, e seus backups continuam compatíveis.

## Correções incluídas

- PiP com vídeo original e camada de legendas, controles nativos e restauração ao fechar.
- Botão de próximo episódio ao lado do volume.
- Avanço imediato em cada repetição de ←/→ e J/L, sem esperar soltar a tecla.
- Barra de controles oculta ao avançar, restaurada ao mover o mouse.
- Correção do foco da bolinha ao selecionar a timeline com o mouse e apertar espaço, preservando a navegação por Tab.

O PiP com legendas exige suporte a Document Picture-in-Picture; o fallback PiP nativo pode não incluir legendas.

## Arquivos e backups

Os arquivos JavaScript em `assets/` são as correções que o patcher insere na extensão, não outro aplicativo. O runtime anterior permite reconhecer patches antigos com segurança. Não são necessários Node.js ou Rust para usar o exe.

Valida os arquivos antes de modificar, salva cópias em `.croptix-rust-backups` (nome mantido por compatibilidade), não duplica patches reconhecidos e recusa patches adulterados. **Restaurar backup** só restaura um snapshot válido correspondente aos arquivos atuais, sem sobrescrever atualizações externas.

Não importa nem executa o arquivo original `pip1.js`. Não há atualização automática pela internet.

## Compilação e testes

Para compilar e testar no Windows com o compilador do .NET Framework e Node.js disponível apenas para testes:

```powershell
./build.ps1 -Publish
```

O build fica em `%LOCALAPPDATA%/CrOptixPatcher/csharp-standalone`. Com `-Publish`, o exe desta pasta é atualizado. A compilação não usa arquivos de outro projeto nem baixa dependências. Se o exe estiver aberto, feche-o antes de publicar o build.

Os testes verificam aplicação, reaplicação, restauração exata, migração, arquivos inválidos, backups corrompidos, rollback e foco da timeline. O teste opcional de comparação com um exe Rust real só roda quando `CROPTIX_RUST_EXE` é informado; o projeto não depende dele. Não são geradas capturas da interface.
