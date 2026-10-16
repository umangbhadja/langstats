& {
    $ErrorActionPreference = 'Stop'
    $repo = 'https://github.com/umangbhadja/langstats'

    if ($env:OS -ne 'Windows_NT') {
        throw 'langstats installer: Windows is required'
    }
    if (-not $env:LOCALAPPDATA) {
        throw 'langstats installer: LOCALAPPDATA is required'
    }

    $processor = Get-CimInstance -ClassName Win32_Processor | Select-Object -First 1
    $target = switch ([int]$processor.Architecture) {
        9  { 'x86_64-pc-windows-msvc' }
        12 { 'aarch64-pc-windows-msvc' }
        default { throw 'langstats installer: unsupported CPU architecture' }
    }

    # Follow the release-page redirect instead of the rate-limited REST API.
    $latest = Invoke-WebRequest -Uri "$repo/releases/latest" -UseBasicParsing
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

    $name = "langstats-$tag-$target"
    $archive = "$name.zip"
    $tempDir = Join-Path ([IO.Path]::GetTempPath()) ('langstats-install-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    try {
        $url = "$repo/releases/download/$tag/$archive"
        $archivePath = Join-Path $tempDir $archive
        $checksumPath = "$archivePath.sha256"
        Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing
        Invoke-WebRequest -Uri "$url.sha256" -OutFile $checksumPath -UseBasicParsing

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

        Expand-Archive -LiteralPath $archivePath -DestinationPath $tempDir
        $contents = Join-Path $tempDir $name
        foreach ($file in 'langstats.exe', 'LICENSE', 'NOTICE') {
            if (-not (Test-Path -LiteralPath (Join-Path $contents $file) -PathType Leaf)) {
                throw "langstats installer: missing $file in archive"
            }
        }

        $installDir = Join-Path $env:LOCALAPPDATA 'Programs\langstats'
        $binaryPath = Join-Path $installDir 'langstats.exe'
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
        Write-Host "$action langstats $($tag.Substring(1)) to $binaryPath"
    }
    finally {
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
