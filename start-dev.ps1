# VibeHub - Development Server
# Usage: .\start-dev.ps1

Write-Host ""
Write-Host "========================================"
Write-Host "  VibeHub - Dev Server"
Write-Host "========================================"
Write-Host ""

# Step 1: Add Cargo to PATH
Write-Host "[1/3] Setting up Rust environment..." -ForegroundColor Yellow
$cargoPath = "$env:USERPROFILE\.cargo\bin"

if (Test-Path $cargoPath) {
    $env:Path = "$cargoPath;$env:Path"
    Write-Host "      OK: Cargo PATH added" -ForegroundColor Green
} else {
    Write-Host "      ERROR: Cargo not found!" -ForegroundColor Red
    Write-Host "      Please install Rust: https://rustup.rs/" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

# Step 2: Verify Cargo
Write-Host ""
Write-Host "[2/3] Verifying Cargo..." -ForegroundColor Yellow
try {
    $version = & cargo --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "      OK: $version" -ForegroundColor Green
    } else {
        throw "Cargo failed"
    }
} catch {
    Write-Host "      ERROR: Cargo verification failed!" -ForegroundColor Red
    Write-Host "      Error: $_" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

# Step 3: Start Tauri dev server
Write-Host ""
Write-Host "[3/3] Starting Tauri dev server..." -ForegroundColor Yellow
Write-Host "      (First run may take a few minutes to compile Rust code)" -ForegroundColor Gray
Write-Host ""

npm run tauri dev

Write-Host ""
Write-Host "Application closed." -ForegroundColor Yellow
