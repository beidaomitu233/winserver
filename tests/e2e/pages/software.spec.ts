/** WinServer E2E - Software Page */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, closeModal, assertElementVisible, clickAndRecord, fillAndRecord, selectAndRecord, checkForToast, ensureDirs } from '../helpers'

const P = 'Software'

test.describe('Software Page', () => {
  test.beforeEach(async ({ page }) => { ensureDirs(); await page.goto('/'); await waitForAppReady(page); await navigateToPage(page, 'software'); await page.waitForTimeout(800) })
  test('Software loads', async ({ page }) => { await assertElementVisible(page, P, '.page-title', 'Title') })
  test('Software import button', async ({ page }) => { await assertElementVisible(page, P, 'button.btn.primary', 'Import') })
  test('Software category tabs', async ({ page }) => { const tabs = page.locator('.tab'); const count = await tabs.count(); await recordStep(page, P, 'Tabs', count > 0, 'Found ' + count); for (let i = 0; i < Math.min(count, 5); i++) { await tabs.nth(i).click(); await page.waitForTimeout(200) } })
  test('Software list items', async ({ page }) => { const items = page.locator('.software-row'); const count = await items.count(); await recordStep(page, P, 'Items', count > 0, 'Found ' + count) })
  test('Software installed badges', async ({ page }) => { const tags = page.locator('.installed-tag'); const count = await tags.count(); await recordStep(page, P, 'Installed tags', true, 'Found ' + count) })
  test('Software import modal', async ({ page }) => { await clickAndRecord(page, P, 'button.btn.primary', 'Import'); const modal = page.locator('.overlay.show:visible .modal, .modal:visible').first(); const v = await modal.isVisible().catch(() => false); await recordStep(page, P, 'Modal', v, v ? 'Opened' : 'Not found'); if (v) { await assertElementVisible(page, P, '.modal:visible select', 'Type select'); await closeModal(page) } })
  test('Software search', async ({ page }) => { await fillAndRecord(page, P, '.search-box input', 'test-sw', 'Search'); await page.locator('.search-box input').clear() })
})
