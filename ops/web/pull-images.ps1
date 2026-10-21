<#
.SYNOPSIS
Runs the stack on published images instead of building them: Multivac follows master this way, as staging.

.DESCRIPTION
Pulls ghcr.io/arcanite24/playmat-server:<Tag> and playmat-web:<Tag> (master by default: the newest build of master,
.github/workflows/images.yml), pins their digests in ops\web\.env (XMAGE_SERVER_IMAGE, XMAGE_WEB_IMAGE), and runs
docker compose up -d --no-build with the compose kit the server image carries (extracted to ops\web\deploy-kit), so
the compose file always matches the images. Nothing changes when the pinned digests are already the newest. When the
new version isn't healthy within five minutes, the previous .env comes back and the stack restarts on it.

The images are public: no registry login or secrets. On Windows, Docker Desktop's credential helper fails in an SSH
session, so run this the way the build-based redeploy runs: from a scheduled task in the console session
(README "Multivac follows master"). Windows PowerShell 5.1 or later.

    powershell -NoProfile -ExecutionPolicy Bypass -File ops\web\pull-images.ps1
    powershell -NoProfile -ExecutionPolicy Bypass -File ops\web\pull-images.ps1 -Tag v1.2.3

To build locally again, empty XMAGE_SERVER_IMAGE and XMAGE_WEB_IMAGE in .env.

.PARAMETER Tag
The image tag: master (default), vX.Y.Z or sha-<7 hex>.

.PARAMETER Project
The compose project. Empty: the project the running xmage container belongs to, else compose's own default.

