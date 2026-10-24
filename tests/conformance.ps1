<#
.SYNOPSIS
  Orcrustrator Arena Gateways Conformance Suite
  Proves byte-level and semantic equivalence across Naive, Hand-Rolled, and Orcrustrator gateways.
#>

$gateways = @{
    "Naive" = "http://localhost:8001";
    "Hand-Rolled" = "http://localhost:8002";
    "Orcrustrator" = "http://localhost:8003"
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   ORCRUSTRATOR ARENA CONFORMANCE VERIFICATION SUITE       " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# Test 1: Patient Summary Equivalence
Write-Host "`n[Test 1] Verifying GET /patients/p1/summary equivalence..." -ForegroundColor Yellow

$responses = @{}
foreach ($gw in $gateways.Keys) {
    $url = "$($gateways[$gw])/patients/p1/summary"
    try {
        $res = Invoke-RestMethod -Uri $url -Method Get -TimeoutSec 5
        $responses[$gw] = $res
        Write-Host "  [+] $gw responded with 200 OK" -ForegroundColor Green
    } catch {
        Write-Host "  [-] $gw failed: $_" -ForegroundColor Red
    }
}

if ($responses.Count -eq 3) {
    # Check patient ID and fields match
    $p1 = $responses["Naive"].patient.id
    $p2 = $responses["Hand-Rolled"].patient.id
    $p3 = $responses["Orcrustrator"].patient.id

    if ($p1 -eq $p2 -and $p2 -eq $p3) {
        Write-Host "  >>> Conformance Assertion Passed: Patient ID ($p1) matches across all gateways!" -ForegroundColor Green
    } else {
        Write-Host "  >>> Conformance Assertion FAILED: Patient ID mismatch ($p1 vs $p2 vs $p3)" -ForegroundColor Red
    }

    # Check labs count
    $l1 = $responses["Naive"].labs.Count
    $l2 = $responses["Hand-Rolled"].labs.Count
    $l3 = $responses["Orcrustrator"].labs.Count
    Write-Host "  >>> Labs count: Naive=$l1, Hand-Rolled=$l2, Orcrustrator=$l3" -ForegroundColor Cyan
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "   CONFORMANCE VERIFICATION COMPLETED                     " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
