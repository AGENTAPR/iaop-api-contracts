/* Theme bootstrap — runs synchronously in <head> before first paint.
   Resolves stored preference (localStorage) else OS preference, then sets
   data-theme on <html>. ADL tokens + Scalar sync both key off this. */
(function () {
  'use strict';
  var KEY = 'iaop-theme';
  var theme = null;
  try { theme = localStorage.getItem(KEY); } catch (e) { /* storage unavailable */ }
  if (theme !== 'light' && theme !== 'dark') {
    theme = (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches)
      ? 'dark' : 'light';
  }
  document.documentElement.setAttribute('data-theme', theme);
  window.__AGENA_THEME__ = theme;
})();
