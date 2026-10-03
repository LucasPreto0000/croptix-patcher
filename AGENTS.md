# Fluxo do projeto C#

- Este é o projeto principal, em C# Windows Forms. `assets/runtime.js` contém as correções JavaScript incorporadas no exe; `runtime-v1.js` reconhece patches antigos.
- Não tirar fotos, capturar ou renderizar a interface, nem em testes, salvo novo pedido explícito do usuário.
- Compilar com `./build.ps1 -Publish`. Não instalar dependências, limpar caches, gerar clones do exe ou repetir builds sem necessidade.
- O cache fica em `%LOCALAPPDATA%/CrOptixPatcher/csharp-standalone`, fora do projeto.
- Se o exe estiver aberto, informar o bloqueio e deixar o build no cache; não encerrar o aplicativo à força.
- Nunca testar alterando extensões reais: usar fixtures. Mudanças no patch ou backups exigem testes de regressão.
- Preservar alterações do usuário. Não publicar caches, arquivos temporários ou informações pessoais no GitHub.
