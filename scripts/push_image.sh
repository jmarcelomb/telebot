#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

version="$("$SCRIPT_DIR/get_version.sh")"

docker push jmarcelomb/telebot:latest
docker push "jmarcelomb/telebot:$version"
