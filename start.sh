#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

pushd "${SCRIPT_DIR}"

cargo build --release

EXE=$(cargo exe --release | grep controller)

echo "Killing any existing grobot processes"

sudo pkill controller

echo "Running grobot.."

sudo bash -c "nohup \"${EXE}\" configs/default.toml > /var/log/grobot-out.log 2>&1 &"

echo "Grobot running..."
