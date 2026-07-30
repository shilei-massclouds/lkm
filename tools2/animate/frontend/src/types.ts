export interface AnimationTrace {
  schema: 'lkm.spec.signal-animation';
  version: 3;
  source: string;
  inputs: { model_fingerprint: string };
  trace: {
    verdict: string;
    total_signals: number;
    total_moments: number;
    root_request: { source: string; target: string; signal: string };
    boundary: {
      source: string;
      target: string;
      signal: string;
      normalized_signal: string;
    } | null;
    summary: {
      inventory_deferred: number;
      inventory_trimmed: number;
      boundary_occurrences: number;
      unresolved_obligations: number;
      [key: string]: unknown;
    };
    boundary_inventory: BoundaryInventoryItem[];
    boundary_occurrences: BoundaryOccurrence[];
    obligations: BoundaryObligation[];
  };
  moments: AnimationMoment[];
  initial_frame: AnimationFrame;
  frames: AnimationFrame[];
}

export interface BoundaryInventoryItem {
  id: string;
  status: 'deferred' | 'trimmed';
  category: string;
  summary: string;
  resolution: { kind: string; text: string };
  owner: string;
  state: string | null;
  handler: { kind: string; name: string } | null;
  context: string | null;
  location: string;
  evidence: Array<{ text: string }>;
}

export interface BoundaryProof {
  evidence_index: number;
  expression: string;
  result: boolean;
  proof_source: string;
  classification: string;
}

export interface BoundaryOccurrence {
  id: string;
  sequence: number;
  boundary_id: string;
  occurrence_index: number;
  signal_id: string | null;
  execution_sequence: number;
  owner: string;
  state: string | null;
  context: string[];
  proofs: BoundaryProof[];
}

export interface BoundaryObligation {
  id: string;
  boundary_id: string;
  occurrence_id: string;
  evidence_index: number;
  expression: string;
  owner: string;
  signal_id: string | null;
  proof_source: string;
  classification: string;
  unresolved: true;
}

export type HandlerKind = 'Transition' | 'Action' | null;
export type AnimationPhase = 'idle' | 'request' | 'before' | 'feedback' | 'settle' | 'terminal' | 'clear';
export type MomentKind = 'request' | 'feedback' | 'settle' | 'terminal';

export interface AnimationMoment {
  index: number;
  id: string;
  kind: MomentKind;
  event_sequence: number;
  signal_id: string;
  cause_id: string | null;
  source: string;
  target: string;
  signal: string;
  delivery: string;
  handler: { id: string | null; kind: HandlerKind; description?: string };
  outcome: 'completed' | 'rejected' | 'failed' | 'truncated' | 'stopped';
  reason: string | null;
  transfer: { from: string; to: string } | null;
  response: { before_state: string | null; after_state: string | null };
}

export interface AnimationNode {
  id: string;
  parent: string | null;
  kind: 'system' | 'external';
  state: string | null;
  structural: boolean;
  first_seen: number;
}

export interface AnimationFrame {
  index: number;
  moment_id: string | null;
  nodes: AnimationNode[];
  sibling_order: Record<string, string[]>;
}
