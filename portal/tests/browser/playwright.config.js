// @ts-check
const { defineConfig } = require('@playwright/test');
const path = require('path');

const binaryPath = path.resolve(__dirname, '../../../target/debug/iaop-developer-portal.exe');
const catalogPath = path.resolve(__dirname, '../../../catalog/services.yaml');
const schemaPath = path.resolve(__dirname, '../../../catalog/catalog.schema.json');
const specsDir = path.resolve(__dirname, '../../../specs');

module.exports = defineConfig({
  testDir: '.',
  timeout: 30000,
  expect: {
    timeout: 10000,
  },
  use: {
    baseURL: 'http://127.0.0.1:8099',
    headless: true,
    screenshot: 'on',
  },
  webServer: {
    command: `"${binaryPath}" --port 8099 --host 127.0.0.1 --catalog-path "${catalogPath}" --catalog-schema-path "${schemaPath}" --specs-dir "${specsDir}"`,
    url: 'http://127.0.0.1:8099/health/ready',
    reuseExistingServer: false,
    timeout: 15000,
  },
});
