
$ErrorActionPreference = "Stop"
Set-Location C:\Users\neri\code\playmat-bt\rust\rl
$tok = ""
Get-Content C:\Users\neri\code\playmat-bt\.env | ForEach-Object {
    if ($_ -match "^HF_TOKEN=(.*)$") {
        $tok = $matches[1].Trim().Trim([char]34, [char]39)
    }
}
if (-not $tok) { throw "HF_TOKEN missing" }

Write-Host "START_COLAB_L4_WAVEC_R11B_10M $(Get-Date -Format o)"
& C:\Users\neri\.local\bin\colab.exe --auth adc run `
  --gpu L4 --timeout 0 `
  --env "HF_TOKEN=$tok" `
  colab_train.py `
  --name wavec-r11b-10m `
  --init-ckpt wavec-r11-10m `
  --oracle-ckpt oracle-tx-6m-v2 --oracle-mix 0.35 `
  --rounds obsv2-r1b obsv2-r1c obsv2-r1 obsv2-r2b obsv2-r2c obsv2-r2 obsv2-r3c obsv2-r3 obsv2-r4b obsv2-r4c obsv2-r4 obsv2-r5c obsv2-r5 obsv2-r5b obsv2-r6c obsv2-r6d obsv2-r6 obsv2-r7c obsv2-r7d obsv2-r7 obsv2-r8c obsv2-r8 obsv2-r8b obsv2-r9 wavec-r10-gcp wavec-r10b-gcp wavec-r11-gcp wavec-r11b-5m-gcp `
  --epochs 5 --bs 1024 --lr 3e-4 --drop 0.12 `
  --val-prob 0.25 --val-wd 0.05 --value-weight 1.2 --aux-weight 0.15 `
  --q-mix 0.15 --ema 0.996 `
  --policy-recent 4 --pol-old-weight 0.20 `
  --samples-per-epoch 2500000 --stage-chunk 800000 --gpu-data-gb 6.0
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "COLAB_L4_WAVEC_R11B_10M_DONE $(Get-Date -Format o)"

