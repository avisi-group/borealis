VERSION 0.7

base-image:
    FROM rust:alpine
    WORKDIR /workdir

    ENV RUSTFLAGS="-D warnings"

    ARG TARGETARCH

    IF [ "$TARGETARCH" = "amd64" ]
        ENV RUST_TARGET=x86_64-unknown-linux-musl
    ELSE IF [ "$TARGETARCH" = "arm64" ]
        ENV RUST_TARGET=aarch64-unknown-linux-musl
    ELSE
        ENV RUST_TARGET=invalid
    END

    RUN rustup component add rustfmt
    RUN apk update && apk add opam alpine-sdk zlib-dev xz m4 z3 gmp-dev clang mold

    # setup OCaml
    RUN opam init --disable-sandboxing --bare -y
    RUN opam switch create 4.14.1+options --packages ocaml-variants.4.14.1+options,ocaml-option-static,ocaml-option-musl,ocaml-option-flambda

    # install sail
    RUN eval `opam env` && opam install --assume-depexts -y sail=0.15 gmp

    RUN cargo install cargo-chef --locked

    SAVE IMAGE --cache-hint --push ghcr.io/avisi-group/borealis/baseimage

chef-prepare:
    FROM +base-image
    COPY . .
    RUN cargo chef prepare --recipe-path recipe.json
    SAVE ARTIFACT recipe.json recipe.json

chef-cook:
    FROM +base-image
    COPY (+chef-prepare/recipe.json) .
    RUN eval `opam env` && cargo chef cook --recipe-path recipe.json --target $RUST_TARGET

build:
    FROM +chef-cook

    # copy full source
    COPY --dir sail common borealis .

    # build borealis
    RUN eval `opam env` && mold -run cargo build --target $RUST_TARGET

    SAVE ARTIFACT target/$RUST_TARGET/debug/borealis borealis

borealis-docs:
    FROM +build
    RUN eval `opam env` && cargo doc --target $RUST_TARGET
    RUN echo '<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0; URL=/borealis/borealis" /></head></html>' > target/$RUST_TARGET/doc/index.html
    SAVE ARTIFACT target/$RUST_TARGET/doc docs

docs:
    FROM scratch
    COPY (+borealis-docs/docs) docs
    CMD [""]
    SAVE IMAGE --push ghcr.io/avisi-group/borealis/docs

docker:
    BUILD +build
    FROM alpine

    RUN apk update && apk add z3

    COPY (+build/borealis) borealis
    ENTRYPOINT ["./borealis"]
    SAVE IMAGE --push ghcr.io/avisi-group/borealis

test:
    BUILD +unit-test
    BUILD --platform=linux/amd64 +e2e-test-archsim

test-chef-cook:
    FROM +base-image
    COPY (+chef-prepare/recipe.json) .
    RUN eval `opam env` && cargo chef cook --recipe-path recipe.json --target $RUST_TARGET --tests

unit-test:
    BUILD +build
    FROM +test-chef-cook

    # copy full source
    COPY . .

    # check formatting
    RUN cargo fmt --all -- --check

    # build and run tests
    RUN eval `opam env` && cargo test --target $RUST_TARGET --no-fail-fast

e2e-test-borealis-genc:
    # must use alpine instead of +docker due to https://github.com/earthly/earthly/issues/2618. reading this comment on 2023-08-23 i do not understand it.
    FROM +docker
    COPY data/arm-v8.5-a.bincode.lz4 arm-v8.5-a.bincode.lz4
    RUN ./borealis sail2genc arm-v8.5-a.bincode.lz4 genc
    SAVE ARTIFACT --force genc AS LOCAL target/genc

e2e-test-gensim:
    BUILD +e2e-test-borealis-genc

    FROM ghcr.io/fmckeogh/gensim:latest
    COPY (+e2e-test-borealis-genc/genc) .
    RUN ./dist/bin/gensim --verbose -a main.genc -t output -s module,arch,decode,disasm,ee_interp,ee_blockjit,jumpinfo,function,makefile -o decode.GenerateDotGraph=1,makefile.libtrace_path=/gensim/support/libtrace/inc,makefile.archsim_path=/gensim/archsim/inc,makefile.Optimise=3,makefile.Debug=1
    SAVE ARTIFACT --force output AS LOCAL target/genc/output
    RUN cd output && make -j$(nproc)
    SAVE ARTIFACT /gensim/build/dist dist
    SAVE ARTIFACT output/arm64.dll arm64.dll

