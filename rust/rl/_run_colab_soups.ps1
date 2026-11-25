
$ErrorActionPreference = "Stop"
Set-Location C:\Users\neri\code\playmat-bt\rust\rl
$c = Get-Content colab_soup.py -Raw
$c = $c.Replace("model.write_pmnet(", "model.export(")
Set-Content -Path colab_soup.py -Value $c -NoNewline
$tok = ""
Get-Content C:\Users\neri\code\playmat-bt\.env | ForEach-Object {
    if ($_ -match "^HF_TOKEN=(.*)$") {
        $tok = $matches[1].Trim().Trim([char]34, [char]39)
    }
}
Write-Host "START_COLAB_CPU_SOUPS_V2 $(Get-Date -Format o)"
& C:\Users\neri\.local\bin\colab.exe --auth adc run --timeout 0 --env "HF_TOKEN=$tok" colab_soup.py
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "COLAB_CPU_SOUPS_DONE $(Get-Date -Format o)"

