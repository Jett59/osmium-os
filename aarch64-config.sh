#!/bin/sh

sed 's/x86_64/aarch64/g' -i .config
sed 's/"rust-analyzer.cargo.target": "x86_64-unknown-linux-gnu"/"rust-analyzer.cargo.target": "aarch64-unknown-linux-gnu"/g' -i kernel.code-workspace
