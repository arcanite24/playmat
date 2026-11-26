param(
    [Parameter(Mandatory=$true)][string]$Name,
    [Parameter(Mandatory=$true)][string]$InitCkpt,
    [string]$ExtraRounds = "",
    [string]$OracleCkpt = "oracle-tx-6m-v4",
    [double]$OracleMix = 0.25,
    [switch]$OracleMode,
    [int]$UpgradeGlobF = 97,
    [double]$ThreatWeight = 0.15,
    [double]$OptPolWeight = 0.20,
    [string]$Opt = "muon",
    [double]$MuonLr = 0.004,
    [int]$Epochs = 6,
    [int]$Bs = 2048,
    [double]$Lr = 5e-4,
    [int]$PolicyRecent = 12,
    [double]$RecencyDecay = 0.93,
    [int]$SamplesPerEpoch = 2000000
)
$ErrorActionPreference = "Stop"
$envPath = "C:\Users\neri\code\playmat-bt\.env"
foreach ($line in Get-Content $envPath) {
    if ($line -match '^HF_TOKEN=(.+)$') { $env:HF_TOKEN = $Matches[1].Trim() }
}
$rounds = @(
    "obsv2-r1","obsv2-r1b","obsv2-r1c",
    "obsv2-r2","obsv2-r2b","obsv2-r2c",
    "obsv2-r3","obsv2-r3c",
    "obsv2-r4","obsv2-r4b","obsv2-r4c",
    "obsv2-r5","obsv2-r5b","obsv2-r5c",
    "obsv2-r6","obsv2-r6c","obsv2-r6d",
    "obsv2-r7","obsv2-r7c","obsv2-r7d",
    "obsv2-r8","obsv2-r8b","obsv2-r8c",
    "obsv2-r9",
    "wavec-r10-gcp","wavec-r10b-gcp",
    "wavec-r11-gcp","wavec-r11b-5m-gcp",
    "wavec-r12-gcp","wavec-r12b-5m-gcp",
    "wavec-r13-gcp","wavec-r13b-5m-gcp",
    "wavec-r14-gcp",
    "phase3-r16-pfsp-gcp",
    "phase3-r17-pfsp-gcp",
    "phase3-r18-pfsp-gcp"
)
if ($ExtraRounds -ne "") {
    foreach ($er in $ExtraRounds.Split(",")) {
        if ($er.Trim() -ne "") { $rounds += $er.Trim() }
    }
}
$scriptArgs = @(
    "--name", $Name,
    "--init-ckpt", $InitCkpt,
    "--upgrade-glob-f", "$UpgradeGlobF",
    "--threat-weight", "$ThreatWeight",
    "--opt-pol-weight", "$OptPolWeight",
    "--opt", "$Opt",
    "--muon-lr", "$MuonLr",
    "--epochs", "$Epochs",
    "--lr", "$Lr",
    "--drop", "0.10",
    "--val-prob", "0.05",
    "--val-wd", "0.05",
    "--aux-weight", "0.20",
    "--act-q-weight", "0.50",
    "--recency-decay", "$RecencyDecay",
    "--policy-recent", "$PolicyRecent",
    "--pol-old-weight", "0.25",
    "--samples-per-epoch", "$SamplesPerEpoch"
)
if ($OracleMode) {
    $scriptArgs += "--oracle-mode"
} elseif ($OracleCkpt -ne "" -and $OracleMix -gt 0) {
    $scriptArgs += @("--oracle-ckpt", $OracleCkpt, "--oracle-mix", "$OracleMix")
}
$scriptArgs += "--rounds"
$scriptArgs += $rounds

$colab = "C:\Users\neri\.local\bin\colab.exe"
$py = "C:\Users\neri\code\playmat-bt\rust\rl\colab_train.py"
foreach ($gpu in @("A100", "L4", "A100", "L4", "A100", "L4", "A100", "L4")) {
    $runBs = if ($gpu -eq "T4" -and $Bs -gt 512) { 512 } elseif ($gpu -eq "L4" -and $Bs -gt 1024) { 1024 } else { $Bs }
    $runGgb = if ($gpu -eq "T4") { "2.0" } elseif ($gpu -eq "L4") { "3.0" } else { "18.0" }
    $runChunk = if ($gpu -eq "T4") { "300000" } elseif ($gpu -eq "L4") { "400000" } else { "1500000" }
    $gpuArgs = @("--bs", "$runBs", "--gpu-data-gb", $runGgb, "--stage-chunk", $runChunk) + $scriptArgs
    Write-Host "TRY_P4_TRAIN $Name GPU=$gpu bs=$runBs ggb=$runGgb $(Get-Date -Format o)"
    & $colab --auth adc run --gpu $gpu --timeout 0 --env "HF_TOKEN=$env:HF_TOKEN" $py -- @gpuArgs
    if ($LASTEXITCODE -eq 0) {
        Write-Host "P4_TRAIN_OK $Name GPU=$gpu $(Get-Date -Format o)"
        exit 0
    }
    Write-Host "P4_TRAIN_RETRY $Name GPU=$gpu exit=$LASTEXITCODE"
    Start-Sleep -Seconds 45
}
exit 1
