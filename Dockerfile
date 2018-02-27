FROM termoshtt/rust-dev
RUN mkdir -p /src && git clone https://github.com/termoshtt/eom-kube /src/eom-kube
WORKDIR /src/eom-kube
RUN cargo build --release
ENTRYPOINT ["cargo", "run", "--release", "--"]

