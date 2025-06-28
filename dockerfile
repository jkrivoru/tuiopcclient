FROM rust:latest

WORKDIR /app
COPY . .

# Install dependencies
RUN apt-get update && apt-get install -y libssl-dev pkg-config

# Build the application
RUN cargo build --release

# Output will be in /app/target/release/