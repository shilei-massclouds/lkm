export interface AnimationTrace {
  schema: 'lkm.spec.signal-animation';
  version: 2;
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
  };
  moments: AnimationMoment[];
  initial_frame: AnimationFrame;
  frames: AnimationFrame[];
}

export type HandlerKind = 'Transition' | 'Action' | null;
export type AnimationPhase = 'idle' | 'send' | 'before' | 'response' | 'clear';
export type MomentKind = 'send' | 'complete' | 'rejected' | 'failed' | 'truncated' | 'stopped';

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
  handler: { id: string | null; kind: HandlerKind };
  outcome: 'completed' | 'rejected' | 'failed' | 'truncated' | 'stopped';
  reason: string | null;
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
