#!/usr/bin/env bash

set -euxo pipefail

# Get directory script is in
SOURCE="${BASH_SOURCE[0]}"
while [ -h "$SOURCE" ]; do # resolve $SOURCE until the file is no longer a symlink
  DIR="$( cd -P "$( dirname "$SOURCE" )" >/dev/null 2>&1 && pwd )"
  SOURCE="$(readlink "$SOURCE")"
  [[ $SOURCE != /* ]] && SOURCE="$DIR/$SOURCE" # if $SOURCE was a relative symlink, we need to resolve it relative to the path where the symlink file was located
done
DIR="$( cd -P "$( dirname "$SOURCE" )" >/dev/null 2>&1 && pwd )"
TOP=$(realpath "${DIR}/..")
BIN=${TOP}/bin

# Remove existing blobs because otherwise this will append object files to the old blobs
rm -f "${BIN}"/*.a

arm-none-eabi-gcc -c -march=armv7e-m "${DIR}/efc.c" -o "${BIN}/efc.o"
arm-none-eabi-ar crs "${BIN}/thumbv7em-none-eabi.a" "${BIN}/efc.o"

arm-none-eabi-gcc -c -march=armv7e-m "${DIR}/efc.c" -DHAS_FPU -o "${BIN}/efc.o"
arm-none-eabi-ar crs "${BIN}/thumbv7em-none-eabihf.a" "${BIN}/efc.o"

# Cleanup object files
rm -f "${BIN}"/*.o
