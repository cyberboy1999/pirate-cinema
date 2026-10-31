$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifest = Join-Path $root 'desktop-rust\Cargo.toml'
$stage = Join-Path $root 'stage'
$release = Join-Path $root 'release'
$version = (Select-String $manifest -Pattern '^version = "(.+)"$').Matches.Groups[1].Value
$env:PATH = (Join-Path $root 'tools\nasm-3.02') + ';' + $env:PATH

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (!(Test-Path $vswhere)) { throw 'Visual Studio Build Tools не найдены' }
$vsInstall = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (!$vsInstall) { throw 'Visual Studio C++ workload не найден' }
Import-Module (Join-Path $vsInstall 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $vsInstall -SkipAutomaticLocation -DevCmdArguments '-arch=x64'

cargo build --locked --offline --release --manifest-path $manifest --bin pirate-cinema
if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed' }

New-Item -ItemType Directory -Path $stage,$release,(Join-Path $stage 'ffmpeg') -Force | Out-Null
Copy-Item (Join-Path $root 'desktop-rust\target\release\pirate-cinema.exe') $stage -Force
Copy-Item (Join-Path $root 'runtime\windows\ffmpeg\ffmpeg.exe') (Join-Path $stage 'ffmpeg') -Force
Copy-Item (Join-Path $root 'runtime\windows\vc_redist.x64.exe') $stage -Force
Copy-Item (Join-Path $root 'LICENSE'),(Join-Path $root 'THIRD_PARTY_NOTICES.md'),(Join-Path $root 'README.md') $stage -Force

$makensis = Join-Path $root 'tools\nsis-3.13\makensis.exe'
$output = Join-Path $release "Pirate-Cinema-Setup-$version-win-x64.exe"
& $makensis "/DAPP_SOURCE=$stage" "/DAPP_OUTPUT=$output" "/DAPP_ICON=$(Join-Path $root 'build\icon.ico')" "/DAPP_VERSION=$version" (Join-Path $root 'desktop-rust\installer.nsi')
if ($LASTEXITCODE -ne 0) { throw "NSIS завершился с кодом $LASTEXITCODE" }

Get-FileHash $output -Algorithm SHA256
