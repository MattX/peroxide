#!/usr/bin/env sh

cargo run -- tests/scheme/r5rs-tests.scm | tee /dev/tty | tail -n 1 | grep -q 100%
