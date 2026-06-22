$ErrorActionPreference = 'Stop'

Write-Host "Verifying configuration contract..."

if (-not (Test-Path "config.json")) {
    Write-Error "config.json not found in the root directory!"
    exit 1
}

$config = Get-Content "config.json" -Raw | ConvertFrom-Json
$port = if ($config.port) { $config.port } else { 3001 }

Write-Host "Config loaded. Expecting backend to bind to port $port."

# Start the backend process
$process = Start-Process -FilePath "cargo" -ArgumentList "run --bin backend -- --config ../config.json" -WorkingDirectory "./backend" -PassThru

Write-Host "Waiting for backend to start (Timeout: 120s)..."

$maxAttempts = 120
$attempt = 0
$success = $false

while ($attempt -lt $maxAttempts) {
    Start-Sleep -Seconds 1
    $attempt++
    
    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/" -UseBasicParsing -ErrorAction Stop
        if ($response.Content -match "rsahp backend running") {
            $success = $true
            break
        }
    } catch {
        # Server not up yet, ignore error
    }
}

# Cleanup
if (-not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
}

if ($success) {
    Write-Host "Configuration verified successfully! Backend booted and bound to port $port." -ForegroundColor Green
    exit 0
} else {
    Write-Error "Verification failed! Backend did not start or failed to bind to port $port within 120 seconds."
    exit 1
}
