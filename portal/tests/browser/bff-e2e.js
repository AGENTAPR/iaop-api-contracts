const { chromium } = require('@playwright/test');
const BASE = 'https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech';
const results = [];
const check = (n, ok, x='') => { results.push(ok); console.log((ok?'PASS':'FAIL')+' '+n+(x?' — '+x:'')); };
(async () => {
  const b = await chromium.launch({headless:true});
  const ctx = await b.newContext({ignoreHTTPSErrors:true});
  const p = await ctx.newPage();

  // 1. Portal login
  await p.goto(BASE+'/', {waitUntil:'domcontentloaded'});
  await p.waitForSelector('input[name="username"]',{timeout:30000});
  await p.fill('input[name="username"]','demo.agent.1');
  await p.fill('input[name="password"]',process.env.DEMO_PASS);
  await p.click('#kc-login, input[type="submit"], button[type="submit"]');
  await p.waitForURL(u=>u.pathname==='/',{timeout:30000});
  check('portal login', p.url().startsWith(BASE));

  // 2. BFF login bootstrap — must NOT be 403 anymore. Capture the first
  //    response (a redirect into Keycloak) since page.goto follows the
  //    whole OIDC chain to its final destination.
  let bffLoginStatus = 0, bffLoginLoc = '';
  p.on('response', r => {
    if (r.url().includes('/agent-bff/auth/login') && !bffLoginStatus) {
      bffLoginStatus = r.status();
      bffLoginLoc = r.headers()['location'] || '';
    }
  });
  await p.goto(BASE+'/agent-bff/auth/login?redirect_uri='+encodeURIComponent(BASE+'/agent-bff/auth/callback'), {waitUntil:'domcontentloaded'});
  check('bff login bootstrap redirects (not 403)', bffLoginStatus >= 300 && bffLoginStatus < 400, 'status='+bffLoginStatus);
  check('bff login → keycloak authorize', /openid-connect\/auth/.test(bffLoginLoc), bffLoginLoc.slice(0,110));

  // 3. Keycloak may auto-continue (SSO session) or show form
  try {
    await p.waitForSelector('input[name="username"]',{timeout:8000});
    await p.fill('input[name="username"]','demo.agent.1');
    await p.fill('input[name="password"]',process.env.DEMO_PASS);
    await p.click('#kc-login, input[type="submit"], button[type="submit"]');
  } catch(e) { /* SSO session auto-approved */ }

  // 4. Land on BFF callback → session cookie → redirect back
  await p.waitForURL(u=>u.pathname.includes('/agent-bff/auth/callback')||u.pathname==='/'||u.pathname.includes('agent-bff'),{timeout:30000}).catch(()=>{});
  await p.waitForTimeout(3000);
  console.log('  after callback url:', p.url().slice(0,110));

  // 5. BFF session inspection
  const sess = await p.evaluate(async()=>{
    const r = await fetch('/agent-bff/auth/session',{credentials:'same-origin'});
    return {status:r.status, body: await r.text()};
  });
  check('bff /auth/session authenticated', sess.status===200 && /"authenticated"\s*:\s*true/.test(sess.body), sess.status+' '+sess.body.slice(0,120));

  // 6. BFF API execution (tester role)
  const me = await p.evaluate(async()=>{
    const r = await fetch('/agent-bff/api/v1/me',{credentials:'same-origin'});
    return {status:r.status, body:(await r.text()).slice(0,200)};
  });
  check('bff /api/v1/me authorized', me.status===200, me.status+' '+me.body.slice(0,100));

  // 7. Cookies present, no JWT in storage
  const cookies = await ctx.cookies(BASE);
  const bffC = cookies.find(c=>c.name==='__Host-iaop_session');
  check('bff session cookie set', !!bffC && bffC.httpOnly, bffC?bffC.name+' httpOnly='+bffC.httpOnly:'missing');
  const ls = await p.evaluate(()=>JSON.stringify({ls:Object.keys(localStorage),ss:Object.keys(sessionStorage)}));
  check('no tokens in web storage', !/eyJ|token|jwt/i.test(ls), ls.slice(0,120));

  // 8. Connect panel on the BFF reference page reflects both identities
  await p.goto(BASE+'/reference/frontend/agent-bff',{waitUntil:'domcontentloaded'});
  await p.waitForSelector('#bff-connect .cp-card',{timeout:15000});
  const panel = await p.evaluate(()=>document.getElementById('bff-connect').innerText);
  check('connect panel shows developer + connected agent',
    /Developer Portal/.test(panel) && /Agent BFF/.test(panel) && /connected/.test(panel),
    panel.replace(/\n+/g,' | ').slice(0,140));

  // 9. Disconnect via CSRF-protected logout, then verify denial
  const sessBody = JSON.parse(sess.body);
  const csrf = sessBody.csrf_token || '';
  const out = await p.evaluate(async(t)=>{
    const r = await fetch('/agent-bff/auth/logout',{method:'POST',credentials:'same-origin',
      headers:{'Accept':'application/json','x-csrf-token':t}});
    return r.status;
  }, csrf);
  check('bff logout with csrf', out===200, 'status='+out);
  const after = await p.evaluate(async()=>{
    const a = await fetch('/agent-bff/api/v1/me',{credentials:'same-origin'});
    const s = await fetch('/agent-bff/auth/session',{credentials:'same-origin'});
    return {api:a.status, sess: await s.text()};
  });
  check('bff api after logout → 401', after.api===401, 'status='+after.api);
  check('bff session reports unauthenticated', /"authenticated"\s*:\s*false/.test(after.sess));

  // 10. Negative: fresh context, portal session but hit BFF API without BFF session
  const ctx2 = await b.newContext({ignoreHTTPSErrors:true});
  const p2 = await ctx2.newPage();
  await p2.goto(BASE+'/',{waitUntil:'domcontentloaded'});
  await p2.waitForSelector('input[name="username"]',{timeout:30000});
  await p2.fill('input[name="username"]','demo.agent.1');
  await p2.fill('input[name="password"]',process.env.DEMO_PASS);
  await p2.click('#kc-login, input[type="submit"], button[type="submit"]');
  await p2.waitForURL(u=>u.pathname==='/',{timeout:30000});
  const noSess = await p2.evaluate(async()=>{
    const r = await fetch('/agent-bff/api/v1/me',{credentials:'same-origin'});
    return r.status;
  });
  check('bff api without bff session → 401', noSess===401, 'status='+noSess);
  await ctx2.close();

  const fails = results.filter(r=>!r).length;
  console.log('\n'+results.length+' checks, '+fails+' failed');
  await b.close(); process.exit(fails?1:0);
})().catch(e=>{console.error('FATAL',e);process.exit(2)});
