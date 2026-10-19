param([string]$SigningDirectory=(Join-Path $env:LOCALAPPDATA 'PirateCinema/android-signing'))
$ErrorActionPreference='Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    $credential=Import-Clixml -LiteralPath (Join-Path $SigningDirectory 'signing-credential.xml')
    $env:PIRATE_ANDROID_KEYSTORE=Join-Path $SigningDirectory 'pirate-cinema-release.p12'
    if(!(Test-Path -LiteralPath $env:PIRATE_ANDROID_KEYSTORE)){throw 'Release keystore is missing'}
    $env:PIRATE_ANDROID_STORE_PASSWORD=$credential.GetNetworkCredential().Password
    $env:PIRATE_ANDROID_KEY_PASSWORD=$env:PIRATE_ANDROID_STORE_PASSWORD
    $env:PIRATE_ANDROID_KEY_ALIAS=$credential.UserName
    pnpm run mobile:typecheck
    if($LASTEXITCODE){throw 'Mobile typecheck failed'}
    pnpm run android:sync
    if($LASTEXITCODE){throw 'Mobile build/sync failed'}
    node --test tests/mobile-torrserver.test.mjs tests/mobile-rutube.test.mjs
    if($LASTEXITCODE){throw 'Mobile tests failed'}
    Push-Location android
    try{ .\gradlew.bat :app:lintRelease :app:assembleRelease; if($LASTEXITCODE){throw 'Android release build failed'} }finally{Pop-Location}
    $sdk=if($env:ANDROID_HOME){$env:ANDROID_HOME}else{Join-Path $env:LOCALAPPDATA 'Android/Sdk'}
    $buildTools=Get-ChildItem (Join-Path $sdk 'build-tools') -Directory | Sort-Object {[version]$_.Name} -Descending | Select-Object -First 1
    $apk=Join-Path $pwd 'android/app/build/outputs/apk/release/app-release.apk'
    $signature=& (Join-Path $buildTools.FullName 'apksigner.bat') verify --verbose --print-certs $apk
    if($LASTEXITCODE){throw 'APK signature verification failed'}
    $signature | Write-Output
    $expectedCertificate=(Get-Content android/release-certificate.sha256 -Raw).Trim()
    if(!(($signature -join "`n").Contains("certificate SHA-256 digest: $expectedCertificate"))){throw 'Unexpected APK signing certificate'}
    $badging=& (Join-Path $buildTools.FullName 'aapt2.exe') dump badging $apk
    if($LASTEXITCODE -or !(($badging -join "`n") -match "package: name='com.piratecinema.app'")){throw 'Refusing to publish a QA or unexpected package'}
    & (Join-Path $buildTools.FullName 'zipalign.exe') -c -P 16 4 $apk
    if($LASTEXITCODE){throw 'APK 16 KiB alignment verification failed'}
    $ndk=if($env:ANDROID_NDK_HOME){$env:ANDROID_NDK_HOME}else{Join-Path $pwd 'local-builds/android-toolchain/ndk/android-ndk-r27c'}
    $reader=Join-Path $ndk 'toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-readelf.exe'
    $libraries=Get-ChildItem android/app/build/intermediates/merged_native_libs/release/mergeReleaseNativeLibs/out/lib/arm64-v8a -Filter '*.so'
    if(!$libraries){throw 'No native libraries found for verification'}
    foreach($library in $libraries){
        $loads=& $reader -l $library.FullName | Select-String ' LOAD '
        if($LASTEXITCODE -or !$loads){throw "Cannot inspect ELF $($library.Name)"}
        foreach($load in $loads){$alignment=($load.Line -split '\s+')[-1];if([Convert]::ToInt64($alignment.Substring(2),16) -lt 16384){throw "ELF is not 16 KiB aligned: $($library.Name)"}}
    }
    $version=(Select-String -Path android/version.properties -Pattern '^versionName=(.+)$').Matches.Groups[1].Value.Trim()
    $output=Join-Path $pwd "release-local/android/$version"
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    $name="Pirate-Cinema-Android-$version-arm64-v8a.apk"
    Copy-Item -LiteralPath $apk -Destination (Join-Path $output $name)
    $hash=(Get-FileHash -LiteralPath (Join-Path $output $name) -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $name" | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS') -Encoding ascii
    Write-Output "Verified release APK: $(Join-Path $output $name)"
} finally {
    Remove-Item Env:PIRATE_ANDROID_STORE_PASSWORD,Env:PIRATE_ANDROID_KEY_PASSWORD -ErrorAction SilentlyContinue
    Pop-Location
}
