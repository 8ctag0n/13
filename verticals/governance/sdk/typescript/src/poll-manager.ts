import { ZkClient } from '@zyber/core';

export interface Poll {
  id: string;
  options: string[];
  merkleRoot: string;
  endTime: number;
  status: 'active' | 'closed' | 'tallied';
  createdAt: number;
  votesCount?: number;
}

export interface CreatePollParams {
  pollId: string;
  options: string[];
  eligibleVotersRoot: string;
  endTime: number;
  minBalance?: bigint;
  requirePoI?: boolean;
}

export interface PollState {
  poll: Poll;
  totalVotes: number;
  isClosed: boolean;
  canTally: boolean;
}

/**
 * Manages poll lifecycle: creation, monitoring, and closing.
 * Does not handle vote casting (use PrivateVote class for that).
 */
export class PollManager {
  private polls: Map<string, Poll> = new Map();

  constructor(private zk: ZkClient) {}

  /**
   * Create a new poll with specified parameters.
   */
  async createPoll(params: CreatePollParams): Promise<Poll> {
    const poll: Poll = {
      id: params.pollId,
      options: params.options,
      merkleRoot: params.eligibleVotersRoot,
      endTime: params.endTime,
      status: 'active',
      createdAt: Date.now(),
      votesCount: 0,
    };

    this.polls.set(params.pollId, poll);
    return poll;
  }

  /**
   * Close a poll, preventing further votes.
   */
  async closePoll(pollId: string): Promise<void> {
    const poll = this.polls.get(pollId);
    if (!poll) {
      throw new Error(`Poll ${pollId} not found`);
    }

    if (poll.status !== 'active') {
      throw new Error(`Poll ${pollId} is already ${poll.status}`);
    }

    poll.status = 'closed';
    this.polls.set(pollId, poll);
  }

  /**
   * Get current poll status and metadata.
   */
  async getPollStatus(pollId: string): Promise<Poll> {
    const poll = this.polls.get(pollId);
    if (!poll) {
      throw new Error(`Poll ${pollId} not found`);
    }

    // Auto-close if past endTime
    if (poll.status === 'active' && Date.now() > poll.endTime) {
      poll.status = 'closed';
      this.polls.set(pollId, poll);
    }

    return poll;
  }

  /**
   * Get detailed poll state for UI.
   */
  async getPollState(pollId: string): Promise<PollState> {
    const poll = await this.getPollStatus(pollId);

    return {
      poll,
      totalVotes: poll.votesCount ?? 0,
      isClosed: poll.status !== 'active',
      canTally: poll.status === 'closed',
    };
  }

  /**
   * Mark poll as tallied after aggregation.
   */
  async markAsTallied(pollId: string): Promise<void> {
    const poll = this.polls.get(pollId);
    if (!poll) {
      throw new Error(`Poll ${pollId} not found`);
    }

    if (poll.status !== 'closed') {
      throw new Error(`Cannot tally poll ${pollId} in status ${poll.status}`);
    }

    poll.status = 'tallied';
    this.polls.set(pollId, poll);
  }

  /**
   * List all polls with optional status filter.
   */
  async listPolls(status?: Poll['status']): Promise<Poll[]> {
    const allPolls = Array.from(this.polls.values());
    return status ? allPolls.filter((p) => p.status === status) : allPolls;
  }

  /**
   * Increment vote count (called after successful vote submission).
   */
  async incrementVoteCount(pollId: string): Promise<void> {
    const poll = this.polls.get(pollId);
    if (!poll) {
      throw new Error(`Poll ${pollId} not found`);
    }

    poll.votesCount = (poll.votesCount ?? 0) + 1;
    this.polls.set(pollId, poll);
  }
}
