#!/bin/bash

docker build -t mcap-viewer-builder -f docker/Dockerfile docker

docker run -v ${PWD}/assets:/workspace/assets -v ${PWD}/crates:/workspace/crates -v ${PWD}/src:/workspace/src -v ${PWD}/Cargo.lock:/workspace/Cargo.lock -v ${PWD}/Cargo.toml:/workspace/Cargo.toml -w /workspace --name mcap-viewer-builder mcap-viewer-builder cargo +nightly build --release

docker cp mcap-viewer-builder:/workspace/target/release/mcap-viewer ./mcap-viewer

docker container rm mcap-viewer-builder
