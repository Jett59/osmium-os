#!/bin/sh

# This rather long sed command prettifies the output of objdump to make it far easier to read the disassembly
llvm-objdump -C -d -l build/target/osmium | sed -r -e 's/[0-9a-fA-F]{2,}\s*<(.*)>/\1/g' -e 's/\s+/ /g' -e 's/^\s*//g' -e 's/^[0-9a-fA-F]+:\s*(.+)/\1/g' -e 's/^([0-9a-fA-F]{2}\s)+//g' > build/target/disassembly.s