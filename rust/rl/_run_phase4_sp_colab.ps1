param(
    [Parameter(Mandatory=$true)][string]$Round,
    [Parameter(Mandatory=$true)][string]$LearnerCkpt,
    [string]$SibCkpt = "pre4-muon004-optpol-5m",
    [string]$OodCkpt = "pre4-hlgauss21-5m",
    [int]$Games = 1200,
    [int]$Iters = 512,
    [int]$Seed = 19000,
    [int]$Tag = 19
)
$ErrorActionPreference = "Stop"
$envPath = "C:\Users\neri\code\playmat-bt\.env"
foreach ($line in Get-Content $envPath) {
    if ($line -match '^HF_TOKEN=(.+)$') { $env:HF_TOKEN = $Matches[1].Trim() }
}
$colab = "C:\Users\neri\.local\bin\colab.exe"
$py = "C:\Users\neri\code\playmat-bt\rust\rl\colab_phase4_selfplay.py"
$sargs = @(
    "--round", $Round,
    "--learner-ckpt", $LearnerCkpt,
    "--sib-ckpt", $SibCkpt,
    "--ood-ckpt", $OodCkpt,
    "--games", "$Games",
    "--iters", "$Iters",
    "--seed", "$Seed",
    "--tag", "$Tag"
)
foreach ($gpu in @("A100", "L4", "A100", "L4", "A100", "L4")) {
    Write-Host "TRY_P4_SP $Round GPU=$gpu $(Get-Date -Format o)"
    & $colab --auth adc run --gpu $gpu --timeout 0 --env "HF_TOKEN=$env:HF_TOKEN" $py -- @sargs
    if ($LASTEXITCODE -eq 0) {
        Write-Host "P4_SP_OK $Round GPU=$gpu $(Get-Date -Format o)"
        exit 0
    }
    Write-Host "P4_SP_RETRY $Round GPU=$gpu exit=$LASTEXITCODE"
    Start-Sleep -Seconds 45
}
exit 1
