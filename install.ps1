# SPEKTR Installer - Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/jcyrus/spektr/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

# Configuration
$REPO = "jcyrus/spektr"
$BINARY_NAME = "spektr.exe"
$INSTALL_DIR = "$env:USERPROFILE\.spektr\bin"

function Write-ColorOutput($ForegroundColor, $Message) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    Write-Output $Message
    $host.UI.RawUI.ForegroundColor = $fc
}

function Get-LatestVersion {
    Write-ColorOutput Cyan "🔍 Fetching latest version..."
    
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
        $version = $response.tag_name -replace '^v', ''
        
        Write-ColorOutput Green "✓ Latest version: v$version"
        return $version
    }
    catch {
        Write-ColorOutput Red "❌ Failed to fetch latest version"
        exit 1
    }
}

function Get-Architecture {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default {
            Write-ColorOutput Red "❌ Unsupported architecture: $arch"
            exit 1
        }
    }
}

function Download-Binary($version, $arch) {
    Write-ColorOutput Cyan "📥 Downloading SPEKTR..."
    
    $filename = "spektr-$version-$arch-pc-windows-msvc.zip"
    $url = "https://github.com/$REPO/releases/download/v$version/$filename"
    
    $tmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    $zipPath = Join-Path $tmpDir $filename
    
    try {
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
        Write-ColorOutput Green "✓ Downloaded"
    }
    catch {
        Write-ColorOutput Red "❌ Download failed: $url"
        Write-ColorOutput Yellow "💡 This might be the first release. Check: https://github.com/$REPO/releases"
        exit 1
    }
    
    Write-ColorOutput Cyan "📦 Extracting..."
    Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force
    
    return $tmpDir
}

function Install-Binary($tmpDir) {
    Write-ColorOutput Cyan "🔧 Installing to $INSTALL_DIR..."
    
    # Create install directory
    if (-not (Test-Path $INSTALL_DIR)) {
        New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
    }
    
    # Move binary
    $binaryPath = Join-Path $tmpDir $BINARY_NAME
    $targetPath = Join-Path $INSTALL_DIR $BINARY_NAME
    
    if (Test-Path $targetPath) {
        Remove-Item $targetPath -Force
    }
    
    Move-Item -Path $binaryPath -Destination $targetPath -Force
    
    Write-ColorOutput Green "✓ Installed successfully"
    
    # Cleanup
    Remove-Item $tmpDir -Recurse -Force
}

function Update-Path {
    $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    
    if ($userPath -notlike "*$INSTALL_DIR*") {
        Write-ColorOutput Cyan "🔧 Adding to PATH..."
        
        $newPath = "$userPath;$INSTALL_DIR"
        [System.Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        
        # Update current session
        $env:Path = "$env:Path;$INSTALL_DIR"
        
        Write-ColorOutput Yellow "⚠️  PATH updated. You may need to restart your terminal."
    }
}

function Verify-Installation {
    $binaryPath = Join-Path $INSTALL_DIR $BINARY_NAME
    
    if (Test-Path $binaryPath) {
        Write-ColorOutput Green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        Write-ColorOutput Green "✅ SPEKTR installed successfully!"
        Write-ColorOutput Green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        Write-Output ""
        Write-ColorOutput Cyan "📍 Location: $binaryPath"
        Write-Output ""
        Write-ColorOutput Cyan "🚀 Quick Start:"
        Write-ColorOutput Yellow "   spektr                    # Scan current directory"
        Write-ColorOutput Yellow "   spektr C:\Projects        # Scan specific path"
        Write-ColorOutput Yellow "   spektr --help             # Show all options"
        Write-Output ""
        Write-ColorOutput Yellow "💡 If 'spektr' is not recognized, restart your terminal."
    }
    else {
        Write-ColorOutput Red "❌ Installation verification failed"
        exit 1
    }
}

# Main installation flow
function Main {
    Write-ColorOutput Cyan "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-ColorOutput Cyan "   SPEKTR Installer"
    Write-ColorOutput Cyan "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    Write-Output ""
    
    $version = Get-LatestVersion
    $arch = Get-Architecture
    $tmpDir = Download-Binary $version $arch
    Install-Binary $tmpDir
    Update-Path
    Verify-Installation
}

Main
