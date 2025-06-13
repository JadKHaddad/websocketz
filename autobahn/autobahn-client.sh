#!/usr/bin/env bash

set -euo pipefail
set -x
SOURCE_DIR=$(readlink -f "${BASH_SOURCE[0]}")
SOURCE_DIR=$(dirname "$SOURCE_DIR")
cd "${SOURCE_DIR}/.."

CONTAINER_NAME=fuzzingserver
function cleanup() {
    docker container stop "${CONTAINER_NAME}"
}
trap cleanup TERM EXIT

function test_diff() {
    local expected="autobahn/expected-results.json"
    local actual="autobahn/client/index.json"

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

docker run -d --rm \
    -v "${PWD}/autobahn:/autobahn" \
    -p 9001:9001 \
    --init \
    --name "${CONTAINER_NAME}" \
    crossbario/autobahn-testsuite \
    wstest -m fuzzingserver -s 'autobahn/fuzzingserver.json'

sleep 3
cargo run --release --example autobahn-client
test_diff
