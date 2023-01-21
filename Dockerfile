FROM debian:11.6
RUN apt-get update && \
    apt-get install -y curl wget make xorriso clang lld grub-efi binutils \
    && apt-get clean
RUN wget -qO - https://sh.rustup.rs | RUSTUP_HOME=/opt/rust CARGO_HOME=/opt/rust sh -s -- --no-modify-path -y
ENV PATH="/opt/rust/bin:${PATH}"
RUN rustup default nightly
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu

ADD . /osmium
WORKDIR /osmium
RUN cargo test
# RUN ./build.sh
# RUN rm -rf /osmium/*
