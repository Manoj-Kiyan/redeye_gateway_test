Write-Host ""
Write-Host "==> Rust auth check" -ForegroundColor Cyan
cargo check -p redeye_auth
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "==> Rust gateway test suite" -ForegroundColor Cyan
cargo test -p redeye_gateway
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "==> Dashboard lint" -ForegroundColor Cyan
Push-Location redeye_dashboard
npm run lint
if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }

Write-Host ""
Write-Host "==> Dashboard build" -ForegroundColor Cyan
npm run build
$exitCode = $LASTEXITCODE
Pop-Location
if ($exitCode -ne 0) { exit $exitCode }

Write-Host ""
Write-Host "CI verification passed." -ForegroundColor Green
