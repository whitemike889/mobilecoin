#!/bin/bash
# Copyright (c) 2018-2022 The MobileCoin Foundation
#
# Wrapper around the mobilecoind drain-accounts.py to set up environment for testing.
#

strategies_dir=/tmp/drain-accounts/strategies
keys_dir="${strategies_dir}/keys"
fog_keys_dir="${strategies_dir}/fog_keys"

mkdir -p "${keys_dir}"
mkdir -p "${fog_keys_dir}"

echo "-- Copy account keys"
echo ""
for i in {0..6}
do
    # shellcheck disable=SC2086 # yes we want globs
    cp /tmp/sample_data/keys/*_${i}.* "${keys_dir}"
    # shellcheck disable=SC2086 # yes we want globs
    cp /tmp/sample_data/fog_keys*_${i}.* "${fog_keys_dir}"
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
python3 drain-accounts.py \
    --key-dir "${keys_dir}" \
    --dest-keys-dir "${fog_keys_dir}" \
    --mobilecoind-host "mobilecoind" \
    --mobilecoind-port 3229 \
    --token-id 1

popd >/dev/null || exit 1
