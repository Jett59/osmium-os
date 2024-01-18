#!/bin/sh

# Detect the OS
OS="$(uname)"

# Use the appropriate sed command based on the OS
if [ "$OS" = "Darwin" ]; then
  # macOS
  sed -i '' 's/x86_64/aarch64/g' .config
  sed -i '' 's/"rust-analyzer.cargo.target": "x86_64-unknown-linux-gnu"/"rust-analyzer.cargo.target": "aarch64-unknown-linux-gnu"/g' kernel.code-workspace
else
  # Linux and others
  sed 's/x86_64/aarch64/g' -i .config
  sed 's/"rust-analyzer.cargo.target": "x86_64-unknown-linux-gnu"/"rust-analyzer.cargo.target": "aarch64-unknown-linux-gnu"/g' -i kernel.code-workspace
fi
