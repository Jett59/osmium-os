#!/bin/sh

set +x
set -e

cat > build/test.txt << EOF
This is a test file!
EOF

cd build
tar -cf initial_ramdisk.tar test.txt
cd ..
