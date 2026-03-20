param(
    [string]$AuthBaseUrl = "http://localhost:8084",
    [string]$GatewayBaseUrl = "http://localhost:8080"
)

$ErrorActionPreference = "Stop"

function Write-Step($message) {
    Write-Host ""
    Write-Host "==> $message" -ForegroundColor Cyan
}

function Invoke-JsonRequest {
    param(
        [string]$Method,
        [string]$Url,
        [hashtable]$Headers = @{},
        $Body = $null
    )

    $params = @{
        Method      = $Method
        Uri         = $Url
        Headers     = $Headers
        ContentType = "application/json"
        ErrorAction = "Stop"
    }

    if ($null -ne $Body) {
        $params.Body = ($Body | ConvertTo-Json -Depth 10)
    }

    Invoke-RestMethod @params
}

$suffix = Get-Random -Maximum 999999
$email = "integration+$suffix@redeye.local"
$password = "Password123!"
$companyName = "RedEye Smoke $suffix"
$workspaceName = "RedEye Workspace $suffix"
$dummyProviderKey = "dummy-openai-key-$suffix"

Write-Step "Checking service health"
try {
    $authHealth = Invoke-JsonRequest -Method GET -Url "$AuthBaseUrl/health"
} catch {
    Write-Host "Warning: optional check failed for $AuthBaseUrl/health -> $($_.Exception.Message)" -ForegroundColor Yellow
    $authHealth = $null
}
$gatewayHealth = Invoke-JsonRequest -Method GET -Url "$GatewayBaseUrl/health"
try {
    $gatewayReady = Invoke-JsonRequest -Method GET -Url "$GatewayBaseUrl/ready"
} catch {
    Write-Host "Warning: optional check failed for $GatewayBaseUrl/ready -> $($_.Exception.Message)" -ForegroundColor Yellow
    $gatewayReady = @{ status = "unknown" }
}

if ($null -ne $authHealth -and $authHealth.status -ne "ok") { throw "Auth health check failed" }
if ($gatewayHealth.status -ne "ok") { throw "Gateway health check failed" }

Write-Step "Signing up tenant user"
$signup = Invoke-JsonRequest -Method POST -Url "$AuthBaseUrl/v1/auth/signup" -Body @{
    email = $email
    password = $password
    company_name = $companyName
}

if (-not $signup.token) { throw "Signup did not return a token" }
$authHeaders = @{ Authorization = "Bearer $($signup.token)" }

Write-Step "Completing onboarding without mandatory real provider key"
$onboard = Invoke-JsonRequest -Method POST -Url "$AuthBaseUrl/v1/auth/onboard" -Headers $authHeaders -Body @{
    workspace_name = $workspaceName
    openai_api_key = $dummyProviderKey
}

if (-not $onboard.redeye_api_key) { throw "Onboarding did not return a RedEye API key" }

Write-Step "Updating provider credentials with dummy integration key"
$providerStatus = Invoke-JsonRequest -Method POST -Url "$AuthBaseUrl/v1/auth/providers" -Headers $authHeaders -Body @{
    openai_api_key = $dummyProviderKey
}

if (-not $providerStatus.openai_configured) { throw "OpenAI provider was not marked configured" }

Write-Step "Saving tenant routes"
try {
    $updatedRoutes = Invoke-JsonRequest -Method PUT -Url "$GatewayBaseUrl/v1/admin/routes" -Headers $authHeaders -Body @{
        routes = @(
            @{
                provider = "openai"
                model = "gpt-4o-mini"
                is_default = $true
            },
            @{
                provider = "gemini"
                model = "gemini-1.5-pro"
                is_default = $false
            }
        )
    }
} catch {
    throw "Tenant route API unavailable. Restart the latest gateway build with .\dev.bat and try again. Original error: $($_.Exception.Message)"
}

if ($updatedRoutes.routes.Count -lt 2) { throw "Route update did not persist expected entries" }

Write-Step "Dry-running tenant route resolution"
$dryRun = Invoke-JsonRequest -Method POST -Url "$GatewayBaseUrl/v1/admin/routes/dry-run" -Headers $authHeaders -Body @{
    model = "gpt-4o-mini"
}

if ($dryRun.resolved_provider -ne "openai") { throw "Dry-run did not resolve the expected provider" }

Write-Step "Reading audit trail"
$audit = Invoke-JsonRequest -Method GET -Url "$GatewayBaseUrl/v1/admin/audit" -Headers $authHeaders

if ($audit.entries.Count -lt 1) { throw "Audit log should contain at least one entry" }

Write-Step "Reading gateway metrics"
$metrics = Invoke-JsonRequest -Method GET -Url "$GatewayBaseUrl/v1/admin/metrics" -Headers $authHeaders

Write-Host ""
Write-Host "Integration smoke test passed." -ForegroundColor Green
Write-Host "Tenant email: $email"
Write-Host "Workspace: $workspaceName"
Write-Host "Gateway ready status: $($gatewayReady.status)"
Write-Host "Resolved dry-run provider: $($dryRun.resolved_provider)"
Write-Host "Metric total requests: $($metrics.total_requests)"
