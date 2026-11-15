/* Theme toggle — wires every [data-theme-toggle] control on the page.
   Persists choice to localStorage and keeps data-theme in sync. */
(function () {
  'use strict';
  var KEY = 'iaop-theme';

  function currentTheme() {
    return document.documentElement.getAttribute('data-theme') ||
      (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  }

  function apply(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    window.__AGENA_THEME__ = theme;
    try { localStorage.setItem(KEY, theme); } catch (e) { /* ignore */ }
    document.querySelectorAll('[data-theme-toggle]').forEach(function (btn) {
      var dark = theme === 'dark';
      btn.setAttribute('aria-pressed', dark ? 'true' : 'false');
      btn.setAttribute('aria-label', dark ? 'Switch to light theme' : 'Switch to dark theme');
      btn.title = dark ? 'Switch to light theme' : 'Switch to dark theme';
    });
  }

  function init() {
    apply(currentTheme());
    document.querySelectorAll('[data-theme-toggle]').forEach(function (btn) {
      btn.addEventListener('click', function () {
        apply(currentTheme() === 'dark' ? 'light' : 'dark');
      });
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
