# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Run from Visual Studio Developer PowerShell with the C++ tools and SDK loaded.
#
# Card STD-EVENT. The Windows arm of the embedded readiness shim uses WSAPoll
# and a runtime-selected AF_UNIX pathname pair, retaining the loopback endpoint
# pair for older SDKs and systems. The Rust test target runs both paths through
# the shared Windows harness; this script remains the direct example runner, on
# the `examples/threads/build-windows.ps1` shape.
#
# `-Run` displays the explicitly selected system trace on stdout.
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

# `write_interest` is deliberately absent: it is the check-only arm and emits no
# C at all, so there is nothing here to build. `quirin check` reports it.
#
# `wake_payload` is the composed program of card COMPOSE-EVENT-LINES: it writes
# its two source-authored lines on stderr beside the selected system trace on
# stdout, and its emitted C verifies the byte each turn drained against the one
# the checker predicted (a mismatch exits 76).
foreach ($name in @('self_pipe', 'wake_payload')) {
    $sourcePath = Join-Path $PSScriptRoot "$name.qr"
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
        & $exePath
        # A non-zero exit is not only a crash: it is also how the emitted program
        # reports that the ready set WSAPoll returned disagreed with the checked
        # declaration. Both are failures here, and deliberately so.
        if ($LASTEXITCODE -ne 0) { throw "$name exited with code $LASTEXITCODE." }
        Write-Host "$name completed with its selected diagnostic log on the Windows arm."
        # Both streams are this program's: stdout carries the system trace it
        # selected and stderr its own `log` lines. The transcripts published in
        # README.md are the comparison; this runner checks the exit code, which
        # is residual TRACE-STDIO-WIN-RUN's shape on this page too.
    }
}

Write-Host "Executables and generated C: $outputDir"
