$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifest = Join-Path $root 'desktop-rust\Cargo.toml'
$env:PATH = (Join-Path $root 'tools\nasm-3.02') + ';' + $env:PATH

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path $vswhere) {
    $vsInstall = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsInstall) {
        Import-Module (Join-Path $vsInstall 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
        Enter-VsDevShell -VsInstallPath $vsInstall -SkipAutomaticLocation -DevCmdArguments '-arch=x64'
    }
}

cargo fmt --manifest-path $manifest -- --check
if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed' }
cargo test --locked --offline --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
cargo clippy --locked --offline --manifest-path $manifest --all-targets --all-features -- -D warnings

if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed' }
Get-ChildItem (Join-Path $PSScriptRoot '..\desktop-rust\tests\*.cjs') | ForEach-Object {
    node $_.FullName
    if ($LASTEXITCODE -ne 0) { throw 'JavaScript tests failed' }
}


