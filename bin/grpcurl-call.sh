#!/bin/bash

BIN="$(cd "$(dirname "$0")" ; pwd)"
PROJECT="$(dirname "${BIN}")"

docker run -v "${HOME}:${HOME}" -w "$(pwd)" -ti \
  fullstorydev/grpcurl -plaintext -import-path "${PROJECT}/proto" -proto hello_world.proto \
    -d '{"name": "Tonic"}' host.docker.internal:50051 hello_world.Greeter/SayHello