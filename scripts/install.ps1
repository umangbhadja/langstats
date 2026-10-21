& {
    $ErrorActionPreference = 'Stop'
    $repo = 'https://github.com/umangbhadja/langstats'
    $totalTimer = [Diagnostics.Stopwatch]::StartNew()
    function Write-Timing($label, $timer) {
        Write-Host ('{0}: {1:0.0}s' -f $label, $timer.Elapsed.TotalSeconds)
    }

    if ($env:OS -ne 'Windows_NT') {
        throw 'langstats installer: Windows is required'
    }
    if (-not $env:LOCALAPPDATA) {
        throw 'langstats installer: LOCALAPPDATA is required'
    }
    $installDir = Join-Path $env:LOCALAPPDATA 'Programs\langstats'
    $binaryPath = Join-Path $installDir 'langstats.exe'
    $installedVersion = ''
    if (Test-Path -LiteralPath $binaryPath -PathType Leaf) {
        try {
            $installedVersion = (& $binaryPath --version 2>$null | Out-String).Trim()
            if ($LASTEXITCODE -ne 0) { $installedVersion = '' }
        }
        catch { $installedVersion = '' }
    }

    # Follow the release-page redirect instead of the rate-limited REST API.
    Write-Host 'Checking langstats...'
    $stageTimer = [Diagnostics.Stopwatch]::StartNew()
    $latest = Invoke-WebRequest -Uri "$repo/releases/latest" -Method Head -TimeoutSec 60 -UseBasicParsing
    $latestUri = $latest.BaseResponse.ResponseUri
    if (-not $latestUri) {
        $latestUri = $latest.BaseResponse.RequestMessage.RequestUri
    }
    $prefix = "$repo/releases/tag/"
    if (-not $latestUri -or -not $latestUri.AbsoluteUri.StartsWith($prefix, [StringComparison]::Ordinal)) {
        throw 'langstats installer: unexpected latest release URL'
    }
    $tag = $latestUri.AbsoluteUri.Substring($prefix.Length)
    if ($tag -notmatch '^v[0-9][A-Za-z0-9._-]*$') {
        throw 'langstats installer: unexpected release tag'
    }

    Write-Timing 'Release lookup' $stageTimer
    $version = $tag.Substring(1)
    if ($installedVersion -eq "langstats $version" -and
        (Test-Path -LiteralPath (Join-Path $installDir 'LICENSE') -PathType Leaf) -and
        (Test-Path -LiteralPath (Join-Path $installDir 'NOTICE') -PathType Leaf)) {
        Write-Host "langstats $version is already up to date at $binaryPath"
        Write-Timing 'Total' $totalTimer
        return
    }

    if (-not (Get-Command curl.exe -ErrorAction SilentlyContinue)) {
        throw 'langstats installer: curl.exe is required'
    }
    $processor = Get-CimInstance -ClassName Win32_Processor | Select-Object -First 1
    $target = switch ([int]$processor.Architecture) {
        9  { 'x86_64-pc-windows-msvc' }
        12 { 'aarch64-pc-windows-msvc' }
        default { throw 'langstats installer: unsupported CPU architecture' }
    }

    $name = "langstats-$tag-$target"
    $archive = "$name.zip"
    $tempDir = Join-Path ([IO.Path]::GetTempPath()) ('langstats-install-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    $downloads = @()
    try {
        $url = "$repo/releases/download/$tag/$archive"
        $archivePath = Join-Path $tempDir $archive
        $checksumPath = "$archivePath.sha256"
        Write-Host "Downloading langstats $version..."
        $stageTimer.Restart()
        # Start both transfers before waiting; compatible with Windows PowerShell 5.1.
        # Quote output paths because Start-Process joins ArgumentList into one string.
        foreach ($transfer in @(
            @{ Path = $archivePath; Url = $url; Options = @('-fL', '--progress-bar') },
            @{ Path = $checksumPath; Url = "$url.sha256"; Options = @('-fsSL') }
        )) {
            $arguments = $transfer.Options + @('--connect-timeout', '10', '--max-time', '60',
                '-o', ('"{0}"' -f $transfer.Path), $transfer.Url)
            $downloads += Start-Process -FilePath 'curl.exe' -ArgumentList $arguments -NoNewWindow -PassThru
            # Retain a process handle so Windows PowerShell can read ExitCode after exit.
            $null = $downloads[-1].Handle
        }
        for ($i = 0; $i -lt $downloads.Count; $i++) {
            $downloads[$i].WaitForExit()
            if ($downloads[$i].ExitCode -ne 0) {
                $failedTransfer = if ($i -eq 0) { 'release archive' } else { 'checksum' }
                throw "langstats installer: could not download $failedTransfer"
            }
        }
        Write-Timing 'Downloads' $stageTimer
        $stageTimer.Restart()

        $checksumLine = (Get-Content -LiteralPath $checksumPath -Raw).Trim()
        $checksumPattern = '^([0-9a-fA-F]{64})\s+' + [regex]::Escape($archive) + '$'
        $checksumMatch = [regex]::Match($checksumLine, $checksumPattern)
        if (-not $checksumMatch.Success) {
            throw 'langstats installer: invalid checksum file'
        }
        $actual = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
        if ($actual -ne $checksumMatch.Groups[1].Value) {
            throw 'langstats installer: release archive SHA-256 mismatch'
        }

        Write-Timing 'Verification' $stageTimer
        $stageTimer.Restart()
        Expand-Archive -LiteralPath $archivePath -DestinationPath $tempDir
        $contents = Join-Path $tempDir $name
        foreach ($file in 'langstats.exe', 'LICENSE', 'NOTICE') {
            if (-not (Test-Path -LiteralPath (Join-Path $contents $file) -PathType Leaf)) {
                throw "langstats installer: missing $file in archive"
            }
        }

        $action = if (Test-Path -LiteralPath $binaryPath -PathType Leaf) { 'Updated' } else { 'Installed' }
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
        foreach ($file in 'langstats.exe', 'LICENSE', 'NOTICE') {
            Copy-Item -LiteralPath (Join-Path $contents $file) -Destination (Join-Path $installDir $file) -Force
        }

        $normalized = $installDir.TrimEnd([char]'\')
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $userEntries = @($userPath -split ';' | ForEach-Object { $_.TrimEnd([char]'\') })
        if ($userEntries -notcontains $normalized) {
            $newUserPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
            [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
        }
        $processEntries = @($env:Path -split ';' | ForEach-Object { $_.TrimEnd([char]'\') })
        if ($processEntries -notcontains $normalized) {
            $env:Path = if ($env:Path) { "$env:Path;$installDir" } else { $installDir }
        }
        Write-Timing 'Installation' $stageTimer
        Write-Host "$action langstats $version to $binaryPath"
        Write-Timing 'Total' $totalTimer
    }
    finally {
        foreach ($download in $downloads) {
            try {
                if (-not $download.HasExited) {
                    $download.Kill()
                    $download.WaitForExit()
                }
            }
            catch { } # A transfer may exit between HasExited and Kill().
            finally { $download.Dispose() }
        }
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
