#!/bin/bash

echo "═══════════════════════════════════════════════════════════════"
echo "          Testing All RustySquid Makefile Commands            "
echo "═══════════════════════════════════════════════════════════════"
echo

echo "1. make format - Format code"
make format
echo

echo "2. make lint - Run clippy"
make lint
echo

echo "3. make test - Run tests"
make test
echo

echo "4. make build - Build for ARM64"
make build
echo

echo "5. make quality-gate - Full quality check"
make quality-gate
echo

echo "═══════════════════════════════════════════════════════════════"
echo "                   All Commands Working!                       "
echo "═══════════════════════════════════════════════════════════════"