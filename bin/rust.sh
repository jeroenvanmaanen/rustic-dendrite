#!/bin/bash

docker run --rm -ti -v "${HOME}:${HOME}" -w "$(pwd)" -e "USER=${USER}" rust:1.31 "$@"
