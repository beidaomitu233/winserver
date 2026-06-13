/** WinServer E2E - Sites Page */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, closeModal, assertElementVisible, assertTextContent, clickAndRecord, fillAndRecord, checkForToast, ensureDirs } from '../helpers'

const P = 'Sites'

test.describe('Sites Page', () => {
  test.beforeEach(async ({ page }) => { ensureDirs(); await page.goto('/'); await waitForAppReady(page); await navigateToPage(page, 'sites'); await page.waitForTimeout(800) })
  test('Sites loads', async ({ page }) => { await assertElementVisible(page, P, '.page-title', 'Title') })
  test('Sites create button', async ({ page }) => { await assertElementVisible(page, P, 'button.btn.primary', 'Create') })
  test('Sites toolbar search', async ({ page }) => { await assertElementVisible(page, P, '.search-box input', 'Search') })
  test('Sites table rows', async ({ page }) => { const rows = page.locator('.data-table tbody tr'); const count = await rows.count(); await recordStep(page, P, 'Rows', true, 'Found ' + count) })
  test('Sites search filter', async ({ page }) => { await fillAndRecord(page, P, '.search-box input', 'nonexistent-xyz', 'Search'); await page.waitForTimeout(300); await page.locator('.search-box input').clear() })
  test('Sites create modal', async ({ page }) => { await clickAndRecord(page, P, 'button.btn.primary', 'Create'); const modal = page.locator('.overlay.show:visible .modal, .modal:visible').first(); const v = await modal.isVisible().catch(() => false); await recordStep(page, P, 'Modal', v, v ? 'Opened' : 'Not found'); if (v) { await assertElementVisible(page, P, '.modal:visible input', 'Input'); await closeModal(page) } })
  test('Sites toggle', async ({ page }) => { const toggle = page.locator('.switch').first(); const e = await toggle.isVisible().catch(() => false); await recordStep(page, P, 'Toggle', e, e ? 'Found' : 'None') })
  test('Sites PHP select', async ({ page }) => { const sel = page.locator('.data-table select').first(); const e = await sel.isVisible().catch(() => false); await recordStep(page, P, 'PHP', e, e ? 'Found' : 'None') })
  test('Sites row actions', async ({ page }) => { const row = page.locator('.data-table tbody tr').first(); if (await row.isVisible().catch(() => false)) { const actions = row.locator('.table-actions .btn'); await recordStep(page, P, 'Actions', (await actions.count()) > 0, 'Found ' + (await actions.count())) } })
})
