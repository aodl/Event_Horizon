export const isNeuronMode = mode => mode === 'neuron' || mode === 'neuron-range';

export function normalizeMode(mode, neuronCapable) {
  return isNeuronMode(mode) && !neuronCapable ? 'account' : mode;
}

export function targetLabels(mode) {
  return isNeuronMode(mode)
    ? { single: 'Neuron nonce', start: 'Start neuron nonce', end: 'End neuron nonce' }
    : { single: 'Numbered subaccount', start: 'Start subaccount', end: 'End subaccount' };
}
