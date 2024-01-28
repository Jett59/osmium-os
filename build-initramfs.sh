#!/bin/sh

set +x
set -e

cat > build/test.txt << EOF
This is a test file!
EOF

cd build
tar -cf initramfs.tar test.txt
cd ..
