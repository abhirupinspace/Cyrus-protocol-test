#!/bin/bash

echo "🚀 Deploying Aptos Settlement Processor..."

cd contracts/aptos

# Compile contract
echo "📦 Compiling Aptos contract..."
aptos move compile --named-addresses cyrus_protocol=default

# Deploy contract
echo "🚢 Deploying to testnet..."
aptos move publish --named-addresses cyrus_protocol=default

# Get account address
ACCOUNT_ADDR=$(aptos config show-profiles --profile default | grep account | cut -d: -f2 | tr -d ' ')

# Update deployed contracts registry
echo "📝 Updating contract registry..."
node -e "
const fs = require('fs');
const path = require('path');
const deployedPath = path.resolve(__dirname, '../deploy-config/deployedContracts.json');
const deployed = JSON.parse(fs.readFileSync(deployedPath, 'utf8'));
deployed.aptos.SettlementProcessor = '$ACCOUNT_ADDR';
deployed.aptos.VaultOwner = '$ACCOUNT_ADDR';
deployed.aptos.deployedAt = new Date().toISOString();
fs.writeFileSync(deployedPath, JSON.stringify(deployed, null, 2));
"

echo "✅ Aptos deployment complete"
echo "�� Contract Address: $ACCOUNT_ADDR"
