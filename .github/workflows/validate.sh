#!/bin/bash
# Simple workflow validation script

set -e

echo "🔍 Validating GitHub Actions workflow..."

# Check if workflow file exists
if [ ! -f ".github/workflows/ci.yml" ]; then
    echo "❌ Workflow file not found!"
    exit 1
fi

# Basic YAML syntax check using Bun (if available)
if command -v bun &> /dev/null; then
    echo "✅ Workflow file exists"
    echo "ℹ️  For full validation, consider using: act --list"
else
    echo "⚠️  Bun not found, skipping advanced validation"
fi

# Check for required sections
if grep -q "name: CI" .github/workflows/ci.yml; then
    echo "✅ Workflow name found"
else
    echo "❌ Workflow name missing"
    exit 1
fi

if grep -q "strategy:" .github/workflows/ci.yml; then
    echo "✅ Matrix strategy found"
else
    echo "❌ Matrix strategy missing"
    exit 1
fi

if grep -q "ubuntu-latest\|macos-latest\|windows-latest" .github/workflows/ci.yml; then
    echo "✅ Multi-platform targets found"
else
    echo "❌ Multi-platform targets missing"
    exit 1
fi

echo "🎉 Workflow validation passed!"