import { describe, it, expect, beforeEach } from 'vitest';
import { ZkClient } from '@zyber/core';
import { BlacklistManager } from '../src';

describe('ComplianceKit E2E', () => {
  let zkClient: ZkClient;
  let blacklistManager: BlacklistManager;

  beforeEach(() => {
    zkClient = {
      createJob: async () => ({ jobId: 'test-job-id' }),
      getJobStatus: async () => ({ status: 'completed', proof: null }),
    } as unknown as ZkClient;

    blacklistManager = new BlacklistManager(zkClient);
  });

  describe('Blacklist Management', () => {
    it('should add address to blacklist', async () => {
      await blacklistManager.addToBlacklist(
        '0x1234567890123456789012345678901234567890',
        'Sanctions violation',
      );

      const isBlacklisted = await blacklistManager.isBlacklisted(
        '0x1234567890123456789012345678901234567890',
      );

      expect(isBlacklisted).toBe(true);
    });

    it('should handle case-insensitive addresses', async () => {
      await blacklistManager.addToBlacklist(
        '0xABCDEF1234567890123456789012345678901234',
        'Test',
      );

      const isBlacklisted = await blacklistManager.isBlacklisted(
        '0xabcdef1234567890123456789012345678901234',
      );

      expect(isBlacklisted).toBe(true);
    });

    it('should remove address from blacklist', async () => {
      const address = '0x9876543210987654321098765432109876543210';

      await blacklistManager.addToBlacklist(address, 'Test');
      expect(await blacklistManager.isBlacklisted(address)).toBe(true);

      await blacklistManager.removeFromBlacklist(address);
      expect(await blacklistManager.isBlacklisted(address)).toBe(false);
    });

    it('should reject duplicate addresses', async () => {
      const address = '0x1111111111111111111111111111111111111111';

      await blacklistManager.addToBlacklist(address, 'First');

      await expect(
        blacklistManager.addToBlacklist(address, 'Second'),
      ).rejects.toThrow('already blacklisted');
    });

    it('should reject removing non-existent address', async () => {
      await expect(
        blacklistManager.removeFromBlacklist('0x0000000000000000000000000000000000000000'),
      ).rejects.toThrow('not blacklisted');
    });

    it('should get entry details', async () => {
      const address = '0x2222222222222222222222222222222222222222';
      const reason = 'OFAC sanctions';
      const metadata = { source: 'OFAC', date: '2024-01-01' };

      await blacklistManager.addToBlacklist(address, reason, metadata);

      const entry = await blacklistManager.getEntry(address);

      expect(entry).not.toBeNull();
      expect(entry!.reason).toBe(reason);
      expect(entry!.metadata).toEqual(metadata);
    });
  });

  describe('Merkle Tree Operations', () => {
    it('should update merkle root on additions', async () => {
      const initialState = await blacklistManager.getState();
      const initialRoot = initialState.merkleRoot;

      await blacklistManager.addToBlacklist(
        '0x3333333333333333333333333333333333333333',
        'Test',
      );

      const newState = await blacklistManager.getState();
      expect(newState.merkleRoot).not.toBe(initialRoot);
      expect(newState.version).toBe(initialState.version + 1);
    });

    it('should generate merkle proof for blacklisted address', async () => {
      const address = '0x4444444444444444444444444444444444444444';

      await blacklistManager.addToBlacklist(address, 'Test');

      const proof = await blacklistManager.getMerkleProof(address);

      expect(proof.root).toBe(blacklistManager.getMerkleRoot());
      expect(proof.leaf).toBeDefined();
    });

    it('should generate proof for non-blacklisted address', async () => {
      await blacklistManager.addToBlacklist(
        '0x5555555555555555555555555555555555555555',
        'Test',
      );

      const proof = await blacklistManager.getMerkleProof(
        '0x6666666666666666666666666666666666666666',
      );

      expect(proof.root).toBe(blacklistManager.getMerkleRoot());
      expect(proof.path).toHaveLength(0); // Non-inclusion proof
    });

    it('should verify valid merkle proof', async () => {
      const address = '0x7777777777777777777777777777777777777777';

      await blacklistManager.addToBlacklist(address, 'Test');

      const proof = await blacklistManager.getMerkleProof(address);
      const isValid = await blacklistManager.verifyProof(proof);

      expect(isValid).toBe(true);
    });

    it('should update root to zero when last address removed', async () => {
      const address = '0x8888888888888888888888888888888888888888';

      await blacklistManager.addToBlacklist(address, 'Test');
      await blacklistManager.removeFromBlacklist(address);

      const state = await blacklistManager.getState();
      expect(state.merkleRoot).toBe(
        '0x0000000000000000000000000000000000000000000000000000000000000000',
      );
    });
  });

  describe('Batch Operations', () => {
    it('should add multiple addresses in batch', async () => {
      const addresses = [
        '0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        '0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        '0xcccccccccccccccccccccccccccccccccccccccc',
      ];

      const result = await blacklistManager.addBatch(addresses, 'Batch test');

      expect(result.added).toBe(3);
      expect(result.skipped).toBe(0);

      for (const address of addresses) {
        expect(await blacklistManager.isBlacklisted(address)).toBe(true);
      }
    });

    it('should skip duplicates in batch', async () => {
      const address = '0xdddddddddddddddddddddddddddddddddddddddd';

      await blacklistManager.addToBlacklist(address, 'First');

      const result = await blacklistManager.addBatch(
        [address, '0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee'],
        'Batch',
      );

      expect(result.added).toBe(1);
      expect(result.skipped).toBe(1);
    });
  });

  describe('Listing and Filtering', () => {
    beforeEach(async () => {
      await blacklistManager.addToBlacklist('0xa1', 'OFAC sanctions');
      await blacklistManager.addToBlacklist('0xa2', 'Fraud');
      await blacklistManager.addToBlacklist('0xa3', 'OFAC sanctions');
    });

    it('should list all blacklisted addresses', async () => {
      const list = await blacklistManager.listBlacklist();

      expect(list.length).toBeGreaterThanOrEqual(3);
    });

    it('should filter by reason', async () => {
      const list = await blacklistManager.listBlacklist({ reason: 'OFAC' });

      expect(list.length).toBeGreaterThanOrEqual(2);
      expect(list.every((e) => e.reason.includes('OFAC'))).toBe(true);
    });

    it('should limit results', async () => {
      const list = await blacklistManager.listBlacklist({ limit: 2 });

      expect(list.length).toBe(2);
    });

    it('should filter by date', async () => {
      const now = Date.now();
      const list = await blacklistManager.listBlacklist({ since: now - 10000 });

      expect(list.every((e) => e.addedAt >= now - 10000)).toBe(true);
    });
  });

  describe('State Management', () => {
    it('should track blacklist state', async () => {
      const initialState = await blacklistManager.getState();

      await blacklistManager.addToBlacklist('0xtest1', 'Test 1');
      await blacklistManager.addToBlacklist('0xtest2', 'Test 2');

      const newState = await blacklistManager.getState();

      expect(newState.totalEntries).toBeGreaterThan(initialState.totalEntries);
      expect(newState.version).toBeGreaterThan(initialState.version);
      expect(newState.lastUpdated).toBeGreaterThanOrEqual(initialState.lastUpdated);
    });

    it('should export blacklist', async () => {
      await blacklistManager.addToBlacklist('0xexport1', 'Export test 1');
      await blacklistManager.addToBlacklist('0xexport2', 'Export test 2');

      const exported = await blacklistManager.exportBlacklist();

      expect(exported.entries.length).toBeGreaterThanOrEqual(2);
      expect(exported.merkleRoot).toBe(blacklistManager.getMerkleRoot());
      expect(exported.version).toBeGreaterThan(0);
    });

    it('should import blacklist', async () => {
      const exportData = {
        entries: [
          {
            address: '0ximport1',
            reason: 'Import test',
            addedAt: Date.now(),
          },
          {
            address: '0ximport2',
            reason: 'Import test',
            addedAt: Date.now(),
          },
        ],
      };

      await blacklistManager.importBlacklist(exportData);

      expect(await blacklistManager.isBlacklisted('0ximport1')).toBe(true);
      expect(await blacklistManager.isBlacklisted('0ximport2')).toBe(true);

      const state = await blacklistManager.getState();
      expect(state.totalEntries).toBeGreaterThanOrEqual(2);
    });
  });

  describe('Complete Compliance Workflow', () => {
    it('should handle full compliance check lifecycle', async () => {
      // Build blacklist
      const sanctionedAddresses = [
        '0xsanctioned1',
        '0xsanctioned2',
        '0xsanctioned3',
      ];

      await blacklistManager.addBatch(sanctionedAddresses, 'OFAC List 2024');

      // Check clean address
      const cleanAddress = '0xclean1';
      const isCleanBlacklisted = await blacklistManager.isBlacklisted(cleanAddress);
      expect(isCleanBlacklisted).toBe(false);

      // Get proof of innocence (non-inclusion proof)
      const innocenceProof = await blacklistManager.getMerkleProof(cleanAddress);
      expect(innocenceProof.root).toBe(blacklistManager.getMerkleRoot());
      expect(innocenceProof.path).toHaveLength(0); // Non-inclusion proof has empty path

      // Check sanctioned address
      const isSanctionedBlacklisted = await blacklistManager.isBlacklisted('0xsanctioned1');
      expect(isSanctionedBlacklisted).toBe(true);

      // Get sanctioned entry details
      const entry = await blacklistManager.getEntry('0xsanctioned1');
      expect(entry?.reason).toBe('OFAC List 2024');

      // Export for audit
      const auditExport = await blacklistManager.exportBlacklist();
      expect(auditExport.entries.length).toBeGreaterThanOrEqual(3);

      // Get current state
      const state = await blacklistManager.getState();
      expect(state.totalEntries).toBeGreaterThanOrEqual(3);
      expect(state.merkleRoot).toBeDefined();
    });

    it('should update proofs when blacklist changes', async () => {
      const address = '0xdynamic1';

      // Initial state: not blacklisted
      let isBlacklisted = await blacklistManager.isBlacklisted(address);
      expect(isBlacklisted).toBe(false);

      const initialRoot = blacklistManager.getMerkleRoot();

      // Add to blacklist
      await blacklistManager.addToBlacklist(address, 'New sanction');

      const newRoot = blacklistManager.getMerkleRoot();
      expect(newRoot).not.toBe(initialRoot);

      // Now blacklisted
      isBlacklisted = await blacklistManager.isBlacklisted(address);
      expect(isBlacklisted).toBe(true);

      // Generate inclusion proof
      const inclusionProof = await blacklistManager.getMerkleProof(address);
      expect(inclusionProof.root).toBe(newRoot);

      // Remove from blacklist
      await blacklistManager.removeFromBlacklist(address);

      const finalRoot = blacklistManager.getMerkleRoot();
      expect(finalRoot).not.toBe(newRoot);

      // No longer blacklisted
      isBlacklisted = await blacklistManager.isBlacklisted(address);
      expect(isBlacklisted).toBe(false);
    });
  });
});
