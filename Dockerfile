# Included for reference and testing that targeting ARM64 / MUSL is not entirely broken, as it is not tested in CI

FROM rust:alpine AS builder
WORKDIR /tmp/build

ENV RUSTFLAGS="-D warnings"

# print some version information
RUN rustc -V
RUN cargo -V
RUN uname -a

# add rustfmt component
RUN rustup component add rustfmt

# install packages
RUN apk update && apk add opam alpine-sdk zlib-dev xz m4 z3 gmp-dev

# setup OCaml
RUN opam init --disable-sandboxing --bare -y
RUN opam switch create 4.11.2+musl+static+flambda

# install sail
RUN eval `opam env` && opam install --assume-depexts -y sail=0.15 gmp

# fetch crates index
RUN cd /tmp && cargo init --lib empty && cd empty && cargo add itoa && cargo build

# build and document rust dependencies by creating empty crates
RUN cargo init --lib borealis && \
    cargo init --lib sail && \
    cargo init --lib common
COPY Cargo.lock .
COPY Cargo.toml .
COPY borealis/Cargo.toml borealis/
COPY sail/Cargo.toml sail/
COPY common/Cargo.toml common/
RUN eval `opam env` && \
    cargo build --release --all-targets && \
    cargo test --release --no-run && \
    cargo doc --release

# copy full source
COPY . .
RUN touch borealis/src/lib.rs sail/src/lib.rs common/src/lib.rs

# check formatting
RUN cargo fmt --all -- --check

# build and run tests
RUN eval `opam env` && cargo test --release --no-fail-fast

# build borealis
RUN eval `opam env` && cargo build --release --all-targets

# build docs
RUN eval `opam env` && cargo doc --release
RUN echo '<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0; URL=/borealis/borealis" /></head></html>' > target/doc/index.html

FROM scratch
COPY --from=builder /tmp/build/target/doc /doc
COPY --from=builder /tmp/build/target/release/borealis .
ENTRYPOINT [ "./borealis", "--force", "--log", "trace", "-i", "model/sail.json", "genc", "-o", "target" ]
