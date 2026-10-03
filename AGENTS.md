# Fluxo deste projeto

- Não capturar, renderizar, fotografar ou inspecionar screenshots da interface, nem em testes, salvo novo pedido explícito do usuário.
- Para mudanças pequenas, inspecionar apenas os arquivos relevantes, editar, executar `rust/build.ps1 -Publish` uma vez e entregar. Não repetir builds, criar executáveis com novos nomes ou fazer benchmarks/revisões delegadas sem necessidade de risco.
- O build.ps1 executa a suíte de testes e compila offline; não instala dependências nem usa rede.
- Não limpar o cache de build entre mudanças: isso torna a próxima compilação mais lenta. Ele fica em `%LOCALAPPDATA%/CrOptixPatcher/cargo-target`, não no projeto.
- Se o exe estiver aberto, informar o bloqueio e manter o build pronto no cache; não encerrar o app à força.
- Preservar fonte histórico e arquivos do usuário. Não modificar extensões reais para testar o patcher; usar fixtures ou cópias.
- Verificar em proporção ao risco. Mudanças em transações, backups ou runtime exigem testes de regressão; mudanças cosméticas não exigem repetir testes de reprodução no navegador.
