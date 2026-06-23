$ErrorActionPreference = "Continue"

Write-Host "Building rsahp..."
cargo build --bin backend
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to build backend."
    exit $LASTEXITCODE
}
cargo build --bin frontend
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to build frontend."
    exit $LASTEXITCODE
}

$backendProc = Get-Process -Name "backend" -ErrorAction SilentlyContinue
if (-not $backendProc) {
    Write-Host "Starting backend..."
    Start-Process -FilePath ".\target\debug\backend.exe" -RedirectStandardOutput "backend.log" -RedirectStandardError "backend.log" -WindowStyle Hidden
} else {
    Write-Host "Backend is already running."
}

$frontendProc = Get-Process -Name "frontend" -ErrorAction SilentlyContinue
if (-not $frontendProc) {
    Write-Host "Starting frontend..."
    Start-Process -FilePath ".\target\debug\frontend.exe" -RedirectStandardOutput "frontend.log" -RedirectStandardError "frontend.log" -WindowStyle Hidden
} else {
    Write-Host "Frontend is already running."
}

Write-Host "rsahp started."
