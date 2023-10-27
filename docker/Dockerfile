FROM rust:1-bullseye

# Install dependencies
RUN apt-get update \
 && apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev \
 && apt-get clean \
 && rm -rf /var/lib/apt/lists/*

RUN rustup install nightly
