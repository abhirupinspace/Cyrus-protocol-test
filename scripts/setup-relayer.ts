import fs from 'fs';
import path from 'path';

/**
 * Configure relayer with deployed contract addresses
 */
async function setupRelayer() {
  console.log("⚙️ Setting up relayer configuration...");
  
  // Load deployed contracts
  const deployedPath = path.resolve(__dirname, '../deploy-config/deployedContracts.json');
  const deployed = JSON.parse(fs.readFileSync(deployedPath, 'utf8'));
  
  // Load relayer config template
  const configPath = path.resolve(__dirname, '../deploy-config/relayer-config.toml');
  let config = fs.readFileSync(configPath, 'utf8');
  
  // Update config with deployed addresses
  config = config.replace('program_id = ""', `program_id = "${deployed.solana.CyrusSettlementEmitter}"`);
  config = config.replace('contract_address = ""', `contract_address = "${deployed.aptos.SettlementProcessor}"`);
  config = config.replace('vault_owner = ""', `vault_owner = "${deployed.aptos.VaultOwner}"`);
  
  // Write updated config to relayer directory
  const relayerConfigPath = path.resolve(__dirname, '../relayer/config/relayer.toml');
  fs.writeFileSync(relayerConfigPath, config);
  
  console.log("✅ Relayer configuration updated");
  console.log("📍 Config location: relayer/config/relayer.toml");
}

if (require.main === module) {
  setupRelayer().catch(console.error);
}

export { setupRelayer };
