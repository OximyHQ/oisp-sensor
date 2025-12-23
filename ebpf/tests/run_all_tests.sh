#!/bin/bash
# Chapter 2 - Master Test Runner
# Runs all tests for eBPF SSL capture verification
#
# Usage:
#   1. In terminal 1, start the capture:
#      ./target/release/oisp-ebpf-capture
#
#   2. In terminal 2, run the tests:
#      ./tests/run_all_tests.sh
#
# Or in Docker:
#   docker run --rm --privileged --pid=host --network=host oisp-ebpf-capture \
#     /bin/bash -c "/app/target/release/oisp-ebpf-capture & sleep 2 && /app/tests/run_all_tests.sh"

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "##################################################"
echo "#  OISP eBPF SSL Capture - Chapter 2 Test Suite  #"
echo "##################################################"
echo ""
echo "Starting tests at $(date)"
echo "PID: $$"
echo ""

# Phase 2.1: Basic verification
echo "=========================================="
echo "  PHASE 2.1: Basic Verification"
echo "=========================================="
bash "$SCRIPT_DIR/test_basic.sh"
echo ""
sleep 1

# Phase 2.2: AI Provider testing  
echo "=========================================="
echo "  PHASE 2.2: AI Provider Testing"
echo "=========================================="
bash "$SCRIPT_DIR/test_ai_providers.sh"
echo ""
sleep 1

# Phase 2.3: Multi-process testing
echo "=========================================="
echo "  PHASE 2.3: Multi-Process Testing"
echo "=========================================="

echo ""
echo "--- Python Tests ---"
if command -v python3 &> /dev/null; then
    python3 "$SCRIPT_DIR/test_multiprocess.py"
else
    echo "Python3 not found, skipping Python tests"
fi
echo ""

echo "--- Node.js Tests ---"
if command -v node &> /dev/null; then
    node "$SCRIPT_DIR/test_multiprocess.js"
else
    echo "Node.js not found, skipping Node.js tests"
fi
echo ""

echo "##################################################"
echo "#  All Chapter 2 Tests Completed!               #"
echo "##################################################"
echo ""
echo "Test completed at $(date)"
echo ""
echo "Next steps:"
echo "  1. Review the eBPF capture output above"
echo "  2. Verify all events were captured correctly"
echo "  3. Check TODO.md and mark completed items"
echo "  4. Proceed to Chapter 3: Integration with Pipeline"
echo ""

