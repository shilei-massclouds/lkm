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
}
