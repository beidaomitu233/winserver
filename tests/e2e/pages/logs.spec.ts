/** WinServer E2E - Logs Page */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, assertElementVisible, clickAndRecord, fillAndRecord, checkForToast, ensureDirs } from '../helpers'

const P = 'Logs'

test.describe('Logs Page', () => {
  test.beforeEach(async ({ page }) => { ensureDirs(); await page.goto('/'); await waitForAppReady(page); await navigateToPage(page, 'logs'); await page.waitForTimeout(800) })
  test('Logs loads', async ({ page }) => { await assertElementVisible(page, P, '.page-title', 'Title') })
  test('Logs source tabs', async ({ page }) => { const tabs = page.locator('.tab'); const count = await tabs.count(); await recordStep(page, P, 'Tabs', count > 0, 'Found ' + count); for (let i = 0; i < Math.min(count, 5); i++) { await tabs.nth(i).click(); await page.waitForTimeout(300) } })
  test('Logs display area', async ({ page }) => { const term = page.locator('.terminal-body'); const v = await term.isVisible().catch(() => false); await recordStep(page, P, 'Terminal', v, v ? 'Visible' : 'Missing') })
  test('Logs auto-refresh toggle', async ({ page }) => { const btn = page.locator('button').filter({ hasText: /自动刷新|手动刷新/ }).first(); const e = await btn.isVisible().catch(() => false); await recordStep(page, P, 'Auto-refresh', e, e ? 'Found' : 'Missing'); if (e) { await btn.click(); await page.waitForTimeout(200); await btn.click() } })
  test('Logs manual refresh', async ({ page }) => { const btn = page.locator('.page-actions button').filter({ hasText: /^刷新$/ }).first(); if (await btn.isVisible().catch(() => false)) { await btn.click(); await page.waitForTimeout(500); await recordStep(page, P, 'Refreshed', true, 'OK') } })
  test('Logs search', async ({ page }) => { await fillAndRecord(page, P, '.search-box input', 'error', 'Search'); await page.keyboard.press('Enter'); await page.waitForTimeout(500); await page.locator('.search-box input').clear() })
  test('Logs clear button', async ({ page }) => { const btn = page.locator('button.btn.danger').first(); const e = await btn.isVisible().catch(() => false); await recordStep(page, P, 'Clear button', e, e ? 'Found' : 'Missing') })
})
