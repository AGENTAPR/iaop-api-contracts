const { chromium } = require('@playwright/test');
const BASE = 'https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech';
const results = [];
const check = (n, ok, x='') => { results.push(ok); console.log((ok?'PASS':'FAIL')+' '+n+(x?' — '+x:'')); };
(async () => {
  const b = await chromium.launch({headless:true});
  const ctx = await b.newContext({ignoreHTTPSErrors:true});
  const p = await ctx.newPage();

  // portal login
  await p.goto(BASE+'/', {waitUntil:'domcontentloaded'});
  await p.waitForSelector('input[name="username"]',{timeout:30000});
  await p.fill('input[name="username"]','demo.agent.1');
  await p.fill('input[name="password"]',process.env.DEMO_PASS);
  await p.click('#kc-login, input[type="submit"], button[type="submit"]');
  await p.waitForURL(u=>u.pathname==='/',{timeout:30000});
  check('portal login', p.url().startsWith(BASE));

  // Scalar reference page — Sign out button must work there too
  await p.goto(BASE+'/reference/frontend/agent-bff',{waitUntil:'domcontentloaded'});
  await p.waitForSelector('#logout-btn',{timeout:15000});
  await p.click('#logout-btn');
  await p.waitForURL(u=>u.pathname.startsWith('/auth/login')||u.hostname.includes('8080'),{timeout:20000}).catch(()=>{});
  check('sign out on Scalar page redirects to login', /\/auth\/login|openid-connect/.test(p.url()), p.url().slice(0,110));

  // session must be gone: return to portal origin, catalog API must not serve data
  await p.goto(BASE+'/',{waitUntil:'domcontentloaded'});
  await p.waitForTimeout(1500);
  const st = await p.evaluate(async()=>{
    const r = await fetch('/api/v1/catalog',{credentials:'same-origin'});
    return r.status;
  });
  check('portal session invalidated after sign out', st===401, 'status='+st+' url='+p.url().slice(0,80));

  const fails = results.filter(r=>!r).length;
  console.log('\n'+results.length+' checks, '+fails+' failed');
  await b.close(); process.exit(fails?1:0);
})().catch(e=>{console.error('FATAL',e);process.exit(2)});
