
$ErrorActionPreference = "Continue"
Set-Location C:\Users\neri\code\playmat-bt\rust\rl
$tok = ""
Get-Content C:\Users\neri\code\playmat-bt\.env | ForEach-Object {
    if ($_ -match "^HF_TOKEN=(.*)$") {
        $tok = $matches[1].Trim().Trim([char]34, [char]39)
    }
}
if (-not $tok) { throw "HF_TOKEN missing" }

$baseArgs = @(
  "colab_train.py",
  "--name","phase3-r15b-5m-aux4",
  "--init-ckpt","wavec-final-5m-soup",
  "--oracle-ckpt","oracle-tx-6m-v3","--oracle-mix","0.30",
  "--rounds","obsv2-r1b","obsv2-r1c","obsv2-r1","obsv2-r2b","obsv2-r2c","obsv2-r2","obsv2-r3c","obsv2-r3","obsv2-r4b","obsv2-r4c","obsv2-r4","obsv2-r5c","obsv2-r5","obsv2-r5b","obsv2-r6c","obsv2-r6d","obsv2-r6","obsv2-r7c","obsv2-r7d","obsv2-r7","obsv2-r8c","obsv2-r8","obsv2-r8b","obsv2-r9","wavec-r10-gcp","wavec-r10b-gcp","wavec-r11-gcp","wavec-r11b-5m-gcp","wavec-r12-gcp","wavec-r12b-5m-gcp","wavec-r13-gcp","wavec-r13b-5m-gcp","wavec-r14-gcp",
  "--epochs","6","--lr","1.8e-4","--drop","0.14",
  "--val-prob","0.25","--val-wd","0.06","--value-weight","1.2","--aux-weight","0.20","--act-q-weight","0.20",
  "--recency-decay","0.92","--q-mix","0.25","--ema","0.997",
  "--policy-recent","9","--pol-old-weight","0.25",
  "--samples-per-epoch","2500000","--stage-chunk","800000"
)

foreach ($spec in @(
    @{gpu="A100"; bs="1024"; ggb="14.0"},
    @{gpu="L4";   bs="512";  ggb="6.0"},
    @{gpu="T4";   bs="256";  ggb="2.0"}
)) {
    $g = $spec.gpu
    Write-Host "TRY_COLAB GPU=$g $(Get-Date -Format o)"
    & C:\Users\neri\.local\bin\colab.exe --auth adc run --gpu $g --timeout 0 --env "HF_TOKEN=$tok" @baseArgs --bs $spec.bs --gpu-data-gb $spec.ggb
    if ($LASTEXITCODE -eq 0) {
        Write-Host "COLAB_P3_R15B_5M_DONE_ON_$g $(Get-Date -Format o)"
        exit 0
    }
    Start-Sleep -Seconds 15
}
exit 1
