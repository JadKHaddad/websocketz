#!/usr/bin/env bash

# https://github.com/snapview/tungstenite-rs/blob/3ffeb33e29824deae10d86f7edff2ed4de22e91b/scripts/autobahn-server.sh

set -euo pipefail
set -x
SOURCE_DIR=$(readlink -f "${BASH_SOURCE[0]}")
SOURCE_DIR=$(dirname "$SOURCE_DIR")
cd "${SOURCE_DIR}/.."

function cleanup() {
    kill -9 ${WSSERVER_PID}
}
trap cleanup TERM EXIT

function test_diff() {
    local expected="autobahn/expected-results.json"
    local actual="autobahn/reports/server/index.json"

    local expected_filtered
    local actual_filtered

    expected_filtered=$(mktemp)
    actual_filtered=$(mktemp)

    jq -S 'del(."websocketz" | .. | .duration?)' "$expected" > "$expected_filtered"
    jq -S 'del(."websocketz" | .. | .duration?)' "$actual" > "$actual_filtered"

    if ! diff -u "$expected_filtered" "$actual_filtered"; then
        echo 'Difference in results. This may be a regression, or you may need to update autobahn/expected-results.json.'
        rm -f "$expected_filtered" "$actual_filtered"
        exit 64
    fi

    rm -f "$expected_filtered" "$actual_filtered"
}

cargo build --release --example autobahn-server
cargo run --release --example autobahn-server & WSSERVER_PID=$!
sleep 3

docker run --rm \
    -v "${PWD}/autobahn/config:/autobahn/config" \
    -v "${PWD}/autobahn/reports:/autobahn/reports" \
    --network host \
    crossbario/autobahn-testsuite \
    wstest -m fuzzingclient -s 'autobahn/config/fuzzingclient.json'

test_diff
