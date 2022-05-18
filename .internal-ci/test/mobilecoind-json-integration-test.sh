#!/bin/bash
# Copyright (c) 2018-2022 The MobileCoin Foundation
#
# Wrapper around the mobilecoind test_client.py to set up environment for testing.
#

set -e

mkdir -p /tmp/strategies/keys

for i in {0..4}
do
    cp /tmp/sample_data/keys/*_${i}.* /tmp/strategies/keys
done

cp /test/mobilecoind/strategies/* /tmp/strategies

pushd /tmp/strategies >/dev/null || exit 1

echo "-- Install requirements"
echo ""
pip3 install -r requirements.txt

echo ""
echo "-- Set up proto files"
echo ""

if [[ -f "/proto/api/external.proto" ]]
then
    python3 -m grpc_tools.protoc \
        -I"/proto/api" \
        --python_out=. "/proto/api/external.proto"
fi

if [[ -f "/proto/api/blockchain.proto" ]]
then
    python3 -m grpc_tools.protoc \
        -I"/proto/api" \
        --python_out=. "/proto/api/blockchain.proto"
fi

if [[ -f "/proto/mobilecoind/mobilecoind_api.proto" ]]
then
    python3 -m grpc_tools.protoc \
        -I"/proto/api" \
        -I"/proto/mobilecoind" \
        -I"/proto/consensus" \
        --python_out=. --grpc_python_out=. "/proto/mobilecoind/mobilecoind_api.proto"
fi

if [[ -f "/proto/api/" ]]
then
python3 -m grpc_tools.protoc \
    -I"/proto/mint-auditor" \
    --python_out=. --grpc_python_out=. "/proto/mint-auditor/mint_auditor.proto"
fi

echo ""
echo "-- Run integration_tests.py"
echo ""
# script mapped from integration_tests.py to mobilecoind_json_integrations_tests.py into bootstrap-toolbox pod
python3 mobilecoind_json_integrations_tests.py \
    --key-dir ./keys \
    --mobilecoind-host "mobilecoind-json" \
    --mobilecoind-port 3229

popd >/dev/null || exit 1
