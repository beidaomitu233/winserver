/**
 * WinServer E2E - Dashboard Page
 */

import { test, expect, type Page } from '@playwright/test'
import {
  recordStep, navigateToPage, waitForAppReady,
  assertElementExists, assertElementVisible, assertTextContent,
  clickAndRecord, checkForToast, closeModal, ensureDirs, waitForModal,
} from '../helpers'

const P = 'Dashboard'

test.describe('Dashboard Page', () => {

  test.beforeEach(async ({ page }) => {
    ensureDirs()
    await page.goto('/')
    await waitForAppReady(page)
  })

  test('Dashboard loads with hero section', async ({ page }) => {
    await assertElementVisible(page, P, '.home-hero', 'Hero section')
    await assertElementVisible(page, P, '.home-title', 'Page title')
  })

  test('Dashboard shows running service count pill', async ({ page }) => {
    const pill = page.locator('.home-state-pill')
    const visible = await pill.isVisible().catch(() => false)
    await recordStep(page, P, 'Service count pill', visible, visible ? 'Pill visible' : 'Not found')
  })

  test('Dashboard shows one-click start button', async ({ page }) => {
    await assertElementVisible(page, P, 'button:has-text("\u4e00\u952e\u542f\u52a8")', 'Start all button')
  })

  test('Dashboard KPI metrics are visible', async ({ page }) => {
    const kpis = page.locator('.home-kpi')
    const count = await kpis.count()
    await recordStep(page, P, 'KPI count', count >= 4, 'Found ' + count + ' KPIs')
    for (let i = 0; i < Math.min(count, 4); i++) {
      const val = await kpis.nth(i).locator('.home-kpi-value').textContent().catch(() => '')
      await recordStep(page, P, 'KPI ' + (i+1), !!val, 'Value: ' + val)
    }
  })

  test('Dashboard quick action buttons exist', async ({ page }) => {
    const quickBtns = page.locator('.home-quick')
    const count = await quickBtns.count()
    await recordStep(page, P, 'Quick actions', count > 0, 'Found ' + count)
  })

  test('Dashboard quick action navigates to sites', async ({ page }) => {
    const btn = page.locator('.home-quick:has-text("\u7f51\u7ad9")').first()
    if (await btn.isVisible().catch(() => false)) {
      await btn.click()
      await page.waitForTimeout(500)
      const isActive = await page.locator('button.nav-item.active').filter({ hasText: /\u7f51\u7ad9/ }).count()
      await recordStep(page, P, 'Quick nav sites', isActive > 0, isActive > 0 ? 'OK' : 'Failed')
    }
  })

  test('Dashboard shows service grid', async ({ page }) => {
    const services = page.locator('.home-service-row')
    const count = await services.count()
    await recordStep(page, P, 'Service list', count > 0, 'Found ' + count)
    for (let i = 0; i < Math.min(count, 10); i++) {
      const svc = services.nth(i)
      const name = await svc.locator('.home-service-name').textContent().catch(() => 'svc-' + i)
      const hasActions = await svc.locator('.home-service-action').count()
      await recordStep(page, P, 'Service ' + name, true, 'actions=' + hasActions)
    }
  })

  test('Dashboard one-click start updates service state', async ({ page }) => {
    await page.locator('button:has-text("一键启动")').first().click()
    await page.waitForTimeout(600)
    const pillText = await page.locator('.home-state-pill').textContent().catch(() => '')
    const updated = /5\s*项服务在线/.test(pillText || '')
    await recordStep(page, P, 'One-click start state', updated, pillText || '')
    expect(updated).toBeTruthy()
  })

  test('Dashboard Redis config supports visual and file modes', async ({ page }) => {
    await page.locator('[data-service="redis"] .home-service-action').filter({ hasText: '配置' }).first().click()
    const modal = await waitForModal(page)
    const visible = await modal.isVisible().catch(() => false)
    await recordStep(page, P, 'Redis config modal', visible, visible ? 'Opened' : 'Missing')
    expect(visible).toBeTruthy()

    await assertElementVisible(page, P, '.service-config-modal input[type="number"]', 'Redis visual port')
    await page.locator('.service-config-modal .config-tab').filter({ hasText: '直接编辑文件' }).click()
    const textarea = page.locator('.service-config-modal textarea').first()
    const hasFileEditor = await textarea.isVisible().catch(() => false)
    const content = hasFileEditor ? await textarea.inputValue().catch(() => '') : ''
    await recordStep(page, P, 'Redis direct file editor', hasFileEditor && content.includes('port 6379'), content.slice(0, 80))
    expect(hasFileEditor).toBeTruthy()
    expect(content).toContain('port 6379')
    await closeModal(page)
  })

  test('Dashboard port quick action checks availability', async ({ page }) => {
    await page.locator('.home-quick').filter({ hasText: '端口' }).click()
    const modal = await waitForModal(page)
    await assertElementVisible(page, P, '.modal:visible input[type="number"]', 'Port input')
    await page.locator('.modal:visible input[type="number"]').fill('18080')
    await page.locator('.modal:visible button:has-text("检测")').click()
    await page.waitForTimeout(300)
    const result = await page.locator('.port-result').textContent().catch(() => '')
    const ok = /18080/.test(result || '') && /可用/.test(result || '')
    await recordStep(page, P, 'Port quick action result', ok, result || '')
    expect(ok).toBeTruthy()
    await closeModal(page)
  })

  test('Dashboard collapse/expand services', async ({ page }) => {
    const toggle = page.locator('.home-section-title-wrap').first()
    if (await toggle.isVisible().catch(() => false)) {
      await toggle.click()
      await page.waitForTimeout(300)
      const isCollapsed = await page.locator('.home-service-grid.collapsed').count()
      await recordStep(page, P, 'Collapse', isCollapsed > 0, isCollapsed > 0 ? 'OK' : 'Not collapsed')
      await toggle.click()
      await page.waitForTimeout(300)
    }
  })

  test('Dashboard resource monitor visible', async ({ page }) => {
    const mini = page.locator('.mini-monitor')
    await assertElementVisible(page, P, '.mini-monitor', 'Resource monitor')
    await recordStep(page, P, 'CPU row', (await mini.locator('.mini-row').filter({ hasText: 'CPU' }).count()) > 0, 'CPU row in mini monitor')
    await recordStep(page, P, 'Memory row', (await mini.locator('.mini-row').filter({ hasText: '内存' }).count()) > 0, 'Memory row in mini monitor')
    await recordStep(page, P, 'Disk row', (await mini.locator('.mini-row').filter({ hasText: '磁盘' }).count()) > 0, 'Disk row in mini monitor')
  })

  test('Dashboard resource values numeric', async ({ page }) => {
    const rows = page.locator('.mini-monitor .mini-row')
    const count = await rows.count()
    for (let i = 0; i < count; i++) {
      const text = await rows.nth(i).textContent().catch(() => '')
      await recordStep(page, P, 'resource pct ' + (i + 1), /\d+%/.test(text || ''), text || '')
    }
  })

  test('Dashboard sidebar mini monitor', async ({ page }) => {
    const mini = page.locator('.mini-monitor')
    const visible = await mini.isVisible().catch(() => false)
    await recordStep(page, P, 'Mini monitor', visible, visible ? 'Visible' : 'Missing')
    if (visible) {
      const rows = mini.locator('.mini-row')
      await recordStep(page, P, 'Mini rows', (await rows.count()) >= 3, 'Found ' + await rows.count())
    }
  })

  test('Dashboard log preview section', async ({ page }) => {
    const logSection = page.locator('.home-log-side')
    const visible = await logSection.isVisible().catch(() => false)
    await recordStep(page, P, 'Log preview', visible, visible ? 'Visible' : 'Missing')
    if (visible) {
      const count = await logSection.locator('.home-log-row').count()
      await recordStep(page, P, 'Log entries', true, 'Found ' + count)
    }
  })
})
