[CmdletBinding()]
param (
    [Parameter(Position = 0)]
    [ValidateSet("up", "down", "bench", "audit", "race", "ui", "check", "help")]
    [string]$Command = "help"
)

switch ($Command) {
    "up" {
        Write-Host ">>> Starting Orcrustrator Arena with Docker Compose..." -ForegroundColor Cyan
        docker compose -f compose/docker-compose.yml up --build -d
        Write-Host ">>> Arena online! UI available at http://localhost:3000" -ForegroundColor Green
    }
    "down" {
        Write-Host ">>> Stopping Orcrustrator Arena containers..." -ForegroundColor Yellow
        docker compose -f compose/docker-compose.yml down -v
    }
    "bench" {
        Write-Host ">>> Executing k6 constant-arrival-rate benchmark harness..." -ForegroundColor Cyan
        docker run --rm -i --network compose_arena-net loadimpact/k6 run - < bench/load-test.js
    }
    "audit" {
        Write-Host ">>> Running Consistency Auditor across all microservices..." -ForegroundColor Cyan
        Invoke-RestMethod -Uri "http://localhost:8090/audit" | ConvertTo-Json -Depth 5
    }
    "race" {
        Write-Host ">>> Firing test race against all three gateways..." -ForegroundColor Cyan
        $naive = Measure-Command { $n = Invoke-RestMethod -Uri "http://localhost:8001/patients/p1/summary" }
        $hand = Measure-Command { $h = Invoke-RestMethod -Uri "http://localhost:8002/patients/p1/summary" }
        $orc = Measure-Command { $o = Invoke-RestMethod -Uri "http://localhost:8003/patients/p1/summary" }
        Write-Host "Naive Gateway: $($naive.TotalMilliseconds) ms" -ForegroundColor Red
        Write-Host "Hand-Rolled Gateway: $($hand.TotalMilliseconds) ms" -ForegroundColor Blue
        Write-Host "Orcrustrator Gateway: $($orc.TotalMilliseconds) ms" -ForegroundColor Green
    }
    "ui" {
        Write-Host ">>> Starting Arena Mission Control UI locally..." -ForegroundColor Cyan
        Set-Location ui
        npm run dev
    }
    "check" {
        Write-Host ">>> Checking Cargo Workspace..." -ForegroundColor Cyan
        & "$env:USERPROFILE\.cargo\bin\cargo.exe" check --workspace
    }
    Default {
        Write-Host "Orcrustrator Arena CLI Commands:" -ForegroundColor Yellow
        Write-Host "  .\run.ps1 up      - Start all containers"
        Write-Host "  .\run.ps1 down    - Stop all containers"
        Write-Host "  .\run.ps1 bench   - Run k6 load test"
        Write-Host "  .\run.ps1 audit   - Query Consistency Auditor"
        Write-Host "  .\run.ps1 race    - Run single side-by-side race"
        Write-Host "  .\run.ps1 ui      - Launch local UI dev server"
        Write-Host "  .\run.ps1 check   - Cargo syntax and DAG check"
    }
}
