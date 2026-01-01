import {
  buildPortfolioComplianceWitness,
  buildNetWorthWitness,
  buildProofOfInnocenceWitness,
  type Groth16Proof,
  type LoadVKeyOptions,
  type PortfolioComplianceWitnessInput,
  type NetWorthWitnessInput,
  type ProofOfInnocenceWitnessInput,
  verifyGroth16,
  ZkClient,
  ZkCircuitType,
} from '@zyber/core';

export interface ProveInnocenceParams {
  walletHash: Uint8Array;
  blacklistRoot: Uint8Array;
  merkleProof: Uint8Array;
  witnessData?: Uint8Array;
}

export interface ProveComplianceParams {
  portfolioHash: Uint8Array;
  complianceRoot: Uint8Array;
  threshold: bigint;
  timestamp: bigint;
  balances: bigint[];
  thresholds: { min?: bigint; max?: bigint }[];
  complianceRule: Uint8Array;
  witnessData?: Uint8Array;
}

export interface ProveNetWorthParams {
  netWorth: bigint;
  blinding: bigint;
  oracleProof: Uint8Array;
  witnessData?: Uint8Array;
}

export interface ComplianceProof {
  proof: Groth16Proof;
  publicSignals: Array<string | bigint>;
}

export interface VerifyParams {
  proof: ComplianceProof;
  loadOptions?: LoadVKeyOptions;
}

/**
 * ComplianceKit manager: delegates proving to zk-generator and verifies locally with VKeys.
 */
export class ComplianceKit {
  constructor(private zk: ZkClient) {}

  static async connect(zk: ZkClient): Promise<ComplianceKit> {
    return new ComplianceKit(zk);
  }

  async proveInnocence(params: ProveInnocenceParams): Promise<void> {
    const witness =
      params.witnessData ??
      buildProofOfInnocenceWitness({
        walletHash: params.walletHash,
        sanctionsRoot: params.blacklistRoot,
        merkleProof: params.merkleProof,
      } satisfies ProofOfInnocenceWitnessInput);

    await this.zk.createJob({
      circuitType: ZkCircuitType.ProofOfInnocence,
      witnessData: witness,
    });
  }

  async proveCompliance(params: ProveComplianceParams): Promise<void> {
    const witness =
      params.witnessData ??
      buildPortfolioComplianceWitness({
        portfolioHash: params.portfolioHash,
        balances: params.balances,
        thresholds: params.thresholds,
        complianceRule: params.complianceRule,
      } satisfies PortfolioComplianceWitnessInput);

    await this.zk.createJob({
      circuitType: ZkCircuitType.PortfolioCompliance,
      witnessData: witness,
    });
  }

  async proveNetWorth(params: ProveNetWorthParams): Promise<void> {
    const witness =
      params.witnessData ??
      buildNetWorthWitness({
        netWorth: params.netWorth,
        blinding: params.blinding,
        oracleProof: params.oracleProof,
      } satisfies NetWorthWitnessInput);

    await this.zk.createJob({
      circuitType: ZkCircuitType.PortfolioNetWorth,
      witnessData: witness,
    });
  }

  async verifyInnocence(params: VerifyParams): Promise<boolean> {
    return verifyGroth16({
      circuitId: ZkCircuitType.ProofOfInnocence,
      proof: params.proof.proof,
      publicSignals: params.proof.publicSignals,
      loadOptions: params.loadOptions,
    });
  }

  async verifyCompliance(params: VerifyParams): Promise<boolean> {
    return verifyGroth16({
      circuitId: ZkCircuitType.PortfolioCompliance,
      proof: params.proof.proof,
      publicSignals: params.proof.publicSignals,
      loadOptions: params.loadOptions,
    });
  }

  async verifyNetWorth(params: VerifyParams): Promise<boolean> {
    return verifyGroth16({
      circuitId: ZkCircuitType.PortfolioNetWorth,
      proof: params.proof.proof,
      publicSignals: params.proof.publicSignals,
      loadOptions: params.loadOptions,
    });
  }
}

// Export lifecycle management
export {
  BlacklistManager,
  type BlacklistEntry,
  type MerkleProof,
  type BlacklistState,
} from './blacklist-manager';
