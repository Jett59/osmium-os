#!/bin/sh

set +x
set -e

export RUSTFLAGS='-C link-arg=--script=linker.ld -C relocation-model=static'

cargo build --target $ARCH-unknown-none $PROFILE_OPTION

mkdir -p build

# Update when adding a new binary 

copy_artifact() {
    mkdir -p build/$1
    cp target/$ARCH-unknown-none/$PROFILE/$2 build/$1/$2
    llvm-strip build/$1/$2
}

copy_artifact services startup
