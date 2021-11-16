git clean -fd

openssl genrsa -out Enclave_private.pem -3 3072
export SGX_MODE=SW
export IAS_MODE=DEV
export CONSENSUS_ENCLAVE_PRIVKEY=$(pwd)/Enclave_private.pem
export INGEST_ENCLAVE_PRIVKEY=$(pwd)/Enclave_private.pem
export LEDGER_ENCLAVE_PRIVKEY=$(pwd)/Enclave_private.pem
export VIEW_ENCLAVE_PRIVKEY=$(pwd)/Enclave_private.pem
export MC_LOG=debug

cargo build -p mc-util-keyfile -p mc-util-generate-sample-ledger -p mc-consensus-service -p mc-ledger-distribution -p mc-admin-http-gateway -p mc-util-grpc-admin-tool -p mc-mobilecoind -p mc-crypto-x509-test-vectors -p mc-fog-distribution -p mc-fog-test-client -p mc-fog-ingest-server -p mc-fog-ingest-client -p mc-fog-view-server -p mc-fog-report-server -p mc-fog-ledger-server -p mc-fog-sql-recovery-db --release

export FOG_AUTHORITY_ROOT=$(./target/release/mc-crypto-x509-test-vectors --type=chain --test-name=ok_rsa_head)
./target/release/sample-keys --num 10 --seed=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
./target/release/generate-sample-ledger --txs 100

service postgresql start
