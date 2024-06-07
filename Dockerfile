# Use the official Rust image as a base image
FROM rust:latest

# Install dependencies
RUN apt-get update && \
    apt-get install -y curl

# Download and install solc (Solidity compiler)
RUN curl -L https://github.com/ethereum/solidity/releases/download/v0.8.26/solc-static-linux -o /usr/local/bin/solc && \
    chmod +x /usr/local/bin/solc

# Create a new directory for the application
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . .

ENV ROCKET_ADDRESS=0.0.0.0

# Build the Rust application
RUN cargo build --release

# Expose the port the app runs on
EXPOSE 8000

# Run the compiled binary
CMD ["./target/release/server"]