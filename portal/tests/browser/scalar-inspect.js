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
    const root = getComputedStyle(document.documentElement);
    const vars = {};
    ['--scalar-color-1','--scalar-color-2','--scalar-color-3','--scalar-color-accent',
     '--scalar-background-1','--scalar-background-2','--scalar-background-card',
     '--scalar-border-color','--scalar-font','--scalar-font-code','--scalar-radius'].forEach(v=>{
      vars[v]=root.getPropertyValue(v).trim();
    });
    // probe real elements
    const pick = (sel) => { const e=document.querySelector(sel); if(!e) return null;
      const c=getComputedStyle(e); return {bg:c.backgroundColor, color:c.color, font:c.fontFamily.slice(0,50), radius:c.borderRadius}; };
    return {vars,
      body: pick('body'),
      h1: pick('h1'),
      sidebar: pick('[class*="sidebar"]'),
      code: pick('code, pre'),
      button: pick('button'),
      scalarRootClasses: document.body.className,
      scalarHTML: (document.querySelector('#api-reference')||{}).outerHTML?.slice(0,100)
    };
  });
  console.log(JSON.stringify(r,null,1));
  await b.close();
})().catch(e=>{console.error(e);process.exit(2)});
