# installed by herdr
# managed by herdr; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDR_INTEGRATION_ID=codex
# HERDR_INTEGRATION_VERSION=9

param([string]$Action = "")

$expectedEvents = @{
    session = @("SessionStart")
    working = @("UserPromptSubmit", "PreToolUse", "PostToolUse")
    blocked = @("PermissionRequest")
    idle = @("Stop")
    metadata = @("SessionStart", "UserPromptSubmit", "Stop")
}

if (-not $expectedEvents.ContainsKey($Action)) { exit 0 }
if ($env:HERDR_ENV -ne "1") { exit 0 }
if ([string]::IsNullOrWhiteSpace($env:HERDR_PANE_ID)) { exit 0 }

$inputText = [Console]::In.ReadToEnd()
try {
    $payload = if ([string]::IsNullOrWhiteSpace($inputText)) { $null } else { $inputText | ConvertFrom-Json }
} catch {
    exit 0
}

$hookEventName = [string]$payload.hook_event_name
if ($expectedEvents[$Action] -notcontains $hookEventName) { exit 0 }

$sessionId = [string]$payload.session_id
if ([string]::IsNullOrWhiteSpace($sessionId)) { exit 0 }
if ([string]::IsNullOrWhiteSpace([string]$payload.transcript_path)) { exit 0 }
if (-not [string]::IsNullOrWhiteSpace($env:CODEX_THREAD_ID) -and $env:CODEX_THREAD_ID -ne $sessionId) { exit 0 }

$seq = [DateTime]::UtcNow.Ticks
$herdr = if ([string]::IsNullOrWhiteSpace($env:HERDR_BIN_PATH)) { "herdr" } else { $env:HERDR_BIN_PATH }

function Invoke-Herdr([string[]]$Arguments) {
    try {
        & $herdr @Arguments 2>$null | Out-Null
    } catch {
    }
}

function Read-AppServerResponse($Process, [int]$Id, [DateTime]$Deadline) {
    while ([DateTime]::UtcNow -lt $Deadline) {
        $remaining = [int][Math]::Max(1, ($Deadline - [DateTime]::UtcNow).TotalMilliseconds)
        try {
            $lineTask = $Process.StandardOutput.ReadLineAsync()
            if (-not $lineTask.Wait($remaining)) { return $null }
            $line = $lineTask.Result
            if ($null -eq $line) { return $null }
            $message = $line | ConvertFrom-Json
            if ($message.id -eq $Id) { return $message }
        } catch {
        }
    }
    return $null
}

function Read-CodexThreadTitle([string]$ThreadId) {
    $codex = $env:HERDR_CODEX_BIN_PATH
    if ([string]::IsNullOrWhiteSpace($codex)) {
        try {
            $codex = (Get-Command codex -ErrorAction Stop).Source
        } catch {
            return $null
        }
    }

    $process = $null
    try {
        $startInfo = New-Object System.Diagnostics.ProcessStartInfo
        $startInfo.FileName = $codex
        $startInfo.Arguments = "app-server --stdio"
        $startInfo.UseShellExecute = $false
        $startInfo.CreateNoWindow = $true
        $startInfo.RedirectStandardInput = $true
        $startInfo.RedirectStandardOutput = $true
        $startInfo.RedirectStandardError = $true
        $process = New-Object System.Diagnostics.Process
        $process.StartInfo = $startInfo
        if (-not $process.Start()) { return $null }

        $deadline = [DateTime]::UtcNow.AddMilliseconds(2500)
        $initialize = @{
            id = 1
            method = "initialize"
            params = @{ clientInfo = @{ name = "herdr"; title = "Herdr"; version = "0.9" } }
        } | ConvertTo-Json -Compress -Depth 5
        $process.StandardInput.WriteLine($initialize)
        $process.StandardInput.Flush()
        if ($null -eq (Read-AppServerResponse $process 1 $deadline)) { return $null }

        $initialized = @{ method = "initialized"; params = @{} } | ConvertTo-Json -Compress -Depth 3
        $read = @{
            id = 2
            method = "thread/read"
            params = @{ threadId = $ThreadId; includeTurns = $false }
        } | ConvertTo-Json -Compress -Depth 4
        $process.StandardInput.WriteLine($initialized)
        $process.StandardInput.WriteLine($read)
        $process.StandardInput.Flush()
        $response = Read-AppServerResponse $process 2 $deadline
        if ($null -eq $response) { return $null }
        if (-not [string]::IsNullOrWhiteSpace([string]$response.result.thread.name)) {
            return [string]$response.result.thread.name
        }
        return [string]$response.result.thread.preview
    } catch {
        return $null
    } finally {
        if ($null -ne $process -and -not $process.HasExited) {
            try { $process.Kill() } catch {}
        }
        if ($null -ne $process) { $process.Dispose() }
    }
}

function Normalize-Title($Value) {
    if ($Value -isnot [string]) { return $null }
    $title = $Value -replace '^\s*(?:\[Image #\d+\]\s*)+', ''
    $title = ($title -replace '\s+', ' ').Trim()
    if ([string]::IsNullOrWhiteSpace($title)) { return $null }
    if ($title.Length -gt 80) { $title = $title.Substring(0, 77).TrimEnd() + "..." }
    return $title
}

$commonArgs = @(
    $env:HERDR_PANE_ID,
    "--source", "herdr:codex",
    "--agent", "codex",
    "--seq", "$seq",
    "--agent-session-id", "$sessionId"
)

if ($Action -eq "session") {
    $args = @("pane", "report-agent-session") + $commonArgs
    if ($hookEventName -eq "SessionStart" -and -not [string]::IsNullOrWhiteSpace([string]$payload.source)) {
        $args += @("--session-start-source", "$($payload.source)")
    }
    Invoke-Herdr $args
} elseif ($Action -eq "metadata") {
    $title = Normalize-Title (Read-CodexThreadTitle $sessionId)
    if ($null -eq $title -and $hookEventName -eq "UserPromptSubmit") {
        $title = Normalize-Title $payload.prompt
    }
    if ($null -ne $title) {
        Invoke-Herdr @(
            "pane", "report-metadata", $env:HERDR_PANE_ID,
            "--source", "herdr:codex",
            "--agent", "codex",
            "--seq", "$seq",
            "--title", $title
        )
    }
} else {
    Invoke-Herdr (@("pane", "report-agent") + $commonArgs + @("--state", $Action))
}