e2e-test-archsim:
    BUILD +e2e-test-gensim

    FROM ghcr.io/fmckeogh/gensim:latest

    RUN apt-get install -yy gcc-aarch64-linux-gnu
    COPY data/fib.S .
    RUN aarch64-linux-gnu-gcc -o fib -nostdlib -static fib.S
    COPY data/mem.S .
    RUN aarch64-linux-gnu-gcc -o mem -nostdlib -static mem.S

    RUN mkdir modules
    COPY (+e2e-test-gensim/arm64.dll) modules

    RUN ./dist/bin/archsim -m aarch64-user -l contiguous -s arm64 --modules ./modules -e ./fib -t -U trace.interp.fib --mode Interpreter
    RUN ./dist/bin/TraceCat trace.interp.fib0
    COPY data/fib.interp.trace .
    RUN bash -c 'diff --strip-trailing-cr -u -w fib.interp.trace <(./dist/bin/TraceCat trace.interp.fib0)'

    RUN ./dist/bin/archsim -m aarch64-user -l contiguous -s arm64 --modules ./modules -e ./fib -t -U trace.jit.fib
    RUN ./dist/bin/TraceCat trace.jit.fib0
    COPY data/fib.jit.trace .
    RUN bash -c 'diff --strip-trailing-cr -u -w fib.jit.trace <(./dist/bin/TraceCat trace.jit.fib0)'

    RUN ./dist/bin/archsim -m aarch64-user -l contiguous -s arm64 --modules ./modules -e ./mem -t -U trace.interp.mem --mode Interpreter
    RUN ./dist/bin/TraceCat trace.interp.mem0
    COPY data/mem.trace .
    RUN bash -c 'diff --strip-trailing-cr -u -w mem.trace <(./dist/bin/TraceCat trace.interp.mem0)'

mcf:
    BUILD +test

    FROM ghcr.io/fmckeogh/gensim:latest

    RUN mkdir modules
    COPY (+e2e-test-gensim/arm64.dll) modules

    COPY data/mcf/mcf_r_base.aarch64-static-64 .

    RUN ./dist/bin/archsim -m aarch64-user -l contiguous -s arm64 --modules ./modules -e ./mcf_r_base.aarch64-static-64 -d -t -U trace --mode Interpreter 2>&1 | tee archsim.mcf.log
    RUN ./dist/bin/TraceCat trace0 2>&1 | tee archsim.mcf.trace
    SAVE ARTIFACT --force archsim.mcf.log AS LOCAL target/archsim.mcf.log
    SAVE ARTIFACT --force archsim.mcf.trace AS LOCAL target/archsim.mcf.trace

bench:
    FROM ubuntu:latest

    COPY (+bench-fib-qemu/qemu.time) .
    COPY (+bench-fib-sail/sail.time) .
    COPY (+bench-fib-archsim/archsim.time) .

    ENV cachebust

    RUN cat qemu.time
    RUN cat sail.time
    RUN cat archsim.time

bench-fib-program:
    FROM ubuntu:latest

    RUN apt-get update
    RUN apt-get install -yy gcc-aarch64-linux-gnu

    COPY data/fib.S .
    RUN sed -i 's/#10/0x08000000/g' fib.S
    RUN aarch64-linux-gnu-gcc -o fib -nostdlib -static fib.S

    RUN aarch64-linux-gnu-objdump -D fib

    SAVE ARTIFACT fib fib

bench-fib-qemu:
    FROM ubuntu:latest

    RUN apt-get update
    RUN apt-get install -yy qemu-user time

    COPY (+bench-fib-program/fib) fib

    RUN { time qemu-aarch64 fib ; } 2> qemu.time

    SAVE ARTIFACT qemu.time qemu.time

bench-fib-sail:
    FROM +base-image

    RUN apk add git

    RUN git clone https://github.com/rems-project/sail-arm.git
    RUN cd sail-arm/arm-v8.5-a && eval `opam env` && make
    RUN cd sail-arm/arm-v8.5-a && eval `opam env` && make aarch64

    COPY (+bench-fib-program/fib) fib

    RUN { time sail-arm/arm-v8.5-a/aarch64 -C cpu.cpu0.RVBAR=0x4000d4 -e fib || true ; } 2> sail.time

    SAVE ARTIFACT sail.time sail.time

bench-fib-archsim:
    FROM ghcr.io/fmckeogh/gensim:latest

    RUN apt-get update
    RUN apt-get install -yy time

    COPY (+bench-fib-program/fib) fib

    RUN mkdir modules
    COPY (+e2e-test-gensim/arm64.dll) modules
    RUN { time ./dist/bin/archsim -m aarch64-user -l contiguous -s arm64 --modules ./modules -e ./fib ; } 2> archsim.time

    SAVE ARTIFACT archsim.time archsim.time
