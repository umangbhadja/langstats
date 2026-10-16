& {
    $ErrorActionPreference = 'Stop'

    if ($env:OS -ne 'Windows_NT') {
        throw 'langstats uninstaller: Windows is required'
    }
    if (-not $env:LOCALAPPDATA) {
        throw 'langstats uninstaller: LOCALAPPDATA is required'
    }

    $installDir = Join-Path $env:LOCALAPPDATA 'Programs\langstats'
    foreach ($file in 'langstats.exe', 'LICENSE', 'NOTICE') {
        $path = Join-Path $installDir $file
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            Remove-Item -LiteralPath $path -Force
        }
    }
    if ((Test-Path -LiteralPath $installDir -PathType Container) -and
        -not (Get-ChildItem -LiteralPath $installDir -Force | Select-Object -First 1)) {
        Remove-Item -LiteralPath $installDir
    }

    $normalized = $installDir.TrimEnd([char]'\')
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath) {
        $entries = @($userPath -split ';')
        $remaining = @($entries | Where-Object { $_.TrimEnd([char]'\') -ne $normalized })
        if ($remaining.Count -ne $entries.Count) {
            [Environment]::SetEnvironmentVariable('Path', ($remaining -join ';'), 'User')
        }
    }
    if ($env:Path) {
        $entries = @($env:Path -split ';')
        $remaining = @($entries | Where-Object { $_.TrimEnd([char]'\') -ne $normalized })
        if ($remaining.Count -ne $entries.Count) {
            $env:Path = $remaining -join ';'
        }
    }

    Write-Host "Uninstalled langstats from $installDir"
}
