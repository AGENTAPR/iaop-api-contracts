/* Scalar darkMode sync — executes after theme-init.js and before
   scalar.min.js on /reference/* pages. Rewrites the api-reference
   element's data-configuration so Scalar initializes in the same theme
   the portal chrome is using (no reliance on OS preference alone). */
(function () {
  'use strict';
  var el = document.getElementById('api-reference');
  if (!el) return;
  var theme = window.__AGENA_THEME__ ||
    document.documentElement.getAttribute('data-theme') || 'light';
  var raw = el.getAttribute('data-configuration');
  if (!raw) return;
  try {
    var cfg = JSON.parse(raw);
    cfg.darkMode = theme === 'dark';
    cfg.hideDarkModeToggle = true;
    el.setAttribute('data-configuration', JSON.stringify(cfg));
  } catch (e) { /* leave server-rendered config untouched */ }
})();
