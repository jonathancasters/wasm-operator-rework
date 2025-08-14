# Use the official amd64 Rust image to ensure our build environment matches the target.
FROM --platform=linux/amd64 rust:1.88.0 AS builder

# Install the musl C toolchain
RUN apt-get update && apt-get install -y musl-tools

# Install the musl target for static linking
RUN rustup target add x86_64-unknown-linux-musl

# Set the working directory
WORKDIR /usr/src/app

# Copy the parent operator's source code
COPY ./parent /usr/src/app/parent

# Build the parent operator as a static binary
RUN cd /usr/src/app/parent && cargo build --release --target x86_64-unknown-linux-musl

# --- Final Image ---
# Use a slim Debian image for the final container
FROM debian:buster-slim

# Create a directory for the Wasm modules
RUN mkdir -p /app/wasm

# Copy the compiled Wasm guest modules from the central build artifact directory
COPY ./benchmark/build/*.wasm /app/wasm/

# Copy the compiled, statically linked parent operator binary from the builder stage
COPY --from=builder /usr/src/app/parent/target/x86_64-unknown-linux-musl/release/parent /usr/local/bin/parent

# Set the command to run the parent operator
CMD ["parent"]