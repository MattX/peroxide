#!/usr/bin/env sh

mkfifo /tmp/peroxide-output
cargo run -- tests/scheme/r5rs-tests.scm | tee /tmp/peroxide-output &
tail -n 1 /tmp/peroxide-output | grep -q 100%
