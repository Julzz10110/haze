# PowerShell script to aggregate logs from multiple HAZE nodes

param(
    [string]$LogDir = ".",
    [string]$Output = "aggregated.log"
)

Write-Host "HAZE Log Aggregator" -ForegroundColor Blue
Write-Host "==================="
Write-Host "Source directory: $LogDir"
Write-Host "Output file: $Output"
Write-Host ""

# Clear output file
if (Test-Path $Output) {
    Remove-Item $Output -Force
}

# Find all node log files
$LogFiles = Get-ChildItem -Path $LogDir -Filter "node*.log" -File | Sort-Object Name

if ($LogFiles.Count -eq 0) {
    Write-Host "No node log files found in $LogDir" -ForegroundColor Yellow
    exit 1
}

Write-Host "Found log files:"
foreach ($log in $LogFiles) {
    Write-Host "  - $($log.Name)"
}
Write-Host ""

# Aggregate logs
foreach ($log in $LogFiles) {
    $nodeName = [System.IO.Path]::GetFileNameWithoutExtension($log.Name)
    Add-Content -Path $Output -Value "=== $nodeName ==="
    Get-Content $log.FullName | Add-Content -Path $Output
    Add-Content -Path $Output -Value ""
}

Write-Host "Logs aggregated to: $Output" -ForegroundColor Green
Write-Host ""
Write-Host "Useful commands:"
Write-Host "  # Extract metrics:"
Write-Host "  Select-String -Path $Output -Pattern 'Metrics:'"
Write-Host ""
Write-Host "  # Extract block events:"
Write-Host "  Select-String -Path $Output -Pattern 'Block created:'"
Write-Host ""
Write-Host "  # Extract errors:"
Write-Host "  Select-String -Path $Output -Pattern 'error|failed|warn' -CaseSensitive:`$false"
Write-Host ""
Write-Host "  # Extract sync events:"
Write-Host "  Select-String -Path $Output -Pattern 'sync|peer' -CaseSensitive:`$false"
