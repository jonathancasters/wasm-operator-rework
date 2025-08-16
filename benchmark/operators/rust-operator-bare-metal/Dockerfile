# Stage 1: Build the Rust binary
FROM rust:latest AS builder

WORKDIR /usr/src/rust-operator-bare-metal

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src

# Build the binary
RUN cargo build --release

# Stage 2: Create the final image
FROM gcr.io/distroless/cc-debian12

# Copy the binary from the builder stage
COPY --from=builder /usr/src/rust-operator-bare-metal/target/release/rust-operator-bare-metal .

# Set the entrypoint
ENTRYPOINT ["/rust-operator-bare-metal"]