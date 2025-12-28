import { describe, it, expect, beforeEach } from 'vitest';
import { ZkClient } from '@zyber/core';
import { PollManager, VoteAggregator } from '../src';

describe('PrivateVote E2E', () => {
  let zkClient: ZkClient;
  let pollManager: PollManager;
  let voteAggregator: VoteAggregator;

  beforeEach(() => {
    // Mock ZkClient for testing
    zkClient = {
      createJob: async () => ({ jobId: 'test-job-id' }),
      getJobStatus: async () => ({ status: 'completed', proof: null }),
    } as unknown as ZkClient;

    pollManager = new PollManager(zkClient);
    voteAggregator = new VoteAggregator(zkClient);
  });

  describe('Poll Lifecycle', () => {
    it('should create a poll', async () => {
      const poll = await pollManager.createPoll({
        pollId: 'poll-001',
        options: ['Option A', 'Option B', 'Option C'],
        eligibleVotersRoot: '0x1234',
        endTime: Date.now() + 86400000, // 24 hours from now
      });

      expect(poll.id).toBe('poll-001');
      expect(poll.options).toHaveLength(3);
      expect(poll.status).toBe('active');
    });

    it('should get poll status', async () => {
      await pollManager.createPoll({
        pollId: 'poll-002',
        options: ['Yes', 'No'],
        eligibleVotersRoot: '0x5678',
        endTime: Date.now() + 86400000,
      });

      const status = await pollManager.getPollStatus('poll-002');
      expect(status.status).toBe('active');
      expect(status.votesCount).toBe(0);
    });

    it('should close a poll', async () => {
      await pollManager.createPoll({
        pollId: 'poll-003',
        options: ['A', 'B'],
        eligibleVotersRoot: '0xabcd',
        endTime: Date.now() + 86400000,
      });

      await pollManager.closePoll('poll-003');
      const status = await pollManager.getPollStatus('poll-003');
      expect(status.status).toBe('closed');
    });

    it('should auto-close expired poll', async () => {
      await pollManager.createPoll({
        pollId: 'poll-004',
        options: ['A', 'B'],
        eligibleVotersRoot: '0xdef0',
        endTime: Date.now() - 1000, // Already expired
      });

      const status = await pollManager.getPollStatus('poll-004');
      expect(status.status).toBe('closed');
    });

    it('should increment vote count', async () => {
      await pollManager.createPoll({
        pollId: 'poll-005',
        options: ['A', 'B'],
        eligibleVotersRoot: '0x1111',
        endTime: Date.now() + 86400000,
      });

      await pollManager.incrementVoteCount('poll-005');
      await pollManager.incrementVoteCount('poll-005');

      const status = await pollManager.getPollStatus('poll-005');
      expect(status.votesCount).toBe(2);
    });

    it('should list polls by status', async () => {
      await pollManager.createPoll({
        pollId: 'poll-006',
        options: ['A', 'B'],
        eligibleVotersRoot: '0x2222',
        endTime: Date.now() + 86400000,
      });

      await pollManager.createPoll({
        pollId: 'poll-007',
        options: ['A', 'B'],
        eligibleVotersRoot: '0x3333',
        endTime: Date.now() + 86400000,
      });

      await pollManager.closePoll('poll-007');

      const activePolls = await pollManager.listPolls('active');
      const closedPolls = await pollManager.listPolls('closed');

      expect(activePolls.length).toBeGreaterThan(0);
      expect(closedPolls.length).toBeGreaterThan(0);
    });
  });

  describe('Vote Aggregation', () => {
    it('should aggregate votes correctly', async () => {
      const votes = [
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'nullifier-1',
          choice: 0,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'nullifier-2',
          choice: 1,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'nullifier-3',
          choice: 0,
        },
      ];

      const result = await voteAggregator.aggregateVotes('poll-001', votes);

      expect(result.totalVotes).toBe(3);
      expect(result.results.get(0)).toBe(2);
      expect(result.results.get(1)).toBe(1);
    });

    it('should prevent double voting with same nullifier', async () => {
      const votes = [
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'duplicate-nullifier',
          choice: 0,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'duplicate-nullifier',
          choice: 1,
        },
      ];

      const result = await voteAggregator.aggregateVotes('poll-001', votes);

      expect(result.totalVotes).toBe(1); // Only first vote counted
    });

    it('should get vote distribution', async () => {
      const votes = [
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'n1',
          choice: 0,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'n2',
          choice: 1,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'n3',
          choice: 2,
        },
      ];

      const result = await voteAggregator.aggregateVotes('poll-001', votes);
      const distribution = voteAggregator.getVoteDistribution(result);

      expect(distribution[0]).toBe(1);
      expect(distribution[1]).toBe(1);
      expect(distribution[2]).toBe(1);
    });

    it('should determine winners', async () => {
      const votes = [
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'n1',
          choice: 0,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'n2',
          choice: 0,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-001', '1'],
          nullifier: 'n3',
          choice: 1,
        },
      ];

      const result = await voteAggregator.aggregateVotes('poll-001', votes);
      const winners = voteAggregator.getWinners(result);

      expect(winners).toEqual([0]); // Option 0 wins with 2 votes
    });
  });

  describe('Complete Workflow', () => {
    it('should complete full poll lifecycle', async () => {
      // Create poll
      const poll = await pollManager.createPoll({
        pollId: 'poll-workflow',
        options: ['Alice', 'Bob', 'Charlie'],
        eligibleVotersRoot: '0xworkflow',
        endTime: Date.now() + 86400000,
      });

      expect(poll.status).toBe('active');

      // Simulate votes
      await pollManager.incrementVoteCount('poll-workflow');
      await pollManager.incrementVoteCount('poll-workflow');
      await pollManager.incrementVoteCount('poll-workflow');

      const state = await pollManager.getPollState('poll-workflow');
      expect(state.totalVotes).toBe(3);

      // Close poll
      await pollManager.closePoll('poll-workflow');

      // Aggregate votes
      const votes = [
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-workflow', '1'],
          nullifier: 'voter-1',
          choice: 0,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-workflow', '1'],
          nullifier: 'voter-2',
          choice: 0,
        },
        {
          proof: { pi_a: ['0'], pi_b: [['0', '0']], pi_c: ['0'], protocol: 'groth16', curve: 'bn128' },
          publicSignals: ['poll-workflow', '1'],
          nullifier: 'voter-3',
          choice: 1,
        },
      ];

      const result = await voteAggregator.aggregateVotes('poll-workflow', votes);
      const winners = voteAggregator.getWinners(result);

      expect(winners).toEqual([0]); // Alice wins

      // Mark as tallied
      await pollManager.markAsTallied('poll-workflow');
      const finalPoll = await pollManager.getPollStatus('poll-workflow');
      expect(finalPoll.status).toBe('tallied');
    });
  });
});
