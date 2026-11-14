// @ts-check
const { test, expect } = require('@playwright/test');
const path = require('path');

// ---------------------------------------------------------------------------
// IAOP Developer Portal V2 — Scalar API Reference browser tests
//
// These tests verify the portal after the Swagger UI → Scalar migration:
//  - Landing page renders with dual-catalog structure
//  - Scalar API Reference loads correctly for specific services
//  - /swagger-ui/ backward-compat redirect works
//  - Catalog API returns V2 dual structure (no agent-bff-proposed)
//  - No CDN references in served HTML
// ---------------------------------------------------------------------------

test.describe('IAOP Developer Portal V2 — Landing Page', () => {
  test('landing page renders with correct heading and sections', async ({ page }) => {
    const consoleErrors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });

    await page.goto('/');

    // h1 must contain portal title
    await expect(page.locator('h1')).toContainText('IAOP Developer Portal');

    // Both section titles must appear
    await expect(page.getByText('Frontend APIs', { exact: false })).toBeVisible();
    await expect(page.getByText('Backend APIs', { exact: false })).toBeVisible();

    // No unhandled resource errors
    const resourceErrors = consoleErrors.filter(e =>
      e.includes('Failed to load resource') && !e.includes('scalar.min.js')
    );
    expect(resourceErrors).toHaveLength(0);
  });

  test('catalog API populates cards on landing page', async ({ page }) => {
    await page.goto('/');
    // Wait for fetch('/api/v1/catalog') to populate the grid
    await page.waitForFunction(() => {
      const grid = document.getElementById('frontend-grid');
      return grid && grid.children.length > 0;
    }, { timeout: 8000 });

    // At least one frontend card must be visible
    const frontendCards = page.locator('#frontend-grid .card');
    await expect(frontendCards.first()).toBeVisible();

    // agent-bff card must exist
    const bffCard = page.locator('#frontend-grid .card').filter({ hasText: 'Agent BFF' });
    await expect(bffCard.first()).toBeVisible();

    // agent-bff-proposed must NOT appear anywhere
    await expect(page.getByText('agent-bff-proposed', { exact: true })).toHaveCount(0);
  });

  test('clicking agent-bff card navigates to /reference/frontend/agent-bff', async ({ page }) => {
    await page.goto('/');
    // Wait for cards to load
    await page.waitForFunction(() => {
      const grid = document.getElementById('frontend-grid');
      return grid && grid.children.length > 0;
    }, { timeout: 8000 });

    const bffCard = page.locator('#frontend-grid .card').filter({ hasText: 'Agent BFF' }).first();
    const href = await bffCard.getAttribute('href');
    expect(href).toBe('/reference/frontend/agent-bff');
  });
});

test.describe('IAOP Developer Portal V2 — Scalar API Reference views', () => {
  test('renders Scalar API Reference for agent-bff (frontend)', async ({ page }) => {
    const consoleErrors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });

    await page.goto('/reference/frontend/agent-bff');

    // The Scalar mount script tag must be present
    const apiRef = page.locator('#api-reference');
    await expect(apiRef).toBeAttached();

    // The script's data-url must point to agent-bff spec
    const dataUrl = await apiRef.getAttribute('data-url');
    expect(dataUrl).toBe('/specs/development/agent-bff/openapi.json');

    // No CDN URLs in the HTML (all assets must be same-origin)
    const html = await page.content();
    expect(html).not.toContain('cdn.jsdelivr.net');
    expect(html).not.toContain('unpkg.com');

    // Scalar JS loaded from /static/
    const scalarScript = page.locator('script[src="/static/scalar.min.js"]');
    await expect(scalarScript).toBeAttached();

    // Take screenshot as evidence
    const screenshotPath = path.resolve(__dirname, 'scalar-portal-agent-bff.png');
    await page.screenshot({ path: screenshotPath, fullPage: false });
    console.log(`Screenshot saved: ${screenshotPath}`);

    // No critical resource failures
    const criticalErrors = consoleErrors.filter(e =>
      e.includes('Failed to load resource') && !e.includes('scalar.min.js')
    );
    expect(criticalErrors).toHaveLength(0);
  });

  test('renders Scalar API Reference for agent-service (backend)', async ({ page }) => {
    await page.goto('/reference/backend/agent-service');

    const apiRef = page.locator('#api-reference');
    await expect(apiRef).toBeAttached();

    const dataUrl = await apiRef.getAttribute('data-url');
    expect(dataUrl).toBe('/specs/development/agent-service/openapi.json');
  });

  test('/reference/frontend redirects to first frontend service', async ({ page }) => {
    const response = await page.goto('/reference/frontend');
    // After redirect, URL must be /reference/frontend/{id}
    expect(page.url()).toMatch(/\/reference\/frontend\/[a-z0-9-]+$/);
  });

  test('/reference/backend redirects to first backend service', async ({ page }) => {
    await page.goto('/reference/backend');
    expect(page.url()).toMatch(/\/reference\/backend\/[a-z0-9-]+$/);
  });

  test('nonexistent service returns 404', async ({ page }) => {
    const response = await page.goto('/reference/frontend/nonexistent-xyz-9999');
    expect(response.status()).toBe(404);
  });
});

