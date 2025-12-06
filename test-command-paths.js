// Test script to check command paths saving
const fs = require('fs');
const path = require('path');

// Get the config file path
const homeDir = process.env.HOME || process.env.USERPROFILE;
const configPath = path.join(homeDir, '.mcprouter', 'config.json');

console.log('Checking config file at:', configPath);

// Read the config file
if (fs.existsSync(configPath)) {
  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
  console.log('Config file exists:');
  console.log(JSON.stringify(config, null, 2));

  // Check if command_paths exists
  if (config.settings && config.settings.command_paths) {
    console.log('\ncommand_paths found:', config.settings.command_paths);
  } else {
    console.log('\ncommand_paths NOT found in config!');
  }
} else {
  console.log('Config file does NOT exist!');
}
