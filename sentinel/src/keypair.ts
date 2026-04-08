import { Keypair } from '@solana/web3.js';

/**
 * Generate a fresh throwaway Solana keypair
 * This keypair is never persisted and has zero balance
 */
export function generateThrowawayKeypair(): Keypair {
  return Keypair.generate();
}

/**
 * Get the public key as a base58 string
 */
export function getPublicKeyString(keypair: Keypair): string {
  return keypair.publicKey.toBase58();
}
