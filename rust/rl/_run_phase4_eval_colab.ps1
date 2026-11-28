param(
    [Parameter(Mandatory=$true)][string]$Preset,
    [string]$Suite = "constructed-2",
    [int]$Deals = 30,
    [int]$Jobs = 24,
    [int]$Vloss = 16,
    [int]$ExtraTurns = 2
)
$ErrorActionPreference = "Stop"
$envPath = "C:\Users\neri\code\playmat-bt\.env"
foreach ($line in Get-Content $envPath) {
    if ($line -match '^HF_TOKEN=(.+)$') { $env:HF_TOKEN = $Matches[1].Trim() }
}
$colab = "C:\Users\neri\.local\bin\colab.exe"
$py = "C:\Users\neri\code\playmat-bt\rust\rl\colab_phase4_eval.py"
$sargs = @(
    "--preset", $Preset,
    "--suite", $Suite,
    "--deals", "$Deals",
    "--jobs", "$Jobs",
    "--vloss", "$Vloss",
    "--extra-turns", "$ExtraTurns"
)
foreach ($gpu in @("L4", "A100", "L4", "A100", "L4", "A100", "L4", "A100")) {
    Write-Host "TRY_P4_EVAL $Preset $Suite deals=$Deals GPU=$gpu $(Get-Date -Format o)"
    & $colab --auth adc run --gpu $gpu --timeout 0 --env "HF_TOKEN=$env:HF_TOKEN" $py -- @sargs
    if ($LASTEXITCODE -eq 0) {
        Write-Host "P4_EVAL_OK $Preset $Suite GPU=$gpu $(Get-Date -Format o)"
        exit 0
    }
    Write-Host "P4_EVAL_RETRY $Preset $Suite GPU=$gpu exit=$LASTEXITCODE"
    Start-Sleep -Seconds 45
}
exit 1
