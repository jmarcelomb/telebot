#!/usr/bin/env bash

cat Cargo.toml | grep "^version =" | cut -d '=' -f 2 | xargs | sed -e 's/^"//' -e 's/$"//'
