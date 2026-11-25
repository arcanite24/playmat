$ErrorActionPreference = "Stop"
$envPath = "C:\Users\neri\code\playmat-bt\.env"
foreach ($line in Get-Content $envPath) {
    if ($line -match '^HF_TOKEN=(.+)$') { $env:HF_TOKEN = $Matches[1].Trim() }
}
$colab = "C:\Users\neri\.local\bin\colab.exe"
$py = "C:\Users\neri\code\playmat-bt\rust\rl\colab_phase3_showcase.py"
for ($attempt = 1; $attempt -le 3; $attempt++) {
    foreach ($gpu in @("L4", "A100")) {
        Write-Host "TRY_SHOWCASE attempt=$attempt GPU=$gpu $(Get-Date -Format o)"
        & $colab --auth adc run --gpu $gpu --timeout 0 --env "HF_TOKEN=$env:HF_TOKEN" $py -- --ckpt phase3-final-5m-v3-soup --c2-deals 60 --ood-deals 40 --iters 1024 --vloss 8 --jobs 12
        if ($LASTEXITCODE -eq 0) {
            Write-Host "SHOWCASE_OK GPU=$gpu $(Get-Date -Format o)"
            exit 0
        }
        Write-Host "SHOWCASE_RETRY attempt=$attempt GPU=$gpu exit=$LASTEXITCODE"
        Start-Sleep -Seconds 25
    }
}
exit 1
