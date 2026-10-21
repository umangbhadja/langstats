# Run with Windows PowerShell 5.1: powershell -NoProfile -File scripts/test_install.ps1
$ErrorActionPreference = 'Stop'
$installer = Join-Path $PSScriptRoot 'install.ps1'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('langstats tests ' + [guid]::NewGuid().ToString('N'))
$oldLocalAppData = $env:LOCALAPPDATA
$oldProcessPath = $env:Path
$oldOS = $env:OS
# Redirect the static registry boundary in the test copy; never modify the user's PATH.
$source = Get-Content -LiteralPath $installer -Raw
$getUserPath = "[Environment]::GetEnvironmentVariable('Path', 'User')"
$setUserPath = '[Environment]::SetEnvironmentVariable(''Path'', $newUserPath, ''User'')'
if (-not $source.Contains($getUserPath) -or -not $source.Contains($setUserPath)) {
    throw 'Update the user PATH test boundary to match the installer.'
}
$installerUnderTest = [scriptblock]::Create($source.Replace($getUserPath, '$script:testUserPath').Replace($setUserPath, '$script:testUserPath = $newUserPath'))
$env:OS = 'Windows_NT'
New-Item -ItemType Directory -Path $testRoot | Out-Null

function Assert($condition, $message) {
    if (-not $condition) { throw "Assertion failed: $message" }
}

# Mock only network/process boundaries; archive extraction and SHA-256 remain real.
function Invoke-WebRequest {
    param($Uri, $Method, $TimeoutSec, [switch]$UseBasicParsing)
    Assert ($Method -eq 'Head') 'release lookup must use HEAD'
    Assert ($TimeoutSec -eq 60) 'release lookup must have a bounded timeout'
    $resolved = if ($script:scenario -eq 'bad-url') {
        'https://example.com/releases/tag/v1.2.3'
    } elseif ($script:scenario -eq 'bad-tag') {
        'https://github.com/umangbhadja/langstats/releases/tag/not-a-version'
    } else { 'https://github.com/umangbhadja/langstats/releases/tag/v1.2.3' }
    [pscustomobject]@{ BaseResponse = [pscustomobject]@{ ResponseUri = [uri]$resolved } }
}
function Get-Command { param($Name, $ErrorAction) [pscustomobject]@{ Name = 'curl.exe' } }
function Get-CimInstance { param($ClassName) [pscustomobject]@{ Architecture = 9 } }
function Write-Host { param($Object) $script:messages += [string]$Object }
function Start-Process {
    param($FilePath, $ArgumentList, [switch]$NoNewWindow, [switch]$PassThru)
    $script:started++
    Assert ($FilePath -eq 'curl.exe') 'downloads use curl.exe'
    Assert (($ArgumentList -join ' ') -match '--connect-timeout 10 --max-time 60') 'transfer timeouts retained'
    $outputIndex = [array]::IndexOf($ArgumentList, '-o') + 1
    $quotedOutput = $ArgumentList[$outputIndex]
    Assert ($quotedOutput.StartsWith('"') -and $quotedOutput.EndsWith('"')) 'output paths quoted'
    $destination = $quotedOutput.Trim('"')
    $script:downloadDir = Split-Path $destination
    if ($script:scenario -eq 'spawn-failure' -and $script:started -eq 2) {
        throw 'simulated process start failure'
    }
    $isChecksum = $destination.EndsWith('.sha256')
    Copy-Item -LiteralPath $(if ($isChecksum) { $script:checksum } else { $script:archive }) -Destination $destination
    $exitCode = if (($script:scenario -eq 'archive-failure' -and -not $isChecksum) -or
        ($script:scenario -eq 'checksum-failure' -and $isChecksum)) { 22 } else { 0 }
    $process = [pscustomobject]@{ HasExited = $false; Handle = 1; ExitCode = $exitCode; Disposed = $false; Killed = $false }
    $process | Add-Member -MemberType ScriptMethod -Name WaitForExit -Value {
        if (-not $this.Killed) { Assert ($script:started -eq 2) 'both transfers must start before waiting' }
        $this.HasExited = $true
    }
    $process | Add-Member -MemberType ScriptMethod -Name Kill -Value { $this.Killed = $true }
    $process | Add-Member -MemberType ScriptMethod -Name Dispose -Value { $this.Disposed = $true }
    $script:processes += $process
    $process
}

