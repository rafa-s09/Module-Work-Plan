# Installs the mwp CLI from this checkout and, if a folder is given, sets up MWP in it.
# Usage: .\src\install.ps1 [project-dir]
# If scripts are blocked: powershell -ExecutionPolicy Bypass -File .\src\install.ps1 [project-dir]
param([string]$Project)

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "cargo not found. Install Rust first: https://rustup.rs"
    exit 1
}

# --force reinstalls, so template and skill changes reach the binary.
cargo install --path "$PSScriptRoot\client" --locked --force
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$root = if ($env:CARGO_INSTALL_ROOT) { $env:CARGO_INSTALL_ROOT } elseif ($env:CARGO_HOME) { $env:CARGO_HOME } else { "$HOME\.cargo" }
$bin = Join-Path $root "bin"
if ($Project) {
    & (Join-Path $bin "mwp.exe") init $Project
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if (($env:PATH -split ';') -notcontains $bin) {
    Write-Host "Add $bin to your PATH to run mwp from anywhere."
}
