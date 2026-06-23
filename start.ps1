$ErrorActionPreference = "Continue"

# Stop existing processes first to release Windows file locks
$backendProc = Get-Process -Name "backend" -ErrorAction SilentlyContinue
if ($backendProc) {
    Write-Host "Stopping existing backend process..."
    Stop-Process -Name "backend" -Force
}

$frontendProc = Get-Process -Name "frontend" -ErrorAction SilentlyContinue
if ($frontendProc) {
    Write-Host "Stopping existing frontend process..."
    Stop-Process -Name "frontend" -Force
}

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

Write-Host "Starting backend..."
if (-not (Test-Path -Path "logs")) {
    New-Item -ItemType Directory -Path "logs" | Out-Null
}
Start-Process -FilePath ".\target\debug\backend.exe" -RedirectStandardOutput "logs\backend_out.log" -RedirectStandardError "logs\backend_err.log" -WindowStyle Hidden

Write-Host "Starting frontend..."
Start-Process -FilePath ".\target\debug\frontend.exe" -RedirectStandardOutput "logs\frontend_out.log" -RedirectStandardError "logs\frontend_err.log" -WindowStyle Hidden

Write-Host "rsahp started successfully."
