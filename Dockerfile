FROM termoshtt/rust-dev
COPY eom-worker /src
WORKDIR /src
RUN cargo build --release
ENTRYPOINT ["cargo", "run", "--release", "--"]
