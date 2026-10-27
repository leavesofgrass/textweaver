#!/usr/bin/env bash
# Runs textweaver-mqtt's ignored Mosquitto test against Debian 13's own
# Mosquitto in Docker.
#
# 1. Builds the image textweaver-mosquitto-test from the local
#    debian:trixie (never pulled), with only mosquitto and
#    mosquitto-clients from Debian's repository, a password file of test
#    accounts and a read-only ACL for the reader.
# 2. Runs `cargo test -p textweaver-mqtt --test mosquitto -- --ignored`.
#    The test starts its own container (textweaver-mosquitto-test), with
#    ports on 127.0.0.1 only, restarts it once, and removes it, and only
#    it, at the end. The test CA, broker certificate and key are made at
#    run time in the target folder; none is in the repository.
# 3. Runs textweaver-broker's ignored test with Mosquitto's clients from
#    the same image against textweaver's own broker (plain and TLS).
#
# Never prunes; the image is kept for the next run. Needs CARGO_TARGET_DIR
# set. Works in Git Bash on Windows (MSYS_NO_PATHCONV=1 keeps Docker's
# container paths as written) and in Bash on Linux and macOS.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${CARGO_TARGET_DIR:?set CARGO_TARGET_DIR to your own build folder}"
# Git Bash: give Docker a Windows path, since path conversion is off.
ctx="$(cd "$here" && (pwd -W 2>/dev/null || pwd))"
export MSYS_NO_PATHCONV=1
docker build --pull=false -t textweaver-mosquitto-test "$ctx"
cd "$here/../../.."
CARGO_TARGET_DIR="$target" \
  cargo test -p textweaver-mqtt --test mosquitto -- --ignored --nocapture --test-threads=1
# 3. textweaver's own broker with Mosquitto's clients from the same image,
#    each client in a container of its own with --rm.
CARGO_TARGET_DIR="$target" \
  cargo test -p textweaver-broker --test mosquitto_clients -- --ignored --nocapture --test-threads=1
