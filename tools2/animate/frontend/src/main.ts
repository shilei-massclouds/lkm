import { mount } from 'svelte';
import App from './App.svelte';
import './style.css';
import type { AnimationTrace } from './types';

const data = document.getElementById('lkm-signal-animation');
const target = document.getElementById('lkm-signal-player');

if (!(data instanceof HTMLScriptElement) || !(target instanceof HTMLElement)) {
  throw new Error('Signal animation document is missing its data or mount point');
}

const animation = JSON.parse(data.textContent || '') as AnimationTrace;
mount(App, { target, props: { animation } });
