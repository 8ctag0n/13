import { ZkClient } from '@zyber/core';

export interface BlacklistEntry {
  address: string;
  reason: string;
  addedAt: number;
  addedBy?: string;
  metadata?: Record<string, unknown>;
}

export interface MerkleProof {
  root: string;
  leaf: string;
  path: string[];
  indices: number[];
}

export interface BlacklistState {
  totalEntries: number;
  merkleRoot: string;
  lastUpdated: number;
  version: number;
}

/**
 * Manages blacklist/sanctions list with Merkle tree proofs.
 * Used for Proof of Innocence (PoI) circuits.
 */
export class BlacklistManager {
  private blacklist: Map<string, BlacklistEntry> = new Map();
  private merkleRoot: string = '0x0000000000000000000000000000000000000000000000000000000000000000';
  private version: number = 0;
  private lastUpdated: number = Date.now();

  constructor(private zk: ZkClient) {}

  /**
   * Add an address to the blacklist.
   */
  async addToBlacklist(
    address: string,
    reason: string,
    metadata?: Record<string, unknown>,
  ): Promise<void> {
    if (this.blacklist.has(address)) {
      throw new Error(`Address ${address} is already blacklisted`);
    }

    const entry: BlacklistEntry = {
      address: address.toLowerCase(),
      reason,
      addedAt: Date.now(),
      metadata,
    };

    this.blacklist.set(address.toLowerCase(), entry);
    await this.updateMerkleRoot();
  }

  /**
   * Remove an address from the blacklist.
   */
  async removeFromBlacklist(address: string): Promise<void> {
    const normalizedAddress = address.toLowerCase();
    if (!this.blacklist.has(normalizedAddress)) {
      throw new Error(`Address ${address} is not blacklisted`);
    }

    this.blacklist.delete(normalizedAddress);
    await this.updateMerkleRoot();
  }

  /**
   * Check if an address is blacklisted.
   */
  async isBlacklisted(address: string): Promise<boolean> {
    return this.blacklist.has(address.toLowerCase());
  }

  /**
   * Get blacklist entry details.
   */
  async getEntry(address: string): Promise<BlacklistEntry | null> {
    return this.blacklist.get(address.toLowerCase()) ?? null;
  }

  /**
   * Generate Merkle proof for an address.
   * For non-blacklisted addresses, this proves non-inclusion.
   */
  async getMerkleProof(address: string): Promise<MerkleProof> {
    const normalizedAddress = address.toLowerCase();
    const isBlacklisted = this.blacklist.has(normalizedAddress);

    // Get all addresses in sorted order for deterministic tree
    const addresses = Array.from(this.blacklist.keys()).sort();

    if (!isBlacklisted) {
      // Proof of non-inclusion
      return this.generateNonInclusionProof(normalizedAddress, addresses);
    }

    // Proof of inclusion
    return this.generateInclusionProof(normalizedAddress, addresses);
  }

  /**
   * Generate inclusion proof for blacklisted address.
   */
  private generateInclusionProof(
    address: string,
    addresses: string[],
  ): MerkleProof {
    const index = addresses.indexOf(address);
    const path: string[] = [];
    const indices: number[] = [];

    // Build Merkle path (simplified - would use actual Merkle tree library)
    let currentIndex = index;
    let currentLevel = addresses.length;

    while (currentLevel > 1) {
      const siblingIndex = currentIndex % 2 === 0 ? currentIndex + 1 : currentIndex - 1;

      if (siblingIndex < currentLevel) {
        path.push(this.hashAddress(addresses[siblingIndex] ?? addresses[currentIndex]));
        indices.push(currentIndex % 2);
      }

      currentIndex = Math.floor(currentIndex / 2);
      currentLevel = Math.ceil(currentLevel / 2);
    }

    return {
      root: this.merkleRoot,
      leaf: this.hashAddress(address),
      path,
      indices,
    };
  }

  /**
   * Generate non-inclusion proof for non-blacklisted address.
   */
  private generateNonInclusionProof(
    address: string,
    addresses: string[],
  ): MerkleProof {
    // For non-inclusion, we prove that the address hash is not in the tree
    // This is a simplified version - real implementation would use sparse Merkle tree
    return {
      root: this.merkleRoot,
      leaf: this.hashAddress(address),
      path: [],
      indices: [],
    };
  }

