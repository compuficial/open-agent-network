# Installs the Open Agent Network engine on its own for Windows. Names are not verified by a sign-in service. In PowerShell:
#
#   irm <this url> | iex                                                  install, then set up (start or join a team)
#   & ([scriptblock]::Create((irm <this url>))) <team link>               install, then join that team
#   & ([scriptblock]::Create((irm <this url>))) <team link> --as <name>   the same, without questions (for agents)
#   & ([scriptblock]::Create((irm <this url>))) init <team> --as <name>   install and start a team, without questions
#
# From cmd or another shell, wrap it: powershell -c "<the line>".
# Environment: OAN_BIN_DIR (default ~\.local\bin), OAN_VERSION (default latest), OAN_REPO,
# OAN_DOWNLOAD_BASE (a mirror serving the release files), OAN_NO_PROMPT (never ask anything: updates).
& {
  $ErrorActionPreference = 'Stop'
  $ProgressPreference = 'SilentlyContinue'
  function die($msg) { throw "oan install: $msg" }

  $repo = if ($env:OAN_REPO) { $env:OAN_REPO } else { 'compuficial/open-agent-network' }
  $bin = if ($env:OAN_BIN_DIR) { $env:OAN_BIN_DIR } else { Join-Path $HOME '.local\bin' }
  $version = if ($env:OAN_VERSION) { $env:OAN_VERSION } else { 'latest' }

  $cpu = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
  $arch = switch ($cpu) { 'AMD64' { 'x86_64' } 'ARM64' { 'aarch64' } default { die "unsupported CPU: $cpu" } }
  $target = "$arch-pc-windows-msvc"

  if ($env:OAN_DOWNLOAD_BASE) { $base = $env:OAN_DOWNLOAD_BASE }
  elseif ($version -eq 'latest') { $base = "https://github.com/$repo/releases/latest/download" }
  else { $base = "https://github.com/$repo/releases/download/$version" }

  $tmp = Join-Path ([IO.Path]::GetTempPath()) "oan-$([guid]::NewGuid().ToString('n'))"
  New-Item -ItemType Directory $tmp | Out-Null
  try {
    Write-Host "Downloading oan for $target..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    $zip = Join-Path $tmp 'oan.zip'
    try { Invoke-WebRequest -UseBasicParsing "$base/oan-$target.zip" -OutFile $zip } catch { die "download failed ($base/oan-$target.zip)" }
    try { Invoke-WebRequest -UseBasicParsing "$base/oan-$target.zip.sha256" -OutFile "$zip.sha256" } catch { die 'checksum download failed; not installing' }
    $want = ([IO.File]::ReadAllText("$zip.sha256") -split '\s+')[0].ToLower()
    $file = [IO.File]::OpenRead($zip)
    try { $got = -join ([Security.Cryptography.SHA256]::Create().ComputeHash($file) | ForEach-Object { $_.ToString('x2') }) } finally { $file.Dispose() }
    if (-not $want -or $want -ne $got) { die 'checksum mismatch; not installing' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [IO.Compression.ZipFile]::ExtractToDirectory($zip, $tmp)

    $new = Join-Path $tmp 'oan.exe'
    & $new --version | Out-Null
    if ($LASTEXITCODE) { die "the downloaded binary doesn't run here; not installing" }
    New-Item -ItemType Directory -Force $bin | Out-Null
    $exe = Join-Path $bin 'oan.exe'
    # Windows can't replace a program that is running, but it can rename it: move the old one aside, and delete
    # the ones set aside before once nothing runs them.
    Get-ChildItem $bin -Filter 'oan.*.old' | Remove-Item -Force -ErrorAction SilentlyContinue
    $old = $null
    if (Test-Path $exe) { $old = Join-Path $bin "oan.$(Get-Random).old"; Move-Item $exe $old }
    try { Move-Item (Join-Path $tmp 'oan.exe') $exe } catch { if ($old) { Move-Item $old $exe }; throw }
  } finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
  }
  Write-Host "Installed $exe ($(& $exe --version))"

  $path = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($path -split ';') -notcontains $bin) {
    [Environment]::SetEnvironmentVariable('Path', (@($path -split ';' | Where-Object { $_ }) + $bin) -join ';', 'User')
    $env:Path = "$env:Path;$bin"
    Write-Host "Added $bin to your PATH (open a new terminal to use it there)."
  }

  # Hand over to oan: it asks a few questions (your name, start or join, which agents) when there's someone to ask.
  if ($args.Count -eq 0) {
    if ($env:OAN_NO_PROMPT -or [Console]::IsInputRedirected) { Write-Host ''; Write-Host 'Next, in a terminal: oan'; return }
    & $exe
  } elseif ($args[0] -match '^(-.*|init|join|setup)$') {
    & $exe @args
  } else {
    & $exe join @args
  }
  if ($LASTEXITCODE) { throw "oan exited with code $LASTEXITCODE" }
} @args
