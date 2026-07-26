param(
  [Parameter(Mandatory=$true)][string]$Executable,
  [Parameter(Mandatory=$true)][string]$ReadyEvidenceJson,
  [string]$OutputDir = "target/w7-gate",
  [int]$StartupThresholdMs = 2000,
  [int]$IdleMemoryThresholdMb = 180
)
$ErrorActionPreference = "Stop"
$exe = (Resolve-Path -LiteralPath $Executable).Path
$evidencePath = (Resolve-Path -LiteralPath $ReadyEvidenceJson).Path
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$evidence = Get-Content -Raw -LiteralPath $evidencePath | ConvertFrom-Json
if ([string]::IsNullOrWhiteSpace($evidence.readySelector) -or @($evidence.startupMs).Count -lt 5 -or -not $evidence.processId) {
  throw "W6 embedded-WDIO evidence must provide readySelector, at least five selector-ready startupMs samples, and the currently ready processId"
}
$binaryHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $exe).Hash
if ($evidence.binarySha256 -and $evidence.binarySha256 -ne $binaryHash) { throw "ready evidence binary SHA256 does not match the measured executable" }
$rootProcess = Get-Process -Id ([int]$evidence.processId) -ErrorAction Stop

function Median([double[]]$values) {
  $sorted = @($values | Sort-Object); $n = $sorted.Count
  if ($n % 2) { return [double]$sorted[[int]($n/2)] }
  return ([double]$sorted[$n/2-1] + [double]$sorted[$n/2]) / 2
}
function ProcessTree([int]$rootId) {
  $all = @(Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath)
  $ids = [Collections.Generic.HashSet[int]]::new(); [void]$ids.Add($rootId)
  do { $before=$ids.Count; foreach($p in $all){ if($ids.Contains([int]$p.ParentProcessId)){[void]$ids.Add([int]$p.ProcessId)} } } while($ids.Count -gt $before)
  return @($all | Where-Object { $ids.Contains([int]$_.ProcessId) -and ($_.ProcessId -eq $rootId -or $_.Name -match '^(careercraft.*|msedgewebview2)\.exe$') })
}

$memorySamples = @(); $stable = $false; $tree = @()
for($attempt=0; $attempt -lt 60; $attempt++) {
  if ($rootProcess.HasExited) { break }
  $tree = ProcessTree $rootProcess.Id
  $totalBytes = 0
  foreach($node in $tree){ $p=Get-Process -Id $node.ProcessId -ErrorAction SilentlyContinue; if($p){$totalBytes += $p.WorkingSet64} }
  $memorySamples += [math]::Round($totalBytes/1MB,2)
  if($memorySamples.Count -ge 5){$last=@($memorySamples | Select-Object -Last 5);$mid=Median $last;$spread=($last|Measure-Object -Maximum).Maximum-($last|Measure-Object -Minimum).Minimum;if($spread -le [math]::Max(2,$mid*0.05)){$stable=$true;break}}
  Start-Sleep -Milliseconds 500
}
$startup = [double[]]@($evidence.startupMs)
$startupP50 = [math]::Round((Median $startup),2)
$startupSorted=@($startup|Sort-Object);$p95Index=[math]::Min($startupSorted.Count-1,[math]::Ceiling($startupSorted.Count*0.95)-1);$startupP95=[double]$startupSorted[$p95Index]
$stableTail=[double[]]@($memorySamples|Select-Object -Last ([math]::Min(5,$memorySamples.Count)))
$idleMedian = if($stableTail.Count){[math]::Round((Median $stableTail),2)}else{0}
$peak = if($memorySamples.Count){[double](($memorySamples|Measure-Object -Maximum).Maximum)}else{0}
$webviewNode=$tree|Where-Object Name -eq 'msedgewebview2.exe'|Select-Object -First 1
$webviewVersion=if($webviewNode -and $webviewNode.ExecutablePath){(Get-Item -LiteralPath $webviewNode.ExecutablePath).VersionInfo.FileVersion}else{"unknown"}
$cases=@(
 [ordered]@{name="desktop_selector_ready_p50";value=$startupP50;threshold=$StartupThresholdMs;unit="ms";passed=($startupP50-le $StartupThresholdMs)},
 [ordered]@{name="desktop_selector_ready_p95";value=$startupP95;threshold=$StartupThresholdMs;unit="ms";passed=($startupP95-le $StartupThresholdMs)},
 [ordered]@{name="desktop_process_tree_idle_median";value=$idleMedian;threshold=$IdleMemoryThresholdMb;unit="MiB";passed=($stable-and $idleMedian-le $IdleMemoryThresholdMb)}
)
$passed=-not($cases|Where-Object{-not $_.passed})
$report=[ordered]@{suite="w7-desktop-performance";passed=$passed;timestampUtc=[DateTime]::UtcNow.ToString("o");binarySha256=$binaryHash;executable=$exe;os=[Environment]::OSVersion.VersionString;webView2Version=$webviewVersion;readySelector=$evidence.readySelector;startupSamplesMs=$startup;startupP50Ms=$startupP50;startupP95Ms=$startupP95;processTree=@($tree);memorySamplesMiB=$memorySamples;memoryStable=$stable;memoryPeakMiB=$peak;memoryIdleMedianMiB=$idleMedian;cases=$cases}
$report|ConvertTo-Json -Depth 8|Set-Content -Encoding UTF8 (Join-Path $OutputDir "w7-desktop.json")
$failures=@($cases|Where-Object{-not $_.passed}).Count;$xml=New-Object Text.StringBuilder;[void]$xml.Append("<testsuite name=`"w7-desktop-performance`" tests=`"$($cases.Count)`" failures=`"$failures`">");foreach($case in $cases){[void]$xml.Append("<testcase name=`"$($case.name)`"><system-out>value=$($case.value) $($case.unit); threshold=$($case.threshold)</system-out>");if(-not $case.passed){[void]$xml.Append("<failure message=`"threshold exceeded or sampling unstable`">value=$($case.value), threshold=$($case.threshold)</failure>")};[void]$xml.Append("</testcase>")};[void]$xml.Append("</testsuite>");$xml.ToString()|Set-Content -Encoding UTF8 (Join-Path $OutputDir "w7-desktop.junit.xml")
$report|ConvertTo-Json -Depth 8
if(-not $passed){exit 1}