  /**
   * Update Merkle root after blacklist changes.
   */
  private async updateMerkleRoot(): Promise<void> {
    const addresses = Array.from(this.blacklist.keys()).sort();

    if (addresses.length === 0) {
      this.merkleRoot = '0x0000000000000000000000000000000000000000000000000000000000000000';
      this.version++;
      this.lastUpdated = Date.now();
      return;
    }

    // Build Merkle tree (simplified - would use actual Merkle tree library)
    let currentLevel = addresses.map((addr) => this.hashAddress(addr));

    while (currentLevel.length > 1) {
      const nextLevel: string[] = [];
      for (let i = 0; i < currentLevel.length; i += 2) {
        const left = currentLevel[i];
        const right = currentLevel[i + 1] ?? left;
        nextLevel.push(this.hashPair(left, right));
      }
      currentLevel = nextLevel;
    }

    this.merkleRoot = currentLevel[0];
    this.version++;
    this.lastUpdated = Date.now();
  }

  /**
   * Get current blacklist state.
   */
  async getState(): Promise<BlacklistState> {
    return {
      totalEntries: this.blacklist.size,
      merkleRoot: this.merkleRoot,
      lastUpdated: this.lastUpdated,
      version: this.version,
    };
  }

  /**
   * Get current Merkle root.
   */
  getMerkleRoot(): string {
    return this.merkleRoot;
  }

  /**
   * List all blacklisted addresses with optional filtering.
   */
  async listBlacklist(filter?: {
    reason?: string;
    since?: number;
    limit?: number;
  }): Promise<BlacklistEntry[]> {
    let entries = Array.from(this.blacklist.values());

    if (filter?.reason) {
      entries = entries.filter((e) =>
        e.reason.toLowerCase().includes(filter.reason!.toLowerCase()),
      );
    }

    if (filter?.since) {
      entries = entries.filter((e) => e.addedAt >= filter.since!);
    }

    entries.sort((a, b) => b.addedAt - a.addedAt);

    if (filter?.limit) {
      entries = entries.slice(0, filter.limit);
    }

    return entries;
  }

  /**
   * Batch add multiple addresses to blacklist.
   */
  async addBatch(
    addresses: string[],
    reason: string,
  ): Promise<{ added: number; skipped: number }> {
    let added = 0;
    let skipped = 0;

    for (const address of addresses) {
      const normalized = address.toLowerCase();
      if (this.blacklist.has(normalized)) {
        skipped++;
        continue;
      }

      this.blacklist.set(normalized, {
        address: normalized,
        reason,
        addedAt: Date.now(),
      });
      added++;
    }

    await this.updateMerkleRoot();

    return { added, skipped };
  }

  /**
   * Verify a Merkle proof against current root.
   */
  async verifyProof(proof: MerkleProof): Promise<boolean> {
    if (proof.root !== this.merkleRoot) {
      return false;
    }

    // Reconstruct root from proof
    let currentHash = proof.leaf;

    for (let i = 0; i < proof.path.length; i++) {
      const sibling = proof.path[i];
      const isLeft = proof.indices[i] === 0;

      currentHash = isLeft
        ? this.hashPair(currentHash, sibling)
        : this.hashPair(sibling, currentHash);
    }

    return currentHash === proof.root;
  }

  /**
   * Export blacklist for backup or sharing.
   */
  async exportBlacklist(): Promise<{
    entries: BlacklistEntry[];
    merkleRoot: string;
    version: number;
    exportedAt: number;
  }> {
    return {
      entries: Array.from(this.blacklist.values()),
      merkleRoot: this.merkleRoot,
      version: this.version,
      exportedAt: Date.now(),
    };
  }

  /**
   * Import blacklist from backup.
   */
  async importBlacklist(data: {
    entries: BlacklistEntry[];
    merkleRoot?: string;
  }): Promise<void> {
    this.blacklist.clear();

    for (const entry of data.entries) {
      this.blacklist.set(entry.address.toLowerCase(), entry);
    }

    await this.updateMerkleRoot();

    // Verify imported root matches if provided
    if (data.merkleRoot && data.merkleRoot !== this.merkleRoot) {
      console.warn('Imported Merkle root does not match recalculated root');
    }
  }

  /**
   * Simple hash function for addresses (would use keccak256 in production).
   */
  private hashAddress(address: string): string {
    // Simplified - would use proper hash function
    let hash = 0;
    for (let i = 0; i < address.length; i++) {
      hash = (hash << 5) - hash + address.charCodeAt(i);
      hash = hash & hash;
    }
    return '0x' + Math.abs(hash).toString(16).padStart(64, '0');
  }

  /**
   * Hash two values together for Merkle tree.
   */
  private hashPair(left: string, right: string): string {
    // Simplified - would use proper hash function with sorted inputs
    return this.hashAddress(left + right);
  }
}
