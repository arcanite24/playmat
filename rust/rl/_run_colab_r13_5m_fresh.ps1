
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
  "--name","wavec-r13-5m-fresh",
  "--arch","tx",
  "--tx","d=256,heads=8,layers=6,ff=768,dim=48,trunk=256,text=2,val_mlp=1,gelu=1,glob_f=81,rel=1,act_ent=1",
  "--oracle-ckpt","oracle-tx-6m-v3","--oracle-mix","0.25",
  "--rounds","obsv2-r1b","obsv2-r1c","obsv2-r1","obsv2-r2b","obsv2-r2c","obsv2-r2","obsv2-r3c","obsv2-r3","obsv2-r4b","obsv2-r4c","obsv2-r4","obsv2-r5c","obsv2-r5","obsv2-r5b","obsv2-r6c","obsv2-r6d","obsv2-r6","obsv2-r7c","obsv2-r7d","obsv2-r7","obsv2-r8c","obsv2-r8","obsv2-r8b","obsv2-r9","wavec-r10-gcp","wavec-r10b-gcp","wavec-r11-gcp","wavec-r11b-5m-gcp","wavec-r12-gcp","wavec-r12b-5m-gcp","wavec-r13-gcp",
  "--lr","1.2e-3","--drop","0.10",
  "--val-prob","0.25","--val-wd","0.05","--value-weight","1.2","--aux-weight","0.15",
  "--q-mix","0.25","--ema","0.996",
  "--policy-recent","7","--pol-old-weight","0.25",
  "--stage-chunk","1000000"
)

for ($attempt = 1; $attempt -le 3; $attempt++) {
    foreach ($spec in @(
        @{gpu="H100"; ep="14"; bs="2048"; spe="3500000"; ggb="18.0"},
        @{gpu="A100"; ep="14"; bs="2048"; spe="3500000"; ggb="14.0"},
        @{gpu="L4";   ep="10"; bs="1024"; spe="2500000"; ggb="6.0"},
        @{gpu="G4";   ep="10"; bs="1024"; spe="2500000"; ggb="6.0"}
    )) {
        $g = $spec.gpu
        Write-Host "TRY_COLAB attempt=$attempt GPU=$g $(Get-Date -Format o)"
        & C:\Users\neri\.local\bin\colab.exe --auth adc run --gpu $g --timeout 0 --env "HF_TOKEN=$tok" @baseArgs --epochs $spec.ep --bs $spec.bs --samples-per-epoch $spec.spe --gpu-data-gb $spec.ggb
        if ($LASTEXITCODE -eq 0) {
            Write-Host "COLAB_WAVEC_R13_5M_FRESH_DONE_ON_$g $(Get-Date -Format o)"
            exit 0
        }
        Start-Sleep -Seconds 20
    }
    Start-Sleep -Seconds 90
}
exit 1

