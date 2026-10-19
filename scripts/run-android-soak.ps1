$ErrorActionPreference='Stop'
$adb=Join-Path $env:LOCALAPPDATA 'Android/Sdk/platform-tools/adb.exe'
# Instrumentation restarts the target process. This private USB peer is intentionally not public/persistent.
$peer=Start-Job -ScriptBlock {
    for($attempt=0;$attempt -lt 60;$attempt++){
        try{Invoke-RestMethod -Uri 'http://127.0.0.1:3030/torrents/9c16fbcfc14cc96340eba40e23954d4ca22eae57/add_peers' -Method Post -Body '127.0.0.1:4242' -TimeoutSec 2 | Out-Null}catch{}
        Start-Sleep -Seconds 2
    }
}
try{& $adb shell am instrument -w -e class com.piratecinema.app.TorrentSoakTest -e soak true com.piratecinema.app.qa.test/androidx.test.runner.AndroidJUnitRunner}
finally{Stop-Job $peer -ErrorAction SilentlyContinue;Remove-Job $peer -Force -ErrorAction SilentlyContinue}
