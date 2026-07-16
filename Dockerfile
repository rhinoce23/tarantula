FROM rust:bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    g++ \
    clang \
    libclang-dev \
    llvm-dev \
    lld \
    cmake \
    libssl-dev \
    protobuf-compiler \
    libprotobuf-dev \
    libabsl-dev \
    libs2-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /tarantula
#COPY . /tarantula
#RUN cargo build --locked

EXPOSE 8080 8090
#CMD ["cargo", "run"]
