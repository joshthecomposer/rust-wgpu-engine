Get-CimInstance Win32_Process |
    Where-Object { $_.Name -match '^python' -and $_.CommandLine -match 'http\.server' } |
    ForEach-Object {
        Write-Host "Stopping PID $($_.ProcessId)"
        Stop-Process -Id $_.ProcessId -Force
    }