.PARAMETER Log
Where to append the log (default: pull-images.log beside .env).
#>
param(
    [string]$Tag = 'master',
    [string]$Project = '',
    [string]$Log = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

$registry = 'ghcr.io/arcanite24'
$webDir = $PSScriptRoot
$envFile = Join-Path $webDir '.env'
$kitDir = Join-Path $webDir 'deploy-kit'
$healthSeconds = 300
if (-not $Log) { $Log = Join-Path $webDir 'pull-images.log' }

function Write-Log([string]$message) {
    $line = '{0} {1}' -f (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'), $message
    Write-Host $line
    Add-Content -Path $Log -Value $line
}

# docker, its output returned and its errors shown, failing on a non-zero exit. Windows PowerShell turns a native
# command's stderr into error records when its own output is redirected (a scheduled task's log), which 'Stop' would
# treat as fatal; the exit code decides here.
function Invoke-Docker {
    $ErrorActionPreference = 'Continue'
    $output = & docker @args 2>&1 | ForEach-Object {
        if ($_ -is [System.Management.Automation.ErrorRecord]) { Write-Host $_.ToString() } else { $_ }
    }
    if ($LASTEXITCODE -ne 0) { throw "docker $($args -join ' ') failed ($LASTEXITCODE)" }
    $output
}

# .env without a byte order mark: compose would read one as part of the first name
$utf8 = New-Object System.Text.UTF8Encoding $false
function Read-EnvLines { if (Test-Path $envFile) { [System.IO.File]::ReadAllLines($envFile) } else { @() } }
function Write-EnvLines([string[]]$lines) { [System.IO.File]::WriteAllLines($envFile, $lines, $utf8) }

function Get-EnvValue([string]$key) {
    $value = ''
    foreach ($line in Read-EnvLines) { if ($line.StartsWith("$key=")) { $value = $line.Substring($key.Length + 1) } }
    $value
}

function Set-EnvValue([string]$key, [string]$value) {
    $lines = New-Object System.Collections.Generic.List[string]
    $done = $false
    foreach ($line in Read-EnvLines) {
        if ($line.StartsWith("$key=")) {
            if (-not $done) { $lines.Add("$key=$value"); $done = $true }
        } else {
            $lines.Add($line)
        }
    }
    if (-not $done) { $lines.Add("$key=$value") }
    Write-EnvLines $lines.ToArray()
}

# pulls repo:tag and returns repo:tag@sha256:...
function Get-PinnedImage([string]$repo) {
    $ref = "${repo}:$Tag"
    Invoke-Docker pull -q $ref | Out-Null
    $digests = Invoke-Docker image inspect -f '{{range .RepoDigests}}{{println .}}{{end}}' $ref
    $digest = @($digests | Where-Object { $_ -like "$repo@sha256:*" })[0]
    if (-not $digest) { throw "no digest for $ref" }
    "$ref@$($digest.Split('@')[1])"
}

function Get-Health([string]$service) {
    $id = @(Invoke-Docker ps -q --filter "label=com.docker.compose.project=$Project" --filter "label=com.docker.compose.service=$service")[0]
    if (-not $id) { return 'missing' }
    Invoke-Docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' $id
}

function Start-Stack {
    $compose = @('compose', '-p', $Project, '--project-directory', $kitDir, '-f', (Join-Path $kitDir 'docker-compose.yml'),
        '--env-file', $envFile, 'up', '-d', '--no-build', '--remove-orphans')
    try { Invoke-Docker @compose | Out-Host } catch { Write-Log $_.Exception.Message }
    $deadline = (Get-Date).AddSeconds($healthSeconds)
    while ((Get-Date) -lt $deadline) {
        $xmage = Get-Health 'xmage'
        $web = Get-Health 'web'
        if ($xmage -eq 'healthy' -and $web -eq 'healthy') { return $true }
        Start-Sleep -Seconds 10
    }
    Write-Log "not healthy after ${healthSeconds}s: xmage $(Get-Health 'xmage'), web $(Get-Health 'web')"
    $false
}

try {
    if ($Tag -notmatch '^(v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?|sha-[0-9a-f]{7,40}|master)$') {
        throw "'$Tag' is not an image tag (master, vX.Y.Z, sha-<hex>)"
    }
    if (-not (Test-Path $envFile)) { throw "$envFile is missing (copy .env.example and edit it)" }
    if (-not $Project) {
        # all the labels, parsed here: Windows PowerShell drops the inner quotes of {{.Label "..."}} on the way to docker
        $labels = @(Invoke-Docker ps --filter 'label=com.docker.compose.service=xmage' --format '{{.Labels}}')[0]
        if ($labels -match '(?:^|,)com\.docker\.compose\.project=([^,]+)') { $Project = $Matches[1] } else { $Project = 'web' }
    }

    $server = Get-PinnedImage "$registry/playmat-server"
    $webImage = Get-PinnedImage "$registry/playmat-web"
    $oldServer = Get-EnvValue 'XMAGE_SERVER_IMAGE'
    $oldWeb = Get-EnvValue 'XMAGE_WEB_IMAGE'
    if ($server -eq $oldServer -and $webImage -eq $oldWeb -and (Get-Health 'xmage') -eq 'healthy') {
        Write-Log "already on $server"
        exit 0
    }
    Write-Log "updating project $Project to $server and $webImage"

    # the compose kit that belongs with these images
    $kitNext = "$kitDir.next"
    if (Test-Path $kitNext) { Remove-Item -Recurse -Force $kitNext }
    New-Item -ItemType Directory -Path $kitNext | Out-Null
    $container = @(Invoke-Docker create $server)[0]
    try { Invoke-Docker cp "${container}:/opt/xmage/deploy-kit/." $kitNext | Out-Null }
    finally { Invoke-Docker rm $container | Out-Null }
    if (-not (Test-Path (Join-Path $kitNext 'docker-compose.yml'))) { throw "$server carries no compose kit" }
    $kitOld = "$kitDir.previous"
    if (Test-Path $kitOld) { Remove-Item -Recurse -Force $kitOld }
    if (Test-Path $kitDir) { Rename-Item $kitDir (Split-Path $kitOld -Leaf) }
    Rename-Item $kitNext (Split-Path $kitDir -Leaf)

    $backup = "$envFile.pull-images.bak"
    Copy-Item $envFile $backup -Force
    Set-EnvValue 'XMAGE_SERVER_IMAGE' $server
    Set-EnvValue 'XMAGE_WEB_IMAGE' $webImage

    if (Start-Stack) {
        Write-Log "running $Tag ($server)"
        Invoke-Docker image prune -f | Out-Null
        exit 0
    }

    Write-Log "rolling back to the previous .env ($oldServer)"
    Copy-Item $backup $envFile -Force
    if (Test-Path $kitOld) {
        Remove-Item -Recurse -Force $kitDir
        Rename-Item $kitOld (Split-Path $kitDir -Leaf)
    } elseif (-not $oldServer) {
        # the first switch from local builds: the checkout's own compose file
        Copy-Item (Join-Path $webDir 'docker-compose.yml'), (Join-Path $webDir 'imgcache.conf.template') $kitDir -Force
    }
    if (Start-Stack) { Write-Log 'rolled back; healthy' } else { Write-Log 'rolled back, and still not healthy: look at it' }
    exit 1
} catch {
    Write-Log "failed: $($_.Exception.Message)"
    exit 1
}
