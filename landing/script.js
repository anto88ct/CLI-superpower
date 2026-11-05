(() => {
  const ua = (navigator.userAgentData?.platform || navigator.platform || navigator.userAgent || '').toLowerCase();
  const os = /win/.test(ua) ? 'windows' : /mac|iphone|ipad/.test(ua) ? 'macos' : /linux|x11|android/.test(ua) ? 'linux' : '';
  document.querySelector('.dl-item[data-os="' + os + '"]')?.classList.add('is-yours');

  const term = document.getElementById('demo-term');
  const label = document.getElementById('theme-label');
  const chips = document.querySelectorAll('.chip');
  if (!term) return;
  const classes = ['t-campbell-ps', 't-onehalf', 't-solarized', 't-onehalf-light', 't-tango-light'];
  const names = {
    'campbell-ps': 'Campbell Powershell', onehalf: 'One Half Dark', solarized: 'Solarized Dark',
    'onehalf-light': 'One Half Light', 'tango-light': 'Tango Light',
  };
  chips.forEach(chip => chip.addEventListener('click', () => {
    const key = chip.dataset.theme;
    term.classList.remove(...classes);
    term.classList.add('t-' + key);
    chips.forEach(other => other.setAttribute('aria-pressed', String(other === chip)));
    if (label) label.textContent = 'Tema ' + names[key];
  }));
})();
