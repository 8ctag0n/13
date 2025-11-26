import { test, expect } from '@playwright/test';

test('ultra simple - just load landing', async ({ page }) => {
  // Just load the landing page
  await page.goto('/');
  
  // Wait for load
  await page.waitForLoadState('networkidle');
  
  // Wait 3 seconds
  await page.waitForTimeout(3000);
  
  // Take screenshot
  await page.screenshot({ path: 'test-results/debug-landing.png' });
  
  // Get page title
  const title = await page.title();
  console.log('Title:', title);
  
  // Check if app div exists and has content
  const appContent = await page.evaluate(() => {
    const app = document.getElementById('app');
    return {
      exists: !!app,
      innerHTML: app?.innerHTML.substring(0, 100),
      hasChildren: app?.children.length || 0
    };
  });
  console.log('App div:', appContent);
  
  // This should pass if page loads
  expect(title).toBeTruthy();
});
