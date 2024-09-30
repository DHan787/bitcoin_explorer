FROM rust:latest
WORKDIR /usr/src/bitcoin-ingestion
COPY . .
RUN cargo install --path .
CMD ["bitcoin-ingestion"]
