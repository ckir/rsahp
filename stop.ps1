Write-Host "Stopping rsahp..."

$backendProc = Get-Process -Name "backend" -ErrorAction SilentlyContinue
if ($backendProc) {
    Write-Host "Stopping backend..."
    $backendProc | Stop-Process -Force
} else {
    Write-Host "Backend is not running."
}

$frontendProc = Get-Process -Name "frontend" -ErrorAction SilentlyContinue
if ($frontendProc) {
    Write-Host "Stopping frontend..."
    $frontendProc | Stop-Process -Force
} else {
    Write-Host "Frontend is not running."
}

Write-Host "rsahp stopped."
