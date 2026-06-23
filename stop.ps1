$ErrorActionPreference = "Continue"

$backendProc = Get-Process -Name "backend" -ErrorAction SilentlyContinue
if ($backendProc) {
    Write-Host "Stopping backend gracefully (may take a moment)..."
    # Send Ctrl+C via external tool or just stop it.
    # Since Windows lacks a native soft-kill for background tasks in standard PS,
    # we use Stop-Process.
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
