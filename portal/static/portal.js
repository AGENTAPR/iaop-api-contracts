/* IAOP Developer Portal — catalog renderer.
   Served same-origin at /static/portal.js (CSP: script-src 'self').
   All catalog data is rendered via DOM APIs — no innerHTML, no injection. */
(function () {
  'use strict';

  var FRONTEND_GRID = document.getElementById('frontend-groups');
  var BACKEND_SECTIONS = document.getElementById('backend-sections');
  if (!FRONTEND_GRID && !BACKEND_SECTIONS) return;

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
      case 'implemented_verified': return tag('tag-implemented', 'implemented ✓');
      case 'implemented_unverified': return tag('tag-unverified', 'implemented');
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
      : tag('tag-proposed', lifecycle || 'planned');
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
    top.appendChild(lifecycleTag(svc.lifecycle));
    card.appendChild(top);

    card.appendChild(el('div', 'card-desc', truncate(svc.description, 180)));

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

    return card;
  }

  function makeGroupLabel(text, count) {
    var g = el('div', 'group-label', text);
    if (typeof count === 'number') g.appendChild(el('span', null, ' · ' + count + ' services'));
    return g;
  }

  function renderFrontend(groups) {
    if (!FRONTEND_GRID) return;
    FRONTEND_GRID.textContent = '';
    var total = 0;
    (groups || []).forEach(function (group) {
      var svcs = group.services || [];
      total += svcs.length;
      FRONTEND_GRID.appendChild(makeGroupLabel(group.label || group.id, svcs.length));
      var grid = el('div', 'grid');
      svcs.forEach(function (svc) { grid.appendChild(makeCard(svc, 'frontend')); });
      FRONTEND_GRID.appendChild(grid);
    });
    updateCount('frontend-count', total);
    if (total === 0) {
      FRONTEND_GRID.appendChild(el('div', 'state-box', 'No frontend APIs published for this environment.'));
    }
  }

  function renderBackend(groups) {
    if (!BACKEND_SECTIONS) return;
    BACKEND_SECTIONS.textContent = '';
    var total = 0;
    (groups || []).forEach(function (group) {
      var svcs = group.services || [];
      total += svcs.length;
      BACKEND_SECTIONS.appendChild(makeGroupLabel(group.label || group.id, svcs.length));
      var grid = el('div', 'grid');
      svcs.forEach(function (svc) { grid.appendChild(makeCard(svc, 'backend')); });
      BACKEND_SECTIONS.appendChild(grid);
    });
    updateCount('backend-count', total);
    if (total === 0) {
      BACKEND_SECTIONS.appendChild(el('div', 'state-box', 'No backend APIs published for this environment.'));
    }
  }

  function updateCount(id, n) {
    var node = document.getElementById(id);
    if (node) node.textContent = n;
  }

  function showError(err) {
    state.error = err;
    [FRONTEND_GRID, BACKEND_SECTIONS].forEach(function (node) {
      if (!node) return;
      node.textContent = '';
      var box = el('div', 'state-box error');
      box.appendChild(el('div', null, 'Failed to load the API catalog: ' + (err && err.message ? err.message : 'network error')));
      var retry = el('button', 'retry', 'Retry');
      retry.type = 'button';
      retry.addEventListener('click', function () { load(); });
      box.appendChild(retry);
      node.appendChild(box);
    });
  }

  function load() {
    if (!state.loaded) {
      [FRONTEND_GRID, BACKEND_SECTIONS].forEach(function (node) {
        if (!node) return;
        node.textContent = '';
        for (var i = 0; i < 3; i++) node.appendChild(el('div', 'skeleton'));
      });
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
        renderFrontend(catalog.frontend);
        renderBackend(catalog.backend);
        var sessionNote = document.getElementById('session-note');
        if (sessionNote) {
          sessionNote.textContent = 'Signed in · ' + (catalog.environment || 'development');
        }
      })
      .catch(function (err) { showError(err); });
  }

  load();
})();
