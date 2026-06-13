/** WinServer E2E - Files Page */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, assertElementVisible, assertTextContent, clickAndRecord, ensureDirs } from '../helpers'

const P = 'Files'

test.describe('Files Page', () => {
  test.beforeEach(async ({ page }) => { ensureDirs(); await page.goto('/'); await waitForAppReady(page); await navigateToPage(page, 'files'); await page.waitForTimeout(800) })
  test('Files loads', async ({ page }) => { await assertElementVisible(page, P, '.page-title', 'Title') })
  test('Files layout', async ({ page }) => { await assertElementVisible(page, P, '.file-layout', 'Layout'); await assertElementVisible(page, P, '.file-sidebar', 'Sidebar'); await assertElementVisible(page, P, '.file-main', 'Main') })
  test('Files sidebar tree', async ({ page }) => { await assertElementVisible(page, P, '.tree-title', 'Tree title'); const items = page.locator('.tree-item'); const count = await items.count(); await recordStep(page, P, 'Tree items', count > 0, 'Found ' + count) })
  test('Files action buttons', async ({ page }) => { const btns = page.locator('.page-actions .btn'); const count = await btns.count(); await recordStep(page, P, 'Action buttons', count > 0, 'Found ' + count) })
})