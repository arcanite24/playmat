
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
  "colab_gpu_mcts.py",
  "--ckpt","phase3-r15b-5m-aux4",
  "--suite","constructed-2",
  "--deals","100",
  "--jobs","8",
  "--iters","1024",
  "--vloss","8",
  "--samples","8",
  "--extra-turns","3",
  "--opponent","host@s6",
  "--tactics","tactics-4"
)

foreach ($g in @("L4", "A100", "T4")) {
    Write-Host "TRY_GPU_MCTS GPU=$g $(Get-Date -Format o)"
    & C:\Users\neri\.local\bin\colab.exe --auth adc run --gpu $g --timeout 0 --env "HF_TOKEN=$tok" @baseArgs
    if ($LASTEXITCODE -eq 0) {
        Write-Host "COLAB_GPU_MCTS_DONE_ON_$g $(Get-Date -Format o)"
        exit 0
    }
    Start-Sleep -Seconds 15
}
exit 1
