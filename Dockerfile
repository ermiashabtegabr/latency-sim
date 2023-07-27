FROM rust:1.67 as builder

RUN cargo new --bin latency-sim
WORKDIR /latency-sim

# Cache dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

# Copy source code
COPY ./src ./src

RUN cargo build --release

# Copy build artifact from build stage
FROM debian:bullseye-slim 

# Install necessary packages to run netem
RUN USER=root apt-get update && \
    apt-get install -y iproute2

# Make sudo dummy replacement, so we don't weaken docker security
RUN echo "#!/bin/bash\n\$@" > /usr/bin/sudo
RUN chmod +x /usr/bin/sudo

# Working directory
RUN mkdir latency-sim

COPY --from=builder /latency-sim/target/release/latency-sim /latency-sim
WORKDIR /latency-sim

# Set start command
CMD ["sudo", "./latency-sim"]
