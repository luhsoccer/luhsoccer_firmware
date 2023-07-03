FROM rust:latest

LABEL maintainer="max.kaenner@luhbots.de"
LABEL versio="0.0.3"
LABEL description="Custom docker image for ci checks and builds"

RUN apt update -y
RUN apt install -y protobuf-compiler
RUN rustup toolchain add nightly
RUN rustup default nightly
RUN rustup target add thumbv6m-none-eabi thumbv7em-none-eabihf
RUN rustup component add llvm-tools-preview clippy rustfmt
RUN cargo install flip-link cargo-llvm-cov cargo-nextest
