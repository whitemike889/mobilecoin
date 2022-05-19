#!/bin/bash
# Copyright (c) 2018-2022 The MobileCoin Foundation
#
# Wrapper around the mobilecoind-json integration_test.py to set up environment for testing.
#

set -e

strategies_dir=/tmp/mobilecoind-json/strategies
keys_dir="${strategies_dir}/keys"

mkdir -p "${keys_dir}"

for i in {0..6}
do
    # shellcheck disable=SC2086
    cp /tmp/sample_data/keys/*_${i}.* "${keys_dir}"
done

# This uses some of the same lib py files as mobilecoind tests.
cp /test/mobilecoind/strategies/* "${strategies_dir}"

pushd "${strategies_dir}" >/dev/null || exit 1

echo "-- Install requirements"
echo ""
pip3 install -r requirements.txt

echo ""
echo "-- Set up proto files"
echo ""

python3 -m grpc_tools.protoc \
    -I"/proto/api" \
    --python_out=. \
    "/proto/api/external.proto"

python3 -m grpc_tools.protoc \
    -I"/proto/api" \
    --python_out=. \
    "/proto/api/blockchain.proto"

python3 -m grpc_tools.protoc \
    -I"/proto/api" \
    -I"/proto/mobilecoind" \
    -I"/proto/consensus" \
    --python_out=. --grpc_python_out=. \
    "/proto/mobilecoind/mobilecoind_api.proto"

python3 -m grpc_tools.protoc \
    -I"/proto/mint-auditor" \
    --python_out=. --grpc_python_out=. \
    "/proto/mint-auditor/mint_auditor.proto"

echo ""
echo "-- Run integration_test.py"
echo ""
python3 /test/mobilecoin-json/integration_test.py \
    --key-dir "${keys_dir}" \
    --mobilecoind-host "mobilecoind-json" \
    --mobilecoind-port 9090

popd >/dev/null || exit 1
