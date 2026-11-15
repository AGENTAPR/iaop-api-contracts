// Deployed-portal verification: anonymous denial, Keycloak SSO login,
// dual-catalog landing render, CSP/console cleanliness, reference navigation.
const { chromium } = require('@playwright/test');

const BASE = process.env.PORTAL_URL || 'https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech';
const USER = process.env.DEMO_USER || 'demo.agent.1';
const PASS = process.env.DEMO_PASS;
const OUT = __dirname + '/shots';
const fs = require('fs');
fs.mkdirSync(OUT, { recursive: true });

const consoleErrors = [];
const cspViolations = [];

(async () => {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true });
  const page = await ctx.newPage();
  page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text()); });
  page.on('pageerror', e => consoleErrors.push('PAGEERROR: ' + e.message));

  const results = [];
  const check = (name, ok, extra='') => { results.push({name, ok, extra}); console.log((ok?'PASS':'FAIL')+' '+name+(extra?' — '+extra:'')); };

  // 1. Anonymous access
  const anon = await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  check('anonymous / denied (401)', anon.status() === 401, 'status=' + anon.status());
  await page.screenshot({ path: OUT + '/01-anon.png' });

  // 2. Portal login flow
  await page.goto(BASE + '/auth/login', { waitUntil: 'domcontentloaded' });
  // Keycloak login form
  await page.waitForSelector('input[name="username"]', { timeout: 20000 });
  await page.fill('input[name="username"]', USER);
  await page.fill('input[name="password"]', PASS);
  await page.click('#kc-login, input[type="submit"], button[type="submit"]');
  await page.waitForURL(u => u.pathname === '/' || u.pathname.startsWith('/reference'), { timeout: 30000 });
  check('portal SSO login', page.url().startsWith(BASE), page.url());

  // 3. Landing page render
  await page.waitForSelector('#frontend-groups .card, #frontend-groups .state-box', { timeout: 15000 });
  await page.waitForTimeout(1500);
  const feCards = await page.locator('#frontend-groups .card').count();
  const beCards = await page.locator('#backend-sections .card').count();
  const feGroups = await page.locator('#frontend-groups .group-label').allTextContents();
  const beGroups = await page.locator('#backend-sections .group-label').allTextContents();
  check('frontend cards rendered', feCards >= 2, feCards + ' cards');
  check('backend cards rendered', beCards >= 10, beCards + ' cards');
  console.log('  FE groups:', JSON.stringify(feGroups));
  console.log('  BE groups:', JSON.stringify(beGroups.slice(0,8)));
  await page.screenshot({ path: OUT + '/02-landing.png', fullPage: true });

  // 4. Card content checks
  const bffCard = page.locator('#frontend-groups a.card', { hasText: 'Agent BFF' });
  check('agent-bff in frontend only', await bffCard.count() === 1);
  const agentSvc = page.locator('#backend-sections a.card', { hasText: 'Agent Service' });
  const agencySvc = page.locator('#backend-sections a.card', { hasText: 'Agency Service' });
  check('agent-service backend card', await agentSvc.count() === 1);
  check('agency-service backend card', await agencySvc.count() === 1);
  const custCard = page.locator('#frontend-groups .card.planned', { hasText: 'Customer' });
  check('customer channel shown as planned', await custCard.count() >= 1);

  // 5. Reference navigation
  await bffCard.first().click();
  await page.waitForLoadState('domcontentloaded');
  await page.waitForTimeout(4000);
  check('bff reference loads', page.url().includes('/reference/frontend/agent-bff'), page.url());
  const scalarMount = await page.locator('#api-reference, script[src*="scalar"]').count();
  check('scalar mount present', scalarMount >= 1);
  const refText = await page.locator('body').innerText().catch(() => '');
  check('scalar rendered content', /agent|api|endpoint|operation/i.test(refText) && refText.length > 200, refText.length + ' chars');
  await page.screenshot({ path: OUT + '/03-bff-ref.png' });

  await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('#backend-sections .card', { timeout: 15000 });
  await agencySvc.first().click();
  await page.waitForLoadState('domcontentloaded');
  await page.waitForTimeout(4000);
  check('agency reference loads', page.url().includes('/reference/backend/agency-service'), page.url());
  await page.screenshot({ path: OUT + '/04-agency-ref.png' });

  // 6. Back navigation
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(800);
  check('back to catalog', page.url() === BASE + '/' || page.url() === BASE + '', page.url());

  // 7. Console/CSP
  const cspErrs = consoleErrors.filter(e => /Content Security Policy|Refused to|violates/i.test(e));
  // "Failed to load resource" console errors are HTTP statuses (incl. the
  // expected anonymous 401), not JS exceptions — pageerror covers real throws.
  const jsErrs = consoleErrors.filter(e => !e.startsWith('Failed to load resource') && !/Content Security Policy|Refused to|violates|favicon/i.test(e));
  check('no CSP violations', cspErrs.length === 0, cspErrs.slice(0,3).join(' | '));
  check('no JS exceptions', jsErrs.length === 0, jsErrs.slice(0,3).join(' | '));

  console.log('\n=== SUMMARY ===');
  const fails = results.filter(r => !r.ok);
  console.log(results.length + ' checks, ' + fails.length + ' failed');
  if (consoleErrors.length) console.log('console errors:', consoleErrors.slice(0,8));
  await browser.close();
  process.exit(fails.length ? 1 : 0);
})().catch(e => { console.error('FATAL', e); process.exit(2); });
