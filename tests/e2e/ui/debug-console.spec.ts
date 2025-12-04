import { test } from '@playwright/test';

test('capture console errors', async ({ page }) => {
  const errors: string[] = [];
  
  page.on('console', msg => {
    if (msg.type() === 'error') {
      console.log('BROWSER ERROR:', msg.text());
      errors.push(msg.text());
    }
  });
  
  page.on('pageerror', error => {
    console.log('PAGE ERROR:', error.message);
    errors.push(error.message);
  });
  
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(2000);
  
  console.log('Total errors:', errors.length);
  errors.forEach((err, i) => console.log(`Error ${i+1}:`, err));
});
