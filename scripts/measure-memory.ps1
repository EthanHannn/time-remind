param(
  [string]$ProcessName = "time-remind",
  [int]$DurationHours = 0,
  [int]$DurationMinutes = 0,
  [int]$DurationSeconds = 0,
  [int]$IntervalSeconds = 60,
  [string]$OutputPath = "logs/memory-samples.csv"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($DurationHours -lt 0) {
  throw "DurationHours 不能小于 0"
}

if ($DurationMinutes -lt 0) {
  throw "DurationMinutes 不能小于 0"
}

if ($DurationSeconds -lt 0) {
  throw "DurationSeconds 不能小于 0"
}

if (($DurationHours -eq 0) -and ($DurationMinutes -eq 0) -and ($DurationSeconds -eq 0)) {
  throw "DurationHours、DurationMinutes 和 DurationSeconds 不能同时为 0"
}

if ($IntervalSeconds -lt 5) {
  throw "IntervalSeconds 必须大于等于 5"
}

$resolvedOutput = Join-Path $PWD $OutputPath
$outputDir = Split-Path -Parent $resolvedOutput
if (-not (Test-Path $outputDir)) {
  New-Item -ItemType Directory -Path $outputDir | Out-Null
}

$endTime = (Get-Date).AddHours($DurationHours).AddMinutes($DurationMinutes).AddSeconds($DurationSeconds)
$samples = New-Object System.Collections.Generic.List[object]

while ((Get-Date) -lt $endTime) {
  $processes = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
  $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"

  if (-not $processes) {
    $samples.Add([PSCustomObject]@{
      Timestamp = $timestamp
      ProcessId = ""
      WorkingSetMB = ""
      PrivateMemoryMB = ""
      Handles = ""
      CpuSeconds = ""
      Status = "not_found"
    }) | Out-Null
  } else {
    foreach ($process in $processes) {
      $samples.Add([PSCustomObject]@{
        Timestamp = $timestamp
        ProcessId = $process.Id
        WorkingSetMB = [math]::Round($process.WorkingSet64 / 1MB, 2)
        PrivateMemoryMB = [math]::Round($process.PrivateMemorySize64 / 1MB, 2)
        Handles = $process.Handles
        CpuSeconds = [math]::Round($process.CPU, 2)
        Status = "running"
      }) | Out-Null
    }
  }

  Start-Sleep -Seconds $IntervalSeconds
}

$samples | Export-Csv -Path $resolvedOutput -NoTypeInformation -Encoding UTF8

$runningSamples = $samples | Where-Object { $_.Status -eq "running" -and $_.WorkingSetMB -ne "" }
if ($runningSamples.Count -gt 0) {
  $firstSample = $runningSamples[0]
  $lastSample = $runningSamples[$runningSamples.Count - 1]
  $delta = [math]::Round([double]$lastSample.WorkingSetMB - [double]$firstSample.WorkingSetMB, 2)
  Write-Output "Memory sample written to: $resolvedOutput"
  Write-Output "WorkingSetMB first=$($firstSample.WorkingSetMB) last=$($lastSample.WorkingSetMB) delta=$delta"
} else {
  Write-Output "Memory sample written to: $resolvedOutput"
  Write-Output "No running process samples captured for process name '$ProcessName'."
}
