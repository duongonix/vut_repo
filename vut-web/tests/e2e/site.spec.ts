import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import type { Page } from '@playwright/test';

async function resetViewportForScreenshot(page: Page) {
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    window.scrollTo({ top: 0, behavior: 'instant' });
  });
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBe(0);
}

test('homepage assets, responsive layout, tabs, theme, and accessibility', async ({
  page
}, testInfo) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1 })).toContainText('Build a brighter');
  const hero = await page.request.get('/images/hero.webp');
  expect(hero.ok()).toBeTruthy();
  expect((await page.request.get('/og.png')).ok()).toBeTruthy();
  const tab = page.getByRole('tab', { name: 'Simple', exact: true });
  await tab.click();
  await tab.press('End');
  await expect(page.getByRole('tab', { name: 'Powerful', exact: true })).toBeFocused();
  await expect(page.getByRole('tabpanel')).toContainText('square(7)');
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)
  ).toBeTruthy();
  await resetViewportForScreenshot(page);
  await page.screenshot({ path: testInfo.outputPath('homepage-dark.png'), fullPage: true });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.getByRole('button', { name: /Switch to light/i }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  expect(errors).toEqual([]);
});

test('docs headings, copy, navigation, and accessibility', async ({
  page,
  context,
  isMobile
}, testInfo) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('/docs/language/functions/');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Functions');
  await page.getByRole('button', { name: 'Copy code', exact: true }).first().click();
  await expect(page.locator('[data-copy-code]').first()).toHaveText('Copied');
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain('fn greet');
  const heading = page.locator('#return-values');
  await heading.hover();
  await heading.getByRole('link').click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain('#return-values');
  if (isMobile) {
    await page.getByText('On this page', { exact: true }).first().click();
    await expect(
      page
        .getByRole('navigation', { name: 'On this page' })
        .first()
        .getByRole('link', { name: 'Return values' })
    ).toBeVisible();
    await page.getByRole('button', { name: 'Open navigation' }).click();
    const drawer = page.getByRole('dialog', { name: 'Navigation', exact: true });
    await expect(drawer).toBeVisible();
    expect(await page.locator('body').evaluate((body) => getComputedStyle(body).overflow)).toBe(
      'hidden'
    );
    await page.keyboard.press('Escape');
    await expect(drawer).not.toBeVisible();
    await expect(page.getByRole('button', { name: 'Open navigation' })).toBeFocused();
    await page.getByRole('button', { name: 'Open navigation' }).click();
    await drawer.getByRole('link', { name: 'Loops', exact: true }).click();
    await expect(drawer).not.toBeVisible();
  } else {
    await page.locator('.sidebar').getByRole('link', { name: 'Loops', exact: true }).click();
  }
  await expect(page).toHaveURL(/\/docs\/language\/loops\/$/);
  await expect(page.getByRole('heading', { name: 'Loops', exact: true })).toBeVisible();
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)
  ).toBeTruthy();
  await resetViewportForScreenshot(page);
  await page.screenshot({ path: testInfo.outputPath('docs-dark.png'), fullPage: true });
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

test('real Pagefind search, keyboard selection, empty results, and Escape', async ({ page }) => {
  await page.goto('/');
  await page.keyboard.press('Control+k');
  const dialog = page.getByRole('dialog', { name: 'Search documentation' });
  await expect(dialog).toBeVisible();
  const query = page.getByRole('combobox', { name: 'Search query' });
  await expect(query).toBeFocused();
  await query.fill('propagation');
  await expect(page.getByRole('option').first()).toBeVisible();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await query.press('ArrowDown');
  await query.press('Enter');
  await expect(dialog).not.toBeVisible();
  await expect(page).toHaveURL(/\/docs\//);
  await page.getByRole('button', { name: 'Search documentation', exact: true }).click();
  await query.fill('zzzzunfindablewordzzzz');
  await expect(dialog.getByRole('status')).toContainText('No results');
  await query.press('Escape');
  await expect(dialog).not.toBeVisible();
});

test('playground is honest and editable; error route is useful', async ({ page }) => {
  await page.goto('/playground/');
  const editor = page.getByRole('textbox', { name: /Vut source/i });
  await expect(editor).toBeVisible();
  await editor.fill('fn main():\n  out("Edited")');
  await expect(editor).toHaveValue(/Edited/);
  await expect(page.getByRole('button', { name: /Run/i })).toBeDisabled();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.goto('/missing-page/');
  await expect(page.getByRole('heading', { level: 1 })).toContainText("doesn't exist");
  await expect(page.getByRole('link', { name: /Back to Docs/ })).toBeVisible();
});

test('small phone layout and reusable documentation tabs', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 720 });
  for (const path of ['/', '/docs/reference/builtin-types/', '/playground/']) {
    await page.goto(path);
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
      path
    ).toBeTruthy();
  }
  await page.goto('/docs/getting-started/installation/');
  await page.getByRole('tab', { name: 'Windows', exact: true }).click();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tabpanel')).toContainText('macOS-compatible');
});
