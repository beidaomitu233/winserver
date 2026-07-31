/** WinServer E2E - Database Page */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, closeModal, assertElementVisible, clickAndRecord, fillAndRecord, checkForToast, ensureDirs } from '../helpers'

const P = 'Database'

test.describe('Database Page', () => {
  test.beforeEach(async ({ page }) => { ensureDirs(); await page.goto('/'); await waitForAppReady(page); await navigateToPage(page, 'database'); await page.waitForTimeout(800) })
  test('Database loads', async ({ page }) => { await assertElementVisible(page, P, '.page-title', 'Title') })
  test('Database create button', async ({ page }) => { await assertElementVisible(page, P, 'button.btn.primary', 'Create') })
  test('Database Root button', async ({ page }) => { await assertElementVisible(page, P, 'button:has-text("Root")', 'Root pw') })
  test('Database toolbar', async ({ page }) => { await assertElementVisible(page, P, '.search-box input', 'Search') })
  test('Database table rows', async ({ page }) => { const rows = page.locator('.data-table tbody tr'); const count = await rows.count(); await recordStep(page, P, 'DB rows', true, 'Found ' + count) })
  test('Database create modal', async ({ page }) => { await clickAndRecord(page, P, 'button.btn.primary', 'Create'); const modal = page.locator('.overlay.show:visible .modal, .modal:visible').first(); const v = await modal.isVisible().catch(() => false); await recordStep(page, P, 'Modal', v, v ? 'Opened' : 'Not found'); if (v) { await assertElementVisible(page, P, '.modal:visible input[type="password"]', 'Password'); await closeModal(page) } })
  test('Database root password modal', async ({ page }) => { await clickAndRecord(page, P, 'button:has-text("Root")', 'Root'); const modal = page.locator('.overlay.show:visible .modal, .modal:visible').first(); const v = await modal.isVisible().catch(() => false); await recordStep(page, P, 'Root modal', v, v ? 'Opened' : 'Not found'); if (v) { await closeModal(page) } })
  test('Database search', async ({ page }) => { await fillAndRecord(page, P, '.search-box input', 'test-db', 'Search'); await page.locator('.search-box input').clear() })
  test('Database information copies as connection template', async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'])
    await page.locator('button[title="复制数据库信息"]').first().click()
    const clipboard = await page.evaluate(() => navigator.clipboard.readText())
    expect(clipboard).toBe('数据库名：app_demo\n用户名：app_demo\n密码：demo_pass')
    await recordStep(page, P, 'Copy database template', true, clipboard.replaceAll('\n', ' | '))
  })
})
