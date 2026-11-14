// @ts-check
const { test, expect } = require('@playwright/test');
const path = require('path');

test.describe('Centralized Swagger UI Developer Portal', () => {
  test('renders Swagger UI, verifies dropdown, switches specs, and asserts Try-It-Out is disabled', async ({ page }) => {
    // Collect console errors & network failures
    const consoleErrors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    // 1. Open Swagger portal URL
    await page.goto('/swagger-ui/');

    // 2. Swagger UI renders
    const swaggerContainer = page.locator('#swagger-ui, .swagger-ui');
    await expect(swaggerContainer.first()).toBeVisible();

    // 3. Dropdown is visible in the topbar
    const dropdown = page.locator('.topbar select, .swagger-ui select');
    await expect(dropdown.first()).toBeVisible();

    // 4. Expected service options exist (at least 19 services)
    const options = await dropdown.first().locator('option').allTextContents();
    console.log(`Found ${options.length} options in Swagger UI dropdown:`, options.slice(0, 5));
    expect(options.length).toBeGreaterThanOrEqual(19);

    // Verify key services in dropdown
    const optionText = options.join(' | ');
    expect(optionText).toContain('Agent BFF');
    expect(optionText).toContain('Agent Service');
    expect(optionText).toContain('Agency Service');
    expect(optionText).toContain('Sales Service');

    // 5. Select Agent BFF (Default or explicit select)
    await dropdown.first().selectOption({ label: options.find(o => o.includes('Agent BFF — Implemented')) });
    
    // Wait for operations to render
    const bffOp = page.locator('.opblock-summary-path').filter({ hasText: '/me' });
    await expect(bffOp.first()).toBeVisible();

    // 6. Assert endpoint operations appear
    const todayOp = page.locator('.opblock-summary-path').filter({ hasText: '/me/today' });
    await expect(todayOp.first()).toBeVisible();

    // 7. Click on an operation to expand it
    await bffOp.first().click();

    // 8. Assert Try It Out button is disabled/absent by default (Section 16)
    const tryItOutBtn = page.locator('.try-out__btn');
    await expect(tryItOutBtn).toHaveCount(0);

    // 9. Select Agent Service from dropdown
    const agentOption = options.find(o => o.includes('Agent Service — Implemented'));
    expect(agentOption).toBeTruthy();
    await dropdown.first().selectOption({ label: agentOption });

    // 10. Agent spec loads & operations appear
    const agentOp = page.locator('.opblock-summary-path').filter({ hasText: '/api/v1/agents' });
    await expect(agentOp.first()).toBeVisible();

    // 11. Select Sales Service (Proposed)
    const salesOption = options.find(o => o.includes('Sales Service'));
    expect(salesOption).toBeTruthy();
    await dropdown.first().selectOption({ label: salesOption });

    // Assert sales health probe or endpoints load
    const healthOp = page.locator('.opblock-summary-path').filter({ hasText: '/health/live' });
    await expect(healthOp.first()).toBeVisible();

    // 12. Take screenshot as verification evidence
    const screenshotPath = path.resolve(__dirname, 'swagger-portal-verified.png');
    await page.screenshot({ path: screenshotPath, fullPage: true });
    console.log(`Saved screenshot verification artifact to ${screenshotPath}`);

    // Verify no unhandled critical asset errors
    expect(consoleErrors.filter(e => e.includes('Failed to load resource'))).toHaveLength(0);
  });
});
