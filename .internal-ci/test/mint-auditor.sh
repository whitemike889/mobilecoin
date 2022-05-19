#!/bin/bash
# Copyright (c) 2018-2022 The MobileCoin Foundation
#
# Wrapper around mint-auditor integration_test.py to set up environment for testing.
#

set -e

echo "-- Install python packages"
echo ""
pip3 install grpcio grpcio-tools

echo ""
echo "-- Set up proto files"
echo ""

pushd /test/mint-auditor || exit 1

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
python3 integration_test.py \
    --mobilecoind-addr "mobilecoind-grpc:80" \
    --mint-auditor-addr "mint-auditor:80" \
    --mint-client-bin /usr/local/bin/mc-consensus-mint-client \
    --node-url "mc://node1.${NAMESPACE}.development.mobilecoin.com/" \
    --mint-singing-key /minting-keys/token1-signer.private.pem

popd >/dev/null || exit 1
