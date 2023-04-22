#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

pushd "${SCRIPT_DIR}"

cargo build --release

EXE=$(cargo exe --release)

echo "Killing any existing grobot processes"

sudo pkill grobot

echo "Running grobot.."

sudo bash -c "nohup \"${EXE}\" configs/default.toml > /var/log/grobot.log 2>&1 &"

echo "Grobot running..."
