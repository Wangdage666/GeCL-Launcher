param([ValidateSet('Build', 'Bundle', 'Check', 'Test', 'Run', 'Debug', 'Agent')][string]$Action = 'Build')
$ErrorActionPreference = 'Stop'
if ($Action -eq 'Agent') {
	Write-Host 'Agent 已合并到正式版，使用统一 Build 入口。'
	$Action = 'Build'
}
$projectRoot = Split-Path $PSScriptRoot -Parent
Push-Location $projectRoot
try {
	$localTools = Join-Path $projectRoot '.tools'
	if (Test-Path (Join-Path $localTools 'cargo/bin/cargo.exe')) {
		$env:CARGO_HOME = Join-Path $localTools 'cargo'
		$env:RUSTUP_HOME = Join-Path $localTools 'rustup'
		$env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
	}
	$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
	if (!(Test-Path $vswhere)) { throw '缺少 Visual Studio C++ Build Tools。' }
	$vsPath = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
	if (!$vsPath) { throw '未找到已安装的 C++ x64 编译工具。' }
	$vc = Get-ChildItem (Join-Path $vsPath 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
	$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10'
	$sdk = Get-ChildItem (Join-Path $sdkRoot 'Lib') -Directory | Sort-Object Name -Descending | Select-Object -First 1
	if (!$sdk -or !(Test-Path (Join-Path $sdk.FullName 'um/x64/kernel32.lib'))) { throw '缺少 Windows SDK x64 链接库。' }
	$env:PATH = "$($vc.FullName)\bin\Hostx64\x64;$sdkRoot\bin\$($sdk.Name)\x64;$env:PATH"
	$env:LIB = "$($vc.FullName)\lib\x64;$($sdk.FullName)\um\x64;$($sdk.FullName)\ucrt\x64"
	$env:INCLUDE = "$($vc.FullName)\include;$sdkRoot\Include\$($sdk.Name)\ucrt;$sdkRoot\Include\$($sdk.Name)\shared;$sdkRoot\Include\$($sdk.Name)\um"
	$env:SQLX_OFFLINE = 'true'
	if ($Action -in @('Build', 'Bundle', 'Agent')) {
		$pluginDirectory = Join-Path $projectRoot 'target/nsis-plugin'
		New-Item -ItemType Directory -Path $pluginDirectory -Force | Out-Null
		$savedLibraryPath = $env:LIB
		try {
			$env:LIB = "$($vc.FullName)\lib\x86;$($sdk.FullName)\um\x86;$($sdk.FullName)\ucrt\x86"
			& (Join-Path $vc.FullName 'bin/Hostx64/x86/cl.exe') /nologo /LD /O1 /MT /utf-8 /D_CRT_SECURE_NO_WARNINGS apps/app/nsis/gecl-ui.cpp "/Fo:$pluginDirectory/gecl-ui.obj" "/Fe:$pluginDirectory/gecl_ui.dll" /link "/DEF:apps/app/nsis/gecl-ui.def" "/IMPLIB:$pluginDirectory/gecl_ui.lib" user32.lib gdi32.lib comctl32.lib
			if ($LASTEXITCODE -ne 0) { throw '安装界面插件构建失败。' }
		} finally { $env:LIB = $savedLibraryPath }
	}
	if (!(Test-Path 'packages/app-lib/.env')) { Copy-Item 'packages/app-lib/.env.prod' 'packages/app-lib/.env' }
	if ($Action -ne 'Test') {
		& python scripts/gecl-agent-catalog.py
		if ($LASTEXITCODE -ne 0) { throw '命令清单生成失败。' }
		New-Item -ItemType Directory -Path 'target/agent-tools' -Force | Out-Null
		& rustc --edition 2024 -O apps/app/src/agent_cli.rs -o target/agent-tools/gecl-agent-x86_64-pc-windows-msvc.exe
		if ($LASTEXITCODE -ne 0) { throw 'Agent 命令工具构建失败。' }
	}
	if ($Action -eq 'Test') {
		& cargo test -p theseus --lib --locked
	} elseif ($Action -eq 'Check') {
		& cargo check -p theseus_gui --locked
	} elseif ($Action -eq 'Debug') {
		& pnpm --filter '@modrinth/app' exec tauri build --debug --no-bundle --config (Join-Path $PSScriptRoot 'tauri.debug.conf.json')
	} elseif ($Action -eq 'Run') {
		& pnpm --filter '@modrinth/app' exec tauri dev --features agent-api
	} elseif ($Action -eq 'Bundle') {
		& pnpm --filter '@modrinth/app' exec tauri bundle --bundles nsis
	} else {
		& pnpm --filter '@modrinth/app' exec tauri build --bundles nsis
	}
	if ($LASTEXITCODE -ne 0) { throw "构建或检查失败，退出码 $LASTEXITCODE。" }
	if ($Action -in @('Build', 'Bundle', 'Agent')) {
		$outputDirectory = Join-Path $projectRoot 'build-gecl'
		New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
		if (Test-Path (Join-Path $outputDirectory 'gecl.exe')) {
			$backupDirectory = Join-Path $projectRoot '.build-backups'
			New-Item -ItemType Directory -Path $backupDirectory -Force | Out-Null
			Copy-Item -LiteralPath (Join-Path $outputDirectory 'gecl.exe') -Destination (Join-Path $backupDirectory ('gecl-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.exe'))
		}
		Copy-Item -LiteralPath 'target/release/gecl.exe' -Destination (Join-Path $outputDirectory 'gecl.exe') -Force
		$releaseVersion = (Get-Content -LiteralPath 'apps/app-frontend/package.json' -Raw | ConvertFrom-Json).version
		$installerName = 'gecl_' + $releaseVersion + '_x64-setup.exe'
		Copy-Item -LiteralPath (Join-Path 'target/release/bundle/nsis' $installerName) -Destination $outputDirectory -Force
		Copy-Item -LiteralPath 'target/agent-tools/gecl-agent-x86_64-pc-windows-msvc.exe' -Destination (Join-Path $outputDirectory 'gecl-agent.exe') -Force
		Copy-Item -LiteralPath 'docs/AGENT_COMMANDS.json', 'docs/AGENT_COMMANDS.md', 'docs/AGENT_API_SCHEMA.json', 'docs/AGENT_TYPES.json' -Destination $outputDirectory -Force
		Copy-Item -LiteralPath 'apps/app/LICENSE' -Destination (Join-Path $outputDirectory 'LICENSE.txt') -Force
		Copy-Item -LiteralPath 'GECL_FORK.md' -Destination $outputDirectory -Force
	}
} finally { Pop-Location }
