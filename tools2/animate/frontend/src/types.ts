export interface AnimationTrace {
  schema: 'lkm.spec.signal-animation';
  version: 1;
  source: string;
  inputs: { model_fingerprint: string };
  trace: {
    verdict: string;
    total_steps: number;
    root_request: { source: string; target: string; signal: string };
  };
  steps: AnimationStep[];
  initial_frame: AnimationFrame;
  frames: AnimationFrame[];
}

export type HandlerKind = 'Transition' | 'Action' | null;
export type AnimationPhase = 'idle' | 'send' | 'response' | 'clear';

export interface AnimationStep {
  index: number;
  id: string;
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
  step_id: string | null;
  nodes: AnimationNode[];
  sibling_order: Record<string, string[]>;
}