try {
    $name = 'langstats-v1.2.3-x86_64-pc-windows-msvc'
    $fixtureRoot = Join-Path $testRoot 'fixture'
    $contents = Join-Path $fixtureRoot $name
    New-Item -ItemType Directory -Path $contents | Out-Null
    Set-Content -LiteralPath (Join-Path $contents 'langstats.exe') -Value 'new binary'
    Set-Content -LiteralPath (Join-Path $contents 'LICENSE') -Value 'license'
    Set-Content -LiteralPath (Join-Path $contents 'NOTICE') -Value 'notice'
    $script:archive = Join-Path $testRoot "$name.zip"
    $script:checksum = "$script:archive.sha256"

    foreach ($case in @(
        @{ Name = 'success'; Error = '' },
        @{ Name = 'archive-failure'; Error = 'could not download release archive' },
        @{ Name = 'checksum-failure'; Error = 'could not download checksum' },
        @{ Name = 'spawn-failure'; Error = 'simulated process start failure' },
        @{ Name = 'bad-checksum'; Error = 'SHA-256 mismatch' },
        @{ Name = 'bad-manifest'; Error = 'invalid checksum file' },
        @{ Name = 'missing-notice'; Error = 'missing NOTICE in archive' },
        @{ Name = 'bad-url'; Error = 'unexpected latest release URL' },
        @{ Name = 'bad-tag'; Error = 'unexpected release tag' }
    )) {
        $script:scenario = $case.Name
        $script:started = 0
        $script:processes = @()
        $script:messages = @()
        $script:downloadDir = ''
        $env:LOCALAPPDATA = Join-Path $testRoot $case.Name
        $destination = Join-Path $env:LOCALAPPDATA 'Programs\langstats'
        New-Item -ItemType Directory -Path $destination -Force | Out-Null
        $existing = Join-Path $destination 'langstats.exe'
        Set-Content -LiteralPath $existing -Value 'existing binary'
        Set-Content -LiteralPath (Join-Path $destination 'LICENSE') -Value 'existing license'
        Set-Content -LiteralPath (Join-Path $destination 'NOTICE') -Value 'existing notice'
        $script:testUserPath = 'existing-user-path'
        $env:Path = "$oldProcessPath;$destination"

        Set-Content -LiteralPath (Join-Path $contents 'NOTICE') -Value 'notice'
        if ($case.Name -eq 'missing-notice') { Remove-Item -LiteralPath (Join-Path $contents 'NOTICE') }
        if (Test-Path -LiteralPath $script:archive) { Remove-Item -LiteralPath $script:archive }
        Compress-Archive -LiteralPath $contents -DestinationPath $script:archive
        $digest = (Get-FileHash -LiteralPath $script:archive -Algorithm SHA256).Hash
        if ($case.Name -eq 'bad-checksum') { $digest = '0' * 64 }
        $manifest = if ($case.Name -eq 'bad-manifest') { 'invalid' } else { "$digest  $name.zip" }
        Set-Content -LiteralPath $script:checksum -Value $manifest

        $failure = ''
        try { & $installerUnderTest } catch { $failure = $_.Exception.Message }
        if ($case.Error) {
            Assert ($failure.Contains($case.Error)) "$($case.Name): expected '$($case.Error)', got '$failure'"
            Assert ((Get-Content -LiteralPath $existing -Raw).Trim() -eq 'existing binary') 'failed install preserves binary'
            Assert ((Get-Content -LiteralPath (Join-Path $destination 'LICENSE') -Raw).Trim() -eq 'existing license') 'failed install preserves license'
            Assert ((Get-Content -LiteralPath (Join-Path $destination 'NOTICE') -Raw).Trim() -eq 'existing notice') 'failed install preserves notice'
        } else {
            Assert (-not $failure) "success install failed: $failure"
            Assert ((Get-Content -LiteralPath $existing -Raw).Trim() -eq 'new binary') 'binary installed'
            Assert ($script:testUserPath -eq "existing-user-path;$destination") 'user PATH receives install directory'
            foreach ($stage in 'Release lookup', 'Downloads', 'Verification', 'Installation', 'Total') {
                Assert (@($script:messages | Where-Object { $_ -match "^${stage}: [0-9]+[.,][0-9]s$" }).Count -eq 1) "$stage timing printed"
            }
        }
        if ($script:downloadDir) { Assert (-not (Test-Path -LiteralPath $script:downloadDir)) 'temporary downloads removed' }
        foreach ($process in $script:processes) {
            Assert $process.HasExited 'download process ended'
            Assert $process.Disposed 'download process disposed'
            if ($case.Name -eq 'spawn-failure') { Assert $process.Killed 'running download stopped after sibling start failure' }
            if ($case.Name -eq 'archive-failure' -and $process -eq $script:processes[1]) {
                Assert $process.Killed 'checksum download stopped after archive failure'
            }
        }
        Write-Output "PASS: $($case.Name)"
    }
}
finally {
    $env:LOCALAPPDATA = $oldLocalAppData
    $env:Path = $oldProcessPath
    $env:OS = $oldOS
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}
