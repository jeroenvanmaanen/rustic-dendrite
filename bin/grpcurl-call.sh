#!/bin/bash

BIN="$(cd "$(dirname "$0")" ; pwd)"
PROJECT="$(dirname "${BIN}")"

source "${BIN}/verbose.sh"

HOST='host.docker.internal'
if [[ ".$1" = '.--host' ]]
then
  HOST="$2"
  shift 2
fi

VARIABLE='message'
VALUE='Tonic'
case "$1" in
--greet)
  PROTO="grpc_example.proto"
  PORT='8181'
  URL='grpc_example.GreeterService/Greet'
  shift
  ;;
--proxy)
  PROTO="grpc_example.proto"
  PORT='3000'
  URL='grpc_example.GreeterService/Greet'
  shift
  ;;
--hello)
  PROTO="hello_world.proto"
  PORT='50051'
  URL='hello_world.Greeter/SayHello'
  VARIABLE='name'
  shift
  ;;
*)
  exit 1
  ;;
esac

if [[ -n "$1" ]]
then
  VALUE="$1"
  shift
fi

PAYLOAD="$(echo "{'${VARIABLE}':'${VALUE}'}" | tr \'\" \"\')"

docker run --rm -v "${PROJECT}:${PROJECT}" -w "${PROJECT}" -ti \
  fullstorydev/grpcurl -plaintext -import-path "${PROJECT}/proto" -proto "${PROTO}" \
    -d "${PAYLOAD}" "${HOST}:${PORT}" "${URL}"
