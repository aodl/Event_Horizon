export function renderRuntimeError(node, message) {
  node.textContent = String(message);
  node.classList.add('error');
}
