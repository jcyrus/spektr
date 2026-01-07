#!/usr/bin/env bash
# SPEKTR Installer - Linux & macOS
# Usage: curl -sL https://raw.githubusercontent.com/jcyrus/spektr/main/install.sh | bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO="jcyrus/spektr"
INSTALL_DIR="${SPEKTR_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="spektr"

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS="linux" ;;
        Darwin*)    OS="macos" ;;
        *)          
            echo -e "${RED}❌ Unsupported OS: $(uname -s)${NC}"
            exit 1
            ;;
    esac
}

# Detect Architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64)     ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *)          
            echo -e "${RED}❌ Unsupported architecture: $(uname -m)${NC}"
            exit 1
            ;;
    esac
}

# Get latest release version
get_latest_version() {
    echo -e "${BLUE}🔍 Fetching latest version...${NC}"
    VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    
    if [ -z "$VERSION" ]; then
        echo -e "${RED}❌ Failed to fetch latest version${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Latest version: v${VERSION}${NC}"
}

# Construct download URL
construct_url() {
    if [ "$OS" = "macos" ]; then
        if [ "$ARCH" = "aarch64" ]; then
            TARGET="aarch64-apple-darwin"
        else
            TARGET="x86_64-apple-darwin"
        fi
    else
        if [ "$ARCH" = "aarch64" ]; then
            TARGET="aarch64-unknown-linux-gnu"
        else
            TARGET="x86_64-unknown-linux-gnu"
        fi
    fi
    
    FILENAME="spektr-${VERSION}-${TARGET}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${FILENAME}"
}

# Download and extract
download_binary() {
    echo -e "${BLUE}📥 Downloading SPEKTR...${NC}"
    
    TMP_DIR=$(mktemp -d)
    cd "$TMP_DIR"
    
    if ! curl -sLO "$DOWNLOAD_URL"; then
        echo -e "${RED}❌ Download failed: ${DOWNLOAD_URL}${NC}"
        echo -e "${YELLOW}💡 This might be the first release. Check: https://github.com/${REPO}/releases${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Downloaded${NC}"
    echo -e "${BLUE}📦 Extracting...${NC}"
    
    tar -xzf "$FILENAME"
    
    chmod +x "$BINARY_NAME"
}

# Install binary
install_binary() {
    echo -e "${BLUE}🔧 Installing to ${INSTALL_DIR}...${NC}"
    
    mkdir -p "$INSTALL_DIR"
    mv "$BINARY_NAME" "$INSTALL_DIR/"
    
    echo -e "${GREEN}✓ Installed successfully${NC}"
    
    # Cleanup
    cd - > /dev/null
    rm -rf "$TMP_DIR"
}

# Add to PATH if needed
update_path() {
    # Check if already in PATH
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        return
    fi
    
    SHELL_RC=""
    case "$SHELL" in
        */bash)
            if [ -f "$HOME/.bashrc" ]; then
                SHELL_RC="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                SHELL_RC="$HOME/.bash_profile"
            fi
            ;;
        */zsh)
            SHELL_RC="$HOME/.zshrc"
            ;;
        */fish)
            SHELL_RC="$HOME/.config/fish/config.fish"
            ;;
    esac
    
    if [ -n "$SHELL_RC" ]; then
        echo "" >> "$SHELL_RC"
        echo "# SPEKTR" >> "$SHELL_RC"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$SHELL_RC"
        
        echo -e "${YELLOW}⚠️  Added $INSTALL_DIR to PATH in $SHELL_RC${NC}"
        echo -e "${YELLOW}   Run: source $SHELL_RC${NC}"
        echo -e "${YELLOW}   Or restart your terminal${NC}"
    else
        echo -e "${YELLOW}⚠️  Manually add to PATH:${NC}"
        echo -e "${YELLOW}   export PATH=\"\$PATH:$INSTALL_DIR\"${NC}"
    fi
}

# Verify installation
verify_install() {
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN}✅ SPEKTR installed successfully!${NC}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        echo -e "${BLUE}📍 Location:${NC} $INSTALL_DIR/$BINARY_NAME"
        echo ""
        echo -e "${BLUE}🚀 Quick Start:${NC}"
        echo -e "   ${YELLOW}spektr${NC}                    # Scan current directory"
        echo -e "   ${YELLOW}spektr /path/to/projects${NC} # Scan specific path"
        echo -e "   ${YELLOW}spektr --help${NC}            # Show all options"
    else
        echo -e "${RED}❌ Installation verification failed${NC}"
        exit 1
    fi
}

# Main installation flow
main() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}   SPEKTR Installer${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    
    detect_os
    detect_arch
    get_latest_version
    construct_url
    download_binary
    install_binary
    update_path
    verify_install
}

main
