import { defineWalletSetup } from '../../node_modules/walletqa/dist/cache/index.js';

// Use a test-only seed phrase — NEVER use a real wallet's seed phrase.
// In CI, load from environment variables:
//   process.env.WALLET_MNEMONIC ?? '...'
const SEED = 'test test test test test test test test test test test junk';
const PASSWORD = 'TestPassword123!';

export default defineWalletSetup('phantom', async (_context, walletPage) => {
  await walletPage.importWallet(SEED, PASSWORD);
});
