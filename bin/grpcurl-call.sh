#!/bin/bash

BIN="$(cd "$(dirname "$0")" ; pwd)"
PROJECT="$(dirname "${BIN}")"

HOST='host.docker.internal'
if [[ ".$1" = '.--host' ]]
then
  HOST="$2"
  shift 2
fi

case "$1" in
--greet)
  PROTO="grpc_example.proto"
  PORT='8181'
  URL='grpc_example.GreeterService/Greet'
  PAYLOAD='{"message": "Tonic"}'
  ;;
--proxy)
  PROTO="grpc_example.proto"
  PORT='3000'
  URL='grpc_example.GreeterService/Greet'
  PAYLOAD='{"message": "Tonic"}'
  ;;
--hello)
  PROTO="hello_world.proto"
  PORT='50051'
  URL='hello_world.Greeter/SayHello'
  PAYLOAD='{"name": "Tonic"}'
  ;;
*)
  exit 1
  ;;
esac

docker run --rm -v "${HOME}:${HOME}" -w "$(pwd)" -ti \
  fullstorydev/grpcurl -plaintext -import-path "${PROJECT}/proto" -proto "${PROTO}" \
    -d '{"message": "Tonic"}' "${HOST}:${PORT}" "${URL}"
