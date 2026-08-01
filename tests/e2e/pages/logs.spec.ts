/** WinServer E2E - Operation Logs Page */

import { test, expect } from '@playwright/test'
import { recordStep, navigateToPage, waitForAppReady, assertElementVisible, fillAndRecord, ensureDirs } from '../helpers'

const P = 'Logs'

test.describe('Logs Page', () => {
  test.beforeEach(async ({ page }) => {
    ensureDirs()
    await page.goto('/')
    await waitForAppReady(page)
    await navigateToPage(page, 'logs')
    await page.waitForTimeout(300)
  })

  test('Only structured operation logs are shown', async ({ page }) => {
    await expect(page.locator('.page-title')).toHaveText('操作日志')
    await expect(page.locator('.tabs, .terminal-body')).toHaveCount(0)
    await expect(page.locator('.operation-row')).toHaveCount(3)
    await assertElementVisible(page, P, '.operation-details', 'Structured request details')
    await expect(page.locator('.operation-list')).not.toContainText('database-secret')
    await recordStep(page, P, 'Structured operation log list', true, '3 operation records')
  })

  test('Operation logs support search and result filtering', async ({ page }) => {
    await fillAndRecord(page, P, '.search-box input', 'app_demo', 'Search')
    await page.keyboard.press('Enter')
    await expect(page.locator('.operation-row')).toHaveCount(1)
    await expect(page.locator('.operation-row')).toContainText('创建数据库')

    await page.locator('.search-box input').clear()
    await page.keyboard.press('Enter')
    await page.locator('.result-filter button').filter({ hasText: '成功' }).click()
    await expect(page.locator('.operation-row')).toHaveCount(3)
  })

  test('Operation logs expose refresh controls without source tabs', async ({ page }) => {
    await assertElementVisible(page, P, '.auto-refresh input', 'Auto refresh toggle')
    await assertElementVisible(page, P, 'button[title="刷新"]', 'Refresh button')
    await assertElementVisible(page, P, 'button[title="清空操作日志"]', 'Clear button')
  })
})
