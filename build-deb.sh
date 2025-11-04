#!/bin/bash
# Script to build DEB package for whisper.cpp

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building DEB package for whisper.cpp..."

# Check if we have debhelper
if ! command -v debuild &> /dev/null; then
    echo "Error: debuild not found. Please install devscripts package:"
    echo "  sudo apt-get install devscripts debhelper"
    exit 1
fi

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf build debian/whisper-cpp debian/.debhelper debian/*.debhelper.log debian/*.substvars
rm -f ../whisper-cpp_*.deb ../whisper-cpp_*.dsc ../whisper-cpp_*.tar.* ../whisper-cpp_*.changes
rm -f ../whisper-cpp_*.build ../whisper-cpp_*.buildinfo

# Build the package
echo "Building package..."
debuild -b -us -uc

echo ""
echo "Build complete! Moving package to project root..."
if ls ../whisper-cpp_*.deb 1> /dev/null 2>&1; then
    mv -v ../whisper-cpp_*.deb ./ 2>/dev/null || true
    echo ""
    echo "Package files in project root:"
    ls -lh whisper-cpp_*.deb 2>/dev/null
    echo ""
    echo "Install with: sudo dpkg -i whisper-cpp_*.deb"
else
    echo "No .deb file found"
fi

