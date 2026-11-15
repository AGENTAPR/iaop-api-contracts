/* Agent BFF connection panel — rendered on frontend BFF reference pages.
   Shows the two independent identity contexts honestly:
     1. Developer Portal identity (portal_session) + API-testing permission.
     2. Agent BFF session (__Host-iaop_session) created by the BFF's own
        Keycloak OIDC flow — never confused with the developer identity.
   All state comes from authoritative endpoints; nothing is faked. */
(function () {
  'use strict';

  var mount = document.getElementById('bff-connect');
  if (!mount) return;

  function el(tag, className, text) {
    var n = document.createElement(tag);
    if (className) n.className = className;
    if (text !== undefined && text !== null) n.textContent = String(text);
    return n;
  }

  function row(label) {
    var r = el('div', 'cp-row');
    r.appendChild(el('span', 'cp-label', label));
    return r;
  }

  function statusPill(text, kind) {
    return el('span', 'tag ' + kind, text);
  }

  function fetchJson(url, opts) {
    return fetch(url, Object.assign({ credentials: 'same-origin', headers: { 'Accept': 'application/json' } }, opts || {}))
      .then(function (r) { return r.json().catch(function () { return null; }).then(function (j) { return { status: r.status, body: j }; }); });
  }

  function render(dev, bff) {
    mount.textContent = '';
    var card = el('div', 'cp-card');
    card.setAttribute('role', 'region');
    card.setAttribute('aria-label', 'Agent connection');

    var title = el('div', 'cp-title', 'Agent connection');
    card.appendChild(title);

    // --- Developer identity ---
    var devRow = row('Developer Portal');
    if (dev && dev.status === 200 && dev.body) {
      devRow.appendChild(el('span', 'cp-value', dev.body.displayName || dev.body.subject || 'authenticated'));
      devRow.appendChild(statusPill(dev.body.apiTester ? 'API testing: allowed' : 'API testing: not allowed',
        dev.body.apiTester ? 'tag-implemented' : 'tag-proposed'));
    } else {
      devRow.appendChild(statusPill('session unavailable', 'tag-notdeployed'));
    }
    card.appendChild(devRow);

    // --- Agent BFF identity ---
    var bffRow = row('Agent BFF');
    var connected = !!(bff && bff.status === 200 && bff.body && bff.body.authenticated);
    if (connected) {
      var name = (bff.body.user && (bff.body.user.display_name || bff.body.user.subject)) || 'agent session';
      bffRow.appendChild(el('span', 'cp-value', name));
      bffRow.appendChild(statusPill('connected', 'tag-implemented'));
      bffRow.appendChild(statusPill('agent session', 'tag-e2e'));
    } else {
      bffRow.appendChild(statusPill('not connected', 'tag-notdeployed'));
    }
    card.appendChild(bffRow);

    // --- Actions ---
    var actions = el('div', 'cp-actions');
    var check = el('button', 'cp-btn', 'Check connection');
    check.type = 'button';
    check.addEventListener('click', function () { load(); });
    actions.appendChild(check);

    if (connected) {
      var disc = el('button', 'cp-btn danger', 'Disconnect agent');
      disc.type = 'button';
      disc.addEventListener('click', function () {
        disc.disabled = true;
        var csrf = bff.body.csrf_token || '';
        fetch('/agent-bff/auth/logout', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Accept': 'application/json', 'x-csrf-token': csrf }
        }).then(function () { load(); }).catch(function () { disc.disabled = false; });
      });
      actions.appendChild(disc);
    } else {
      var conn = el('a', 'cp-btn primary', 'Connect agent identity');
      conn.href = '/agent-bff/auth/login?redirect_uri=' +
        encodeURIComponent(location.origin + '/agent-bff/auth/callback') +
        '&return_to=' + encodeURIComponent(location.pathname + location.search);
      actions.appendChild(conn);
    }
    card.appendChild(actions);

    var note = el('div', 'cp-note',
      'Agent sign-in uses Keycloak independently of your developer session. ' +
      'If your browser already has a Keycloak session for a different account, ' +
      'use a separate browser profile to authenticate as the test agent. ' +
      'API execution additionally requires the portal-api-tester role.');
    card.appendChild(note);

    mount.appendChild(card);
  }

  function load() {
    mount.textContent = '';
    mount.appendChild(el('div', 'cp-loading', 'Checking sessions…'));
    Promise.all([
      fetchJson('/auth/me'),
      fetchJson('/agent-bff/auth/session')
    ]).then(function (results) { render(results[0], results[1]); })
      .catch(function () {
        mount.textContent = '';
        mount.appendChild(el('div', 'cp-note', 'Unable to load connection state.'));
      });
  }

  load();
})();
