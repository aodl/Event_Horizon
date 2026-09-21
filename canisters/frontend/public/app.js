import { buildMemo } from './memo.js';
const $ = id => document.getElementById(id);
const form = $('memo-form');
function render() {
  const out = $('memo-output'); const help = $('memo-help');
  try {
    const { memo, bytes } = buildMemo($('principal').value, $('subaccount').value, $('amount').value);
    out.textContent = memo;
    help.textContent = `${bytes}/32 bytes · Endow this exact declaration with at least 10 ICP through Jupiter Faucet.${$('amount').value.trim() ? '' : ' Blank threshold means every incoming transfer is relevant.'}`;
    help.classList.remove('error');
  } catch (e) {
    out.textContent = '—'; help.textContent = e.message; help.classList.add('error');
  }
}
form.addEventListener('input', render); render();
