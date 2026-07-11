/** WinServer E2E - Settings Page */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, assertElementVisible, assertTextContent, clickAndRecord, fillAndRecord, checkForToast, ensureDirs } from '../helpers'

const P = 'Settings'

test.describe('Settings Page', () => {
  test.beforeEach(async ({ page }) => { ensureDirs(); await page.goto('/'); await waitForAppReady(page); await navigateToPage(page, 'settings'); await page.waitForTimeout(800) })
  test('Settings loads', async ({ page }) => { await assertElementVisible(page, P, '.page-title', 'Title'); await assertElementVisible(page, P, '.settings-layout', 'Layout'); await assertElementVisible(page, P, '.settings-menu', 'Menu') })
  test('Settings menu sections', async ({ page }) => { const items = page.locator('.settings-menu-item'); const count = await items.count(); await recordStep(page, P, 'Menu items', count >= 5, 'Found ' + count) })
  test('Settings general toggles', async ({ page }) => { const toggles = page.locator('.setting-row .switch'); const count = await toggles.count(); await recordStep(page, P, 'General toggles', count > 0, 'Found ' + count) })
  test('Settings network section', async ({ page }) => { await clickAndRecord(page, P, '.settings-menu-item:nth-child(2)', 'Network'); await page.waitForTimeout(300); await assertElementVisible(page, P, 'button:has-text("检测端口")', 'Port check') })
  test('Settings security section', async ({ page }) => { await clickAndRecord(page, P, '.settings-menu-item:nth-child(3)', 'Security'); await page.waitForTimeout(300) })
  test('Settings backup section', async ({ page }) => { await clickAndRecord(page, P, '.settings-menu-item:nth-child(4)', 'Backup'); await page.waitForTimeout(300); await assertElementVisible(page, P, 'button:has-text("导出")', 'Export') })
  test('Settings appearance dark mode', async ({ page }) => { await clickAndRecord(page, P, '.settings-menu-item:nth-child(5)', 'Appearance'); await page.waitForTimeout(300); const toggle = page.locator('.setting-row .switch').first(); if (await toggle.isVisible().catch(() => false)) { const before = await page.evaluate(() => document.documentElement.dataset.theme); await toggle.click(); await page.waitForTimeout(300); const after = await page.evaluate(() => document.documentElement.dataset.theme); await recordStep(page, P, 'Dark toggle', before !== after, before + ' -> ' + after); await toggle.click() } })
  test('Settings save button', async ({ page }) => { await assertElementVisible(page, P, 'button:has-text("保存")', 'Save') })
})
