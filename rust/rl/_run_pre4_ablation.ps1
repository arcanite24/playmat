param(
    [Parameter(Mandatory=$true)][string]$Name,
    [Parameter(Mandatory=$true)][string]$InitCkpt,
    [string]$OracleCkpt = "oracle-tx-6m-v4",
    [double]$OracleMix = 0.25,
    [int]$UpgradeGlobF = 89,
    [int]$UpgradeValBins = 0,
    [double]$OptPolWeight = 0.0,
    [string]$Opt = "adamw",
    [double]$MuonLr = 0.012,
    [int]$Epochs = 6,
    [int]$Bs = 2048,
    [double]$Lr = 5e-4,
    [double]$ValProb = 0.05,
    [double]$AuxWeight = 0.20,
    [double]$ActQWeight = 0.50,
    [double]$RecencyDecay = 0.92,
    [int]$PolicyRecent = 10,
    [double]$PolOldWeight = 0.25,
    [int]$SamplesPerEpoch = 1800000
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
$scriptArgs = @(
    "--name", $Name,
    "--init-ckpt", $InitCkpt,
    "--upgrade-glob-f", "$UpgradeGlobF",
    "--upgrade-val-bins", "$UpgradeValBins",
    "--opt-pol-weight", "$OptPolWeight",
    "--opt", "$Opt",
    "--muon-lr", "$MuonLr",
    "--epochs", "$Epochs",
    "--bs", "$Bs",
    "--lr", "$Lr",
    "--drop", "0.10",
    "--val-prob", "$ValProb",
    "--val-wd", "0.05",
    "--aux-weight", "$AuxWeight",
    "--act-q-weight", "$ActQWeight",
    "--recency-decay", "$RecencyDecay",
    "--policy-recent", "$PolicyRecent",
    "--pol-old-weight", "$PolOldWeight",
    "--samples-per-epoch", "$SamplesPerEpoch",
    "--gpu-data-gb", "18.0"
)
if ($OracleCkpt -ne "" -and $OracleMix -gt 0) {
    $scriptArgs += @("--oracle-ckpt", $OracleCkpt, "--oracle-mix", "$OracleMix")
}
$scriptArgs += "--rounds"
$scriptArgs += $rounds

$colab = "C:\Users\neri\.local\bin\colab.exe"
$py = "C:\Users\neri\code\playmat-bt\rust\rl\colab_train.py"

foreach ($gpu in @("L4", "A100", "L4", "A100", "L4", "A100")) {
    Write-Host "TRY_PRE4_TRAIN $Name GPU=$gpu $(Get-Date -Format o)"
    & $colab --auth adc run --gpu $gpu --high-mem --timeout 0 --env "HF_TOKEN=$env:HF_TOKEN" $py -- @scriptArgs
    if ($LASTEXITCODE -eq 0) {
        Write-Host "PRE4_TRAIN_OK $Name GPU=$gpu $(Get-Date -Format o)"
        exit 0
    }
    Write-Host "PRE4_TRAIN_RETRY $Name GPU=$gpu exit=$LASTEXITCODE"
    Start-Sleep -Seconds 45
}
exit 1
