#!/bin/bash

set -ex

cargo build

cargo build --target aarch64-unknown-linux-gnu --release
