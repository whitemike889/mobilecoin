git clean -fd
service postgresql start
export FOG_AUTHORITY_ROOT=$(./target/release/mc-crypto-x509-test-vectors --type=chain --test-name=ok_rsa_head)
./target/release/sample-keys --num 10 --seed=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
./target/release/generate-sample-ledger --txs 100
MC_LOG="info,rustls=warn,hyper=warn,tokio_reactor=warn,mio=warn,want=warn,rusoto_core=error,h2=error,reqwest=error,rocket=error,<unknown>=error" \
    LEDGER_BASE=$(pwd)/ledger \
    python3 tools/fog-local-network/fog_local_network.py --network-type dense5 --skip-build &
zsh:1: command not found: :q
./target/release/sample-keys --num 4 --output-dir fog_keys --fog-report-url 'insecure-fog://localhost:6200' --fog-authority-root $FOG_AUTHORITY_ROOT
./target/release/fog-distribution \
    --sample-data-dir . \
    --max-threads 1 \
    --peer insecure-mc://localhost:3200/ \
    --peer insecure-mc://localhost:3201/ \
    --peer insecure-mc://localhost:3202/ \
    --peer insecure-mc://localhost:3203/ \
    --peer insecure-mc://localhost:3204/ \
    --num-tx-to-send 10
        sleep 5
        ./target/release/test_client \
            --consensus insecure-mc://localhost:3200/ \
            --consensus insecure-mc://localhost:3201/ \
            --consensus insecure-mc://localhost:3202/ \
            --consensus insecure-mc://localhost:3203/ \
            --consensus insecure-mc://localhost:3204/ \
            --num-clients 4 \
            --num-transactions 200 \
            --consensus-wait 300 \
            --transfer-amount 20 \
            --fog-view insecure-fog-view://localhost:8200 \
            --fog-ledger insecure-fog-ledger://localhost:8200 \
            --key-dir $(pwd)/fog_keys

