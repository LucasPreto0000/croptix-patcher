param([switch]$Publish)
$ErrorActionPreference = 'Stop'
$timer = [Diagnostics.Stopwatch]::StartNew()
$projectDir = $PSScriptRoot
$buildDir = Join-Path $env:LOCALAPPDATA 'CrOptixPatcher/csharp-standalone'
New-Item -ItemType Directory -Path $buildDir -Force | Out-Null
$exePath = Join-Path $buildDir 'CrOptix Patcher.exe'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if (!(Test-Path -LiteralPath $compiler)) { throw 'Compilador C# do .NET Framework não encontrado.' }
$iconPath = Join-Path $projectDir 'assets\croptix.ico'
$manifestPath = Join-Path $PSScriptRoot 'app.manifest'
$runtimePath = Join-Path $projectDir 'assets\runtime.js'
$legacyPath = Join-Path $projectDir 'assets\runtime-v1.js'
$sources = @('App.cs','PatchEngine.cs','ExplorerFolderPicker.cs') | ForEach-Object { Join-Path $PSScriptRoot $_ }
& $compiler /nologo /target:winexe /platform:x64 /optimize+ /utf8output "/out:$exePath" "/win32icon:$iconPath" "/win32manifest:$manifestPath" /reference:System.Windows.Forms.dll /reference:System.Drawing.dll /reference:System.Web.Extensions.dll "/resource:$iconPath,croptix.ico" "/resource:$runtimePath,runtime.js" "/resource:$legacyPath,runtime-v1.js" @sources
if ($LASTEXITCODE -ne 0) { throw 'A compilação C# falhou.' }
$previousExe = $env:CROPTIX_CSHARP_EXE
try {
    $env:CROPTIX_CSHARP_EXE = $exePath
    & node --test "$projectDir/tests/csharp.test.cjs" "$projectDir/tests/runtime.test.cjs"
    if ($LASTEXITCODE -ne 0) { throw 'Os testes falharam; executável não publicado.' }
} finally { $env:CROPTIX_CSHARP_EXE = $previousExe }
if ($Publish) {
    try { Copy-Item -LiteralPath $exePath -Destination (Join-Path $projectDir 'CrOptix Patcher.exe') -Force }
    catch { throw "Feche o aplicativo para substituir o exe. Build pronto em: $exePath" }
}
Write-Output "Build C# concluído em $([Math]::Round($timer.Elapsed.TotalSeconds,2))s: $exePath ($((Get-Item -LiteralPath $exePath).Length) bytes)"
