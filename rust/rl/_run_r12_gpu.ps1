
$ErrorActionPreference = "Stop"
Set-Location C:\Users\neri\code\playmat-bt

Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "python.exe" -and $_.CommandLine -like "*infer_server.py*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Seconds 1

$srv = Start-Process -FilePath python -ArgumentList "-u","rust\rl\infer_server.py","--dir","rust\rl\runs\wavec-r11-5m-sem-oracle","--port","18400","--amp","1" -PassThru -RedirectStandardOutput "rust\rl\infer_r11_5m.log" -RedirectStandardError "rust\rl\infer_r11_5m.err"
try {
    Start-Sleep -Seconds 6
    $NET = "tcp://127.0.0.1:18400"

    Write-Host "=== 1. TACTICS-3: wavec-r11-5m-sem-oracle (iters=512, extra_turns=3, gumbel_root=1, par=4) ==="
    & .\rust\target\release\tactics.exe --suite archbench\tactics-3 `
      --entrants "cfg:policy=ismcts+net=$NET+iters=512+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+samples=4+par=4+width=16+consider=12+extra_turns=3"

    Write-Host "=== 2. START WAVEC-R12-GPU-5M: 1200 MULTI-TURN (extra_turns=2) ULTRA-DEEP (iters=512, par=4) 5M GPU GAMES ==="
    $OUT = "rust\rl\data\wavec-r12-gpu-5m"
    if (Test-Path $OUT) { Remove-Item -Recurse -Force $OUT }
    $LEARNER = "cfg:policy=ismcts+net=$NET+iters=512+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+samples=8+par=4+width=16+consider=12+mulligan_net=1+extra_turns=2+budget=8000+playout_cap=400+temp=200+temp_turn=4"
    & .\rust\target\release\selfplay.exe `
      --out $OUT `
      --suite archbench\constructed-2 `
      --repo C:\Users\neri\code\playmat-bt `
      --learner $LEARNER `
      --opponents "self*7;host@s6*1" `
      --games 1200 `
      --jobs 8 `
      --keep 350 `
      --fast ismcts:64 `
      --seed 970000 `
      --tag 12
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Get-Content "rust\rl\infer_r11_5m.log" -ErrorAction SilentlyContinue | Select-Object -Last 10
} finally {
    if ($srv -and -not $srv.HasExited) { Stop-Process -Id $srv.Id -Force -ErrorAction SilentlyContinue }
}

Set-Location C:\Users\neri\code\playmat-bt\rust\rl
Get-Content C:\Users\neri\code\playmat-bt\.env | ForEach-Object {
    if ($_ -match "^([^#=]+)=(.*)$") {
        [Environment]::SetEnvironmentVariable($matches[1].Trim(), $matches[2].Trim().Trim([char]34, [char]39), "Process")
    }
}
Write-Host "COMPACT_AND_UPLOAD_WAVEC_R12_GPU_5M $(Get-Date -Format o)"
python hf_archive.py --data data\wavec-r12-gpu-5m --compact
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "WAVEC_R12_GPU_5M_DONE $(Get-Date -Format o)"

