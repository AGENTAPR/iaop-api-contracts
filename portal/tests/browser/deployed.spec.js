// Deployed-portal ADL verification: anonymous denial, Keycloak SSO login,
// dual-catalog render in both themes, reference navigation, mobile layout,
// error state, CSP/console cleanliness.
const { chromium } = require('@playwright/test');

const BASE = process.env.PORTAL_URL || 'https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech';
const USER = process.env.DEMO_USER || 'demo.agent.1';
const PASS = process.env.DEMO_PASS;
const OUT = __dirname + '/shots';
const fs = require('fs');
fs.mkdirSync(OUT, { recursive: true });

const consoleErrors = [];

(async () => {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true });
  const page = await ctx.newPage();
  page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text()); });
  page.on('pageerror', e => consoleErrors.push('PAGEERROR: ' + e.message));

  const results = [];
  const check = (name, ok, extra='') => { results.push({name, ok, extra}); console.log((ok?'PASS':'FAIL')+' '+name+(extra?' — '+extra:'')); };

  // 1. Anonymous page navigation auto-redirects into Keycloak login
  const anonResp = await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  // Browser follows 303 → /auth/login → 303 → Keycloak; final page is the
  // Keycloak login form (HTTP 200) — proves the redirect chain works.
  const onKeycloak = /\/realms\/iaop-dev\//.test(page.url()) ||
    (await page.locator('input[name="username"]').count()) > 0;
  check('anonymous / redirects to Keycloak login', onKeycloak, page.url());
  await page.screenshot({ path: OUT + '/01-anon-keycloak.png' });

  // Anonymous API/spec requests still get JSON 401 (no redirect).
  const apiResp = await page.request.get(BASE + '/api/v1/catalog');
  check('anonymous catalog API → 401 JSON', apiResp.status() === 401, 'status=' + apiResp.status());
  const specResp = await page.request.get(BASE + '/specs/development/agent-bff/openapi.json');
  check('anonymous spec → 401 JSON', specResp.status() === 401, 'status=' + specResp.status());

  // Deep-link: anonymous Scalar page navigation redirects with return_to.
  const deep = await page.request.get(BASE + '/reference/frontend/agent-bff', {
    headers: { 'Accept': 'text/html' }, maxRedirects: 0,
  });
  const deepLoc = deep.headers()['location'] || '';
  check('deep link anon → 303 login + return_to',
    deep.status() === 303 && deepLoc.includes('/auth/login?return_to=') && deepLoc.includes('agent-bff'),
    deep.status() + ' ' + deepLoc.slice(0, 120));

  // 2. Complete the Keycloak login already reached via the redirect
  await page.waitForSelector('input[name="username"]', { timeout: 20000 });
  await page.fill('input[name="username"]', USER);
  await page.fill('input[name="password"]', PASS);
  await page.click('#kc-login, input[type="submit"], button[type="submit"]');
  await page.waitForURL(u => u.pathname === '/' || u.pathname.startsWith('/reference'), { timeout: 30000 });
  check('portal SSO login returns to portal', page.url().startsWith(BASE), page.url());

  // 3. Landing page render — light theme
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
  const themeAttr = await page.evaluate(() => document.documentElement.getAttribute('data-theme'));
  check('data-theme initialized', themeAttr === 'light' || themeAttr === 'dark', themeAttr);
  await page.screenshot({ path: OUT + '/02-landing-light.png', fullPage: true });

  // 4. Card content checks
  const bffCard = page.locator('#frontend-groups a.card', { hasText: 'Agent BFF' });
  check('agent-bff in frontend only', await bffCard.count() === 1);
  const agentSvc = page.locator('#backend-sections a.card', { hasText: 'Agent Service' });
  const agencySvc = page.locator('#backend-sections a.card', { hasText: 'Agency Service' });
  check('agent-service backend card', await agentSvc.count() === 1);
  check('agency-service backend card', await agencySvc.count() === 1);
  const custCard = page.locator('#frontend-groups .card.planned', { hasText: 'Customer' });
  check('customer channel shown as planned', await custCard.count() >= 1);

  // 5. Dark theme toggle
  await page.click('[data-theme-toggle]');
  await page.waitForTimeout(300);
  const darkAttr = await page.evaluate(() => document.documentElement.getAttribute('data-theme'));
  check('theme toggle → dark', darkAttr === 'dark', darkAttr);
  await page.screenshot({ path: OUT + '/03-landing-dark.png', fullPage: true });
  await page.click('[data-theme-toggle]');
  await page.waitForTimeout(300);
  const lightAttr = await page.evaluate(() => document.documentElement.getAttribute('data-theme'));
  check('theme toggle → light', lightAttr === 'light', lightAttr);

  // 6. Section screenshots
  await page.locator('#frontend-apis').screenshot({ path: OUT + '/04-frontend-section.png' });
  await page.locator('#backend-apis').screenshot({ path: OUT + '/05-backend-section.png' });

  // 7. Reference navigation — BFF
  await bffCard.first().click();
  await page.waitForLoadState('domcontentloaded');
  await page.waitForTimeout(4000);
  check('bff reference loads', page.url().includes('/reference/frontend/agent-bff'), page.url());
  const scalarMount = await page.locator('#api-reference, script[src*="scalar"]').count();
  check('scalar mount present', scalarMount >= 1);
  const refText = await page.locator('body').innerText().catch(() => '');
  check('scalar rendered content', /agent|api|endpoint|operation/i.test(refText) && refText.length > 200, refText.length + ' chars');
  const refTheme = await page.evaluate(() => document.documentElement.getAttribute('data-theme'));
  check('reference page theme synced', refTheme === 'light' || refTheme === 'dark', refTheme);
  await page.screenshot({ path: OUT + '/06-bff-ref-light.png', fullPage: false });
  await page.locator('.ref-context').screenshot({ path: OUT + '/07-auth-context-bar.png' }).catch(() => {});

  // BFF reference in dark mode — portal theme toggle flips data-theme;
  // ADL scalar-overrides re-skin Scalar's CSS vars live.
  const bgBefore = await page.evaluate(() =>
    getComputedStyle(document.body).getPropertyValue('--scalar-background-1').trim());
  await page.click('[data-theme-toggle]');
  await page.waitForTimeout(600);
  await page.screenshot({ path: OUT + '/08-bff-ref-dark.png', fullPage: false });
  const bgAfter = await page.evaluate(() =>
    getComputedStyle(document.body).getPropertyValue('--scalar-background-1').trim());
  check('scalar surface re-themed', bgBefore !== bgAfter && bgAfter.length > 0, bgBefore + ' → ' + bgAfter);
  await page.click('[data-theme-toggle]');
  await page.waitForTimeout(300);

  // 8. Agency reference
  await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('#backend-sections .card', { timeout: 15000 });
  await agencySvc.first().click();
  await page.waitForLoadState('domcontentloaded');
  await page.waitForTimeout(4000);
  check('agency reference loads', page.url().includes('/reference/backend/agency-service'), page.url());
  await page.screenshot({ path: OUT + '/09-agency-ref.png', fullPage: false });

  // 9. Back navigation
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(800);
  check('back to catalog', page.url() === BASE + '/' || page.url() === BASE + '', page.url());

  // 10. Mobile viewport
  const mob = await ctx.newPage();
  mob.on('pageerror', e => consoleErrors.push('MOBILE PAGEERROR: ' + e.message));
  await mob.setViewportSize({ width: 375, height: 800 });
  await mob.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await mob.waitForSelector('#frontend-groups .card, #frontend-groups .state-box', { timeout: 15000 });
  await mob.waitForTimeout(1200);
  const hScroll = await mob.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 2);
  check('mobile no horizontal scroll', !hScroll, 'scrollW=' + await mob.evaluate(() => document.documentElement.scrollWidth));
  await mob.screenshot({ path: OUT + '/10-mobile-landing.png', fullPage: true });
  await mob.close();

  // 11. Error state — intercept catalog fetch and force failure
  const errPage = await ctx.newPage();
  await errPage.route('**/api/v1/catalog', route => route.fulfill({ status: 500, body: '{}' }));
  await errPage.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await errPage.waitForSelector('.state-box.error', { timeout: 15000 });
  const errText = await errPage.locator('.state-box.error').first().innerText();
  check('error state meaningful', /catalog|unavailable|failed/i.test(errText), errText.slice(0, 80));
  await errPage.screenshot({ path: OUT + '/11-error-state.png', fullPage: true });
  await errPage.close();

  // 12. Console/CSP
  const cspErrs = consoleErrors.filter(e => /Content Security Policy|Refused to|violates/i.test(e));
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
