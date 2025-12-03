param(
    [int]$Port = 18080,
    [string]$Path = "/"
)

if (-not $Path.StartsWith("/")) {
    $Path = "/$Path"
}

$uri = "http://127.0.0.1:$Port$Path"
try {
    $resp = Invoke-WebRequest $uri -UseBasicParsing -TimeoutSec 3
    Write-Host "Health check OK ($($resp.StatusCode)) => $uri"
    exit 0
} catch {
    Write-Error ("Health check failed for {0}: {1}" -f $uri, $_)
    exit 1
}