test.describe('IAOP Developer Portal V2 — Backward compat', () => {
  test('/swagger-ui/ redirects to /reference/frontend', async ({ page }) => {
    await page.goto('/swagger-ui/');
    // After redirect, URL must be under /reference/frontend
    expect(page.url()).toContain('/reference/frontend');
  });

  test('/swagger-ui (no trailing slash) redirects to /reference/frontend', async ({ page }) => {
    await page.goto('/swagger-ui');
    expect(page.url()).toContain('/reference/frontend');
  });
});

test.describe('IAOP Developer Portal V2 — Catalog API', () => {
  test('GET /api/v1/catalog returns V2 dual structure', async ({ request }) => {
    const res = await request.get('/api/v1/catalog');
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body).toHaveProperty('frontend');
    expect(body).toHaveProperty('backend');
    expect(body.schemaVersion).toBe(2);

    expect(Array.isArray(body.frontend)).toBe(true);
    expect(Array.isArray(body.backend)).toBe(true);
  });

  test('catalog has exactly one agent-bff entry', async ({ request }) => {
    const res = await request.get('/api/v1/catalog');
    const body = await res.json();

    const allServices = [
      ...(body.frontend || []).flatMap(g => g.services || []),
      ...(body.backend || []).flatMap(g => g.services || []),
    ];

    const bffEntries = allServices.filter(s => s.id === 'agent-bff');
    expect(bffEntries).toHaveLength(1);

    const proposedEntries = allServices.filter(s => s.id === 'agent-bff-proposed');
    expect(proposedEntries).toHaveLength(0);
  });

  test('catalog has no duplicate service IDs', async ({ request }) => {
    const res = await request.get('/api/v1/catalog');
    const body = await res.json();

    const allIds = [
      ...(body.frontend || []).flatMap(g => g.services || []),
      ...(body.backend || []).flatMap(g => g.services || []),
    ].map(s => s.id);

    const unique = new Set(allIds);
    expect(unique.size).toBe(allIds.length);
  });

  test('agent-bff is in frontend catalog groups', async ({ request }) => {
    const res = await request.get('/api/v1/catalog/frontend');
    const body = await res.json();

    const allIds = (body || []).flatMap(g => (g.services || []).map(s => s.id));
    expect(allIds).toContain('agent-bff');
  });

  test('agent-service is in backend catalog groups', async ({ request }) => {
    const res = await request.get('/api/v1/catalog/backend');
    const body = await res.json();

    const allIds = (body || []).flatMap(g => (g.services || []).map(s => s.id));
    expect(allIds).toContain('agent-service');
  });

  test('/api/v1/services/{id} returns ScalarConfig', async ({ request }) => {
    const res = await request.get('/api/v1/services/agent-bff');
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.serviceMeta.id).toBe('agent-bff');
    expect(body.serviceMeta.catalog).toBe('frontend');
    expect(body.spec.url).toMatch(/^\/specs\//);
  });
});

test.describe('IAOP Developer Portal V2 — Health probes', () => {
  test('/health/live returns ok', async ({ request }) => {
    const res = await request.get('/health/live');
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('ok');
  });

  test('/health/ready returns ready with services_count and schema_version', async ({ request }) => {
    const res = await request.get('/health/ready');
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('ready');
    expect(body.services_count).toBeGreaterThanOrEqual(18);
    expect(body.schema_version).toBe(2);
  });
});
