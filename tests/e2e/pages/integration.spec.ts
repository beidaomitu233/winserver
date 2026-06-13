/** WinServer E2E - Cross-Page Integration */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, assertElementVisible, clickAndRecord, checkForToast, ensureDirs } from '../helpers'

const P = 'Integration'

test.describe('Cross-Page Integration', () => {
  test.beforeEach(async ({ page }) => { ensureDirs(); await page.goto('/'); await waitForAppReady(page) })
  test('App shell structure', async ({ page }) => { await assertElementVisible(page, P, '.sidebar', 'Sidebar'); await assertElementVisible(page, P, '.workspace', 'Workspace') })
  test('Sidebar brand', async ({ page }) => { await assertElementVisible(page, P, '.brand', 'Brand'); await assertElementVisible(page, P, '.brand-mark', 'WS mark') })
  test('Sidebar nav items', async ({ page }) => { const items = page.locator('button.nav-item'); const count = await items.count(); await recordStep(page, P, 'Nav items', count >= 7, 'Found ' + count) })
  test('Navigate all pages sequentially', async ({ page }) => { const pages = ['dashboard', 'sites', 'database', 'software', 'files', 'logs', 'settings']; let allOK = true; for (const p of pages) { const ok = await navigateToPage(page, p); await page.waitForTimeout(400); await recordStep(page, P, 'Nav ' + p, ok, ok ? 'OK' : 'Failed'); if (!ok) allOK = false } expect(allOK).toBeTruthy() })
  test('Back-and-forth navigation', async ({ page }) => { await navigateToPage(page, 'sites'); await page.waitForTimeout(400); await navigateToPage(page, 'dashboard'); await page.waitForTimeout(400); await navigateToPage(page, 'sites'); const title = page.locator('.page-title'); const v = await title.isVisible().catch(() => false); await recordStep(page, P, 'Back-forth', v, v ? 'Stable' : 'Broken') })
  test('Toast container exists', async ({ page }) => { const c = page.locator('.toast-stack, #toastStack'); const e = await c.count(); await recordStep(page, P, 'Toast container', e > 0, e > 0 ? 'Found' : 'Missing') })
  test('Active nav matches page', async ({ page }) => { for (const p of ['dashboard', 'sites', 'database']) { await navigateToPage(page, p); await page.waitForTimeout(400); const active = page.locator('button.nav-item.active'); const count = await active.count(); await recordStep(page, P, 'Active nav ' + p, count > 0, count > 0 ? 'OK' : 'No active') } })
})