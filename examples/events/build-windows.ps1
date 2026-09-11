# Run from Visual Studio Developer PowerShell with the C++ tools and SDK loaded.
#
# Card STD-EVENT. The Windows arm of the embedded readiness shim — WSAPoll and a
# loopback endpoint pair — is the one this repository's standing gate does not
# run (it runs the POSIX arm, sanitized and byte-compared, on Linux). This script
# is that arm's runner, on the `examples/threads/build-windows.ps1` shape.
#
# `-Run` additionally compares the program's stdout with the transcript
# examples/events/README.md publishes, which is the same comparison the Linux
# test performs against the reference execution.
[CmdletBinding()]
param([switch]$Run)

$ErrorActionPreference = 'Stop'
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    throw 'Open Visual Studio Developer PowerShell with the C++ tools installed, then run this script again.'
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Install Rust with its MSVC toolchain and put cargo on PATH.'
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$outputDir = Join-Path $repoRoot 'target/event-examples'
$manifest = Join-Path $repoRoot 'Cargo.toml'
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

# The expected trace, read out of the README so this script and that page cannot
# drift apart. The first fenced block after the "```\nQCORE_EVENT_OPEN" marker is
# the published transcript.
$readme = Get-Content -Raw (Join-Path $PSScriptRoot 'README.md')
$expected = [regex]::Match($readme, '(?ms)^```\r?\n(QCORE_EVENT_OPEN.*?)^```').Groups[1].Value
$expected = ($expected -split "`r?`n" | Where-Object { $_ -ne '' })
if ($expected.Count -eq 0) {
    throw 'examples/events/README.md publishes no QCORE_EVENT transcript to compare against.'
}

# `write_interest` is deliberately absent: it is the check-only arm and emits no
# C at all, so there is nothing here to build. `quirin check` reports it.
foreach ($name in @('self_pipe')) {
    $sourcePath = Join-Path $PSScriptRoot "$name.qcore"
    $cPath = Join-Path $outputDir "$name.c"
    $objectPath = Join-Path $outputDir "$name.obj"
    $exePath = Join-Path $outputDir "$name.exe"

    Write-Host "Building $name"
    # `-p qcorec` names the package, which is unchanged; the binary it builds is
    # `quirin`. Selecting by package keeps this working from a checkout without
    # depending on the installed command name.
    & cargo run --quiet --manifest-path $manifest -p qcorec -- emit $sourcePath -o $cPath
    if ($LASTEXITCODE -ne 0) { throw "Quirin emission failed for $name." }

    # No ws2_32.lib on this command line on purpose: the emitted shim links it
    # through `#pragma comment`, so the emitted program stays self-contained on
    # this arm exactly as it is on the other two.
    & cl.exe /nologo /std:c11 /utf-8 /W4 /WX $cPath "/Fo$objectPath" "/Fe:$exePath"
    if ($LASTEXITCODE -ne 0) { throw "MSVC compilation failed for $name." }

    if ($Run) {
        Write-Host "Running $name"
        $actual = & $exePath
        # A non-zero exit is not only a crash: it is also how the emitted program
        # reports that the ready set WSAPoll returned disagreed with the checked
        # declaration. Both are failures here, and deliberately so.
        if ($LASTEXITCODE -ne 0) { throw "$name exited with code $LASTEXITCODE." }
        $actual = @($actual)
        if ($actual.Count -ne $expected.Count) {
            throw "$name wrote $($actual.Count) line(s); the README publishes $($expected.Count)."
        }
        for ($index = 0; $index -lt $expected.Count; $index++) {
            if ($actual[$index] -ne $expected[$index]) {
                throw "$name line $($index + 1): got '$($actual[$index])', README publishes '$($expected[$index])'."
            }
        }
        Write-Host "$name reproduced the published trace on the Windows arm."
    }
}

Write-Host "Executables and generated C: $outputDir"
