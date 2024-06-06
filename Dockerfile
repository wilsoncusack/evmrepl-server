# Use the official Rust image as a base image
FROM rust:latest

# Install solc (Solidity compiler)
RUN apt-get update && apt-get install -y solc

# Create a new directory for the application
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . .

# Build the Rust application
RUN cargo build --release

# Expose the port the app runs on
EXPOSE 8000

# Run the compiled binary
CMD ["./target/release/server"]