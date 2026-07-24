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
  initial_frame: AnimationFrame;
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
