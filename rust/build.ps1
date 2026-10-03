param([switch]$Publish)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    if (-not $env:CARGO_TARGET_DIR) {
        $env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA 'CrOptixPatcher\cargo-target'
    }
    node --test ../tests/runtime.test.cjs
    if ($LASTEXITCODE -ne 0) { throw 'Testes JavaScript falharam; executável não publicado.' }
    cargo test --offline --quiet
    if ($LASTEXITCODE -ne 0) { throw 'Testes falharam; executável não publicado.' }
    cargo build --release --offline --quiet
    if ($LASTEXITCODE -ne 0) { throw 'Compilação falhou; executável não publicado.' }
    $metadata = cargo metadata --offline --no-deps --format-version 1 | ConvertFrom-Json
    $exe = Join-Path $metadata.target_directory 'release\croptix-patcher.exe'
    if ($Publish) {
        $destination = Join-Path (Split-Path $PSScriptRoot -Parent) 'CrOptix Patcher.exe'
        try { Copy-Item -LiteralPath $exe -Destination $destination -Force }
        catch { throw "Feche o app para atualizar. Build pronto em: $exe" }
    }
    $timer.Stop()
    Write-Output ('Verificado em {0:N2}s. Executável: {1}' -f $timer.Elapsed.TotalSeconds, $exe)
} finally { Pop-Location }
