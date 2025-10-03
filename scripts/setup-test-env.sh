#!/bin/bash

set -e

echo "Setting up test environment..."

mkdir -p ~/.config/solana

if [ ! -f ~/.config/solana/id.json ]; then
    echo "Generating test keypair..."
    solana-keygen new --no-bip39-passphrase -o ~/.config/solana/id.json --force
else
    echo "Test keypair already exists"
fi

echo "Configuring Solana CLI for localnet..."
solana config set --url http://127.0.0.1:8899

echo ""
echo "Current Solana configuration:"
solana config get

echo ""
echo "Setup complete"
echo ""
echo "To run tests:"
echo "  1. Start local validator: solana-test-validator"
echo "  2. Run tests: anchor test --skip-local-validator"
echo ""
