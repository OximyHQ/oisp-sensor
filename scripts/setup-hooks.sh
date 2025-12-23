#!/bin/sh
#
# Setup git hooks for oisp-sensor development
#
# Usage: ./scripts/setup-hooks.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Setting up git hooks for oisp-sensor..."

# Configure git to use our hooks directory
git config core.hooksPath .githooks

# Make hooks executable
chmod +x "$REPO_ROOT/.githooks/"*

echo ""
echo "Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs cargo fmt, clippy, and tests"
echo ""
echo "To skip hooks temporarily (not recommended):"
echo "  git commit --no-verify"
echo ""

