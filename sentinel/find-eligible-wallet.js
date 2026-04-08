// Let's just use a well-known mainnet wallet with activity
// We can't actually control it, but we can see what the site would do

const WALLETS_TO_TRY = [
  'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK', // Random mainnet wallet
  'CuieVDEDtLo7FypA9SbLM9saXFdb1dsshEkyErMqkRQq', // Another one
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',  // Token program (has tons of activity)
];

console.log('Wallets to try spoofing:');
WALLETS_TO_TRY.forEach((w, i) => {
  console.log(`${i + 1}. ${w}`);
});

console.log('\nTo use these, we need to either:');
console.log('1. Import the actual seed phrase (if we have it)');
console.log('2. Intercept the Phantom extension itself');
console.log('3. Use a custom wallet stub instead of real Phantom');
