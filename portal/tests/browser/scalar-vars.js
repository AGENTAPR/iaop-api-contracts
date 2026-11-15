const { chromium } = require('@playwright/test');
const BASE = 'https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech';
(async () => {
  const b = await chromium.launch({headless:true});
  const ctx = await b.newContext({ignoreHTTPSErrors:true});
  const p = await ctx.newPage();
  await p.goto(BASE+'/auth/login'); await p.waitForSelector('input[name="username"]');
  await p.fill('input[name="username"]','demo.agent.1'); await p.fill('input[name="password"]',process.env.DEMO_PASS);
  await p.click('#kc-login, input[type="submit"], button[type="submit"]');
  await p.waitForURL(u=>u.pathname==='/',{timeout:30000});
  await p.goto(BASE+'/reference/frontend/agent-bff');
  await p.waitForTimeout(5000);
  const r = await p.evaluate(() => {
    // find elements that define --scalar-color-1
    const found = {};
    document.querySelectorAll('*').forEach(el => {
      const cs = getComputedStyle(el);
      const v = cs.getPropertyValue('--scalar-color-1').trim();
      if (v && !found[v]) {
        found[v] = el.tagName + '.' + (el.className||'').toString().slice(0,60);
      }
    });
    // dump all scalar vars on body
    const bodyCS = getComputedStyle(document.body);
    const all = {};
    for (const sheet of document.styleSheets) {}
    // collect var names from a deep element
    const deep = document.querySelector('main, [class*="scalar"], #api-reference') || document.body;
    const dcs = getComputedStyle(deep);
    // can't enumerate computed custom props directly; read stylesheet rules instead
    const names = new Set();
    const walk = (rules) => { for (const r of rules) {
      if (r.cssRules) walk(r.cssRules);
      else if (r.style) for (const prop of r.style) if (prop.startsWith('--scalar')) names.add(prop);
    }};
    for (const sh of document.styleSheets) { try { walk(sh.cssRules); } catch(e){} }
    const vals = {};
    [...names].sort().forEach(n => { const v = dcs.getPropertyValue(n).trim(); if(v) vals[n]=v; });
    return {color1Owners: found, deepTag: deep.tagName+'.'+(deep.className||'').toString().slice(0,50), vals};
  });
  console.log(JSON.stringify(r,null,1).slice(0,6000));
  await b.close();
})().catch(e=>{console.error(e);process.exit(2)});
