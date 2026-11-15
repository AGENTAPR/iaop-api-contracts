/* IAOP Developer Portal — catalog renderer + session chrome.
   Served same-origin at /static/scripts/portal.js (CSP: script-src 'self').
   All catalog data is rendered via DOM APIs — no innerHTML, no injection. */
(function () {
  'use strict';

  var FRONTEND_GRID = document.getElementById('frontend-groups');
  var BACKEND_SECTIONS = document.getElementById('backend-sections');

  var state = { loaded: false, error: null };

  function el(tag, className, text) {
    var node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined && text !== null) node.textContent = String(text);
    return node;
  }

  function tag(className, text) {
    return el('span', 'tag ' + className, text);
  }

  function implTag(status) {
    switch (status) {
      case 'implemented_verified': return tag('tag-implemented', 'implemented');
      case 'implemented_unverified': return tag('tag-unverified', 'implemented · unverified');
      case 'proposed':
      case 'not_started': return tag('tag-proposed', 'proposed');
      default: return tag('tag-proposed', status || 'unknown');
    }
  }

  function deployTag(status) {
    return status === 'deployed'
      ? tag('tag-deployed', 'deployed')
      : tag('tag-notdeployed', status === 'not_deployed' ? 'not deployed' : (status || 'unknown'));
  }

  function verifyTag(status) {
    switch (status) {
      case 'e2e_verified': return tag('tag-e2e', 'e2e verified');
      case 'unit_verified': return tag('tag-unit', 'unit verified');
      default: return tag('tag-proposed', 'not verified');
    }
  }

  function lifecycleTag(lifecycle) {
    return lifecycle === 'active'
      ? tag('tag-implemented', 'active')
      : tag('tag-proposed', lifecycle === 'planned' ? 'planned' : (lifecycle || 'planned'));
  }

  function truncate(s, n) {
    if (!s) return '';
    return s.length > n ? s.slice(0, n - 1) + '…' : s;
  }

  function isExecutable(svc) {
    return svc.deploymentStatus === 'deployed' ||
           svc.implementationStatus === 'implemented_verified' ||
           svc.implementationStatus === 'implemented_unverified';
  }

  function makeCard(svc, catalogKind) {
    var planned = !isExecutable(svc);
    var card;
    if (planned) {
      card = el('div', 'card planned');
      card.setAttribute('aria-disabled', 'true');
    } else {
      card = el('a', 'card');
      card.href = catalogKind === 'frontend'
        ? '/reference/frontend/' + encodeURIComponent(svc.id)
        : '/reference/backend/' + encodeURIComponent(svc.id);
    }

    var top = el('div', 'card-top');
    var nameWrap = el('div');
    nameWrap.appendChild(el('div', 'card-name', svc.name || svc.id));
    nameWrap.appendChild(el('div', 'card-repo', svc.repository || ''));
    top.appendChild(nameWrap);
    var life = lifecycleTag(svc.lifecycle);
    life.classList.add('card-lifecycle');
    top.appendChild(life);
    card.appendChild(top);

    card.appendChild(el('div', 'card-desc', truncate(svc.description, 200)));

    var meta = el('div', 'card-meta');
    meta.appendChild(tag(catalogKind === 'frontend' ? 'tag-frontend' : 'tag-backend',
      catalogKind === 'frontend' ? 'Frontend BFF' : 'Backend API'));
    meta.appendChild(implTag(svc.implementationStatus));
    meta.appendChild(deployTag(svc.deploymentStatus));
    meta.appendChild(verifyTag(svc.verificationStatus));
    if (svc.apiVersion) meta.appendChild(tag('tag-version', 'v' + svc.apiVersion));
    if (typeof svc.operationCount === 'number') {
      meta.appendChild(tag('tag-ops', svc.operationCount + (svc.operationCount === 1 ? ' op' : ' ops')));
    }
    card.appendChild(meta);

    var context = [];
    if (svc.boundedContext) context.push(svc.boundedContext);
    if (svc.channel && catalogKind === 'frontend') context.push('channel: ' + svc.channel);
    if (svc.domainOwner) context.push(svc.domainOwner);
    if (context.length) card.appendChild(el('div', 'card-context', context.join(' · ')));

    card.appendChild(el('div', 'card-cta',
      planned ? 'Not yet available' : 'Open API reference →'));

    return card;
  }

  function makeGroupLabel(text, count) {
    var g = el('div', 'group-label', text);
    if (typeof count === 'number') {
      var suffix = el('span', null, ' · ' + count + (count === 1 ? ' service' : ' services'));
      suffix.style.color = 'var(--agena-text-tertiary)';
      suffix.style.fontWeight = '500';
      g.appendChild(suffix);
    }
    return g;
  }

  function renderSection(node, groups, catalogKind, emptyMsg) {
    if (!node) return;
    node.textContent = '';
    var total = 0;
    (groups || []).forEach(function (group) {
      var svcs = group.services || [];
      total += svcs.length;
      node.appendChild(makeGroupLabel(group.label || group.id, svcs.length));
      var grid = el('div', 'grid');
      svcs.forEach(function (svc) { grid.appendChild(makeCard(svc, catalogKind)); });
      node.appendChild(grid);
    });
    if (total === 0) {
      var box = el('div', 'state-box');
      box.appendChild(el('div', 'state-title', 'Nothing published yet'));
      box.appendChild(el('div', 'state-detail', emptyMsg));
      node.appendChild(box);
    }
    return total;
  }

  function showError(err) {
    state.error = err;
    var message = 'Failed to load the API catalog.';
    var detail = err && err.message ? err.message : 'network error';
    [FRONTEND_GRID, BACKEND_SECTIONS].forEach(function (node) {
      if (!node) return;
      node.textContent = '';
      var box = el('div', 'state-box error');
      box.setAttribute('role', 'alert');
      box.appendChild(el('div', 'state-title', 'Catalog unavailable'));
      box.appendChild(el('div', 'state-detail', message + ' ' + detail));
      var retry = el('button', 'retry', 'Retry');
      retry.type = 'button';
      retry.addEventListener('click', function () { load(); });
      box.appendChild(retry);
      node.appendChild(box);
    });
  }

  function skeleton(node) {
    if (!node) return;
    node.textContent = '';
    var grid = el('div', 'grid');
    for (var i = 0; i < 3; i++) grid.appendChild(el('div', 'skeleton'));
    node.appendChild(grid);
  }

  function load() {
    if (!state.loaded) {
      skeleton(FRONTEND_GRID);
      skeleton(BACKEND_SECTIONS);
    }
    fetch('/api/v1/catalog', {
      headers: { 'Accept': 'application/json' },
      credentials: 'same-origin'
    })
      .then(function (res) {
        if (!res.ok) throw new Error('HTTP ' + res.status);
        return res.json();
      })
      .then(function (catalog) {
        state.loaded = true;
        state.error = null;
        var feTotal = renderSection(FRONTEND_GRID, catalog.frontend, 'frontend',
          'No frontend BFF contracts are published for this environment.');
        var beTotal = renderSection(BACKEND_SECTIONS, catalog.backend, 'backend',
          'No backend domain APIs are published for this environment.');
        setCount('frontend-count', feTotal);
        setCount('backend-count', beTotal);
        var env = document.getElementById('env-name');
        if (env) env.textContent = catalog.environment || 'development';
        var sessionNote = document.getElementById('session-note');
        if (sessionNote) {
          sessionNote.textContent = 'Signed in · ' + (catalog.environment || 'development');
        }
      })
      .catch(function (err) { showError(err); });
  }

  function setCount(id, n) {
    var node = document.getElementById(id);
    if (node) node.textContent = n == null ? '' : String(n);
  }

  /* ---- Session chrome: user chip + logout ---- */

  function initSession() {
    var nameNode = document.getElementById('user-name');
    var logoutBtn = document.getElementById('logout-btn');
    if (logoutBtn) {
      logoutBtn.addEventListener('click', function () {
        logoutBtn.disabled = true;
        fetch('/auth/logout', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Accept': 'application/json' }
        }).then(function () {
          window.location.href = '/auth/login';
        }).catch(function () {
          logoutBtn.disabled = false;
        });
      });
    }
    fetch('/auth/me', { credentials: 'same-origin', headers: { 'Accept': 'application/json' } })
      .then(function (res) { return res.ok ? res.json() : null; })
      .then(function (me) {
        if (me && nameNode) {
          var name = me.displayName || me.name || me.preferred_username || me.email || me.sub;
          if (name) nameNode.textContent = name;
        }
      })
      .catch(function () { /* anonymous or expired — header still renders */ });
  }

  initSession();
  if (FRONTEND_GRID || BACKEND_SECTIONS) load();
})();
