#!/usr/bin/env bash
# SPEKTR Uninstaller - Linux & macOS

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BINARY_NAME="spektr"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}   SPEKTR Uninstaller${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Try to find spektr binary
INSTALL_PATH=$(which $BINARY_NAME 2>/dev/null || echo "")

if [ -z "$INSTALL_PATH" ]; then
    # Check default location
    if [ -f "$DEFAULT_INSTALL_DIR/$BINARY_NAME" ]; then
        INSTALL_PATH="$DEFAULT_INSTALL_DIR/$BINARY_NAME"
    else
        echo -e "${YELLOW}⚠️  SPEKTR not found in PATH or default location${NC}"
        echo -e "${YELLOW}   Default location: $DEFAULT_INSTALL_DIR${NC}"
        echo ""
        read -p "Enter custom installation path (or press Enter to skip): " CUSTOM_PATH
        
        if [ -n "$CUSTOM_PATH" ] && [ -f "$CUSTOM_PATH" ]; then
            INSTALL_PATH="$CUSTOM_PATH"
        else
            echo -e "${RED}❌ SPEKTR binary not found. Exiting.${NC}"
            exit 1
        fi
    fi
fi

echo -e "${BLUE}📍 Found: ${INSTALL_PATH}${NC}"
echo ""
echo -e "${YELLOW}⚠️  This will remove SPEKTR from your system.${NC}"
read -p "Continue? [y/N]: " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${BLUE}👋 Uninstall cancelled.${NC}"
    exit 0
fi

# Remove binary
echo -e "${BLUE}🗑️  Removing binary...${NC}"
rm -f "$INSTALL_PATH"

if [ ! -f "$INSTALL_PATH" ]; then
    echo -e "${GREEN}✓ Binary removed${NC}"
else
    echo -e "${RED}❌ Failed to remove binary (may need sudo)${NC}"
    exit 1
fi

# Offer to clean PATH
echo ""
echo -e "${YELLOW}💡 Optional: Remove PATH entry from shell config?${NC}"
read -p "Clean PATH? [y/N]: " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    INSTALL_DIR=$(dirname "$INSTALL_PATH")
    
    # Check common shell configs
    for RC in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.config/fish/config.fish"; do
        if [ -f "$RC" ]; then
            if grep -q "$INSTALL_DIR" "$RC"; then
                echo -e "${BLUE}🔧 Cleaning $RC...${NC}"
                # Create backup
                cp "$RC" "${RC}.backup"
                # Remove SPEKTR PATH entry
                sed -i.tmp '/# SPEKTR/d' "$RC"
                sed -i.tmp "\|$INSTALL_DIR|d" "$RC"
                rm -f "${RC}.tmp"
                echo -e "${GREEN}✓ Cleaned (backup: ${RC}.backup)${NC}"
            fi
        fi
    done
fi

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ SPEKTR uninstalled successfully!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}👋 Thanks for using SPEKTR!${NC}"
