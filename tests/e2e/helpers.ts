/**
 * WinServer E2E Test Helpers
 */

import { type Page, expect } from '@playwright/test'
import * as fs from 'fs'
import * as path from 'path'

const RESULTS_DIR = path.resolve(__dirname, '../../test-results')
const STEPS_DIR = path.join(RESULTS_DIR, 'e2e-steps')
const SNAPSHOTS_DIR = path.join(RESULTS_DIR, 'e2e-snapshots')

export interface StepLog {
  step: number
  page: string
  action: string
  passed: boolean
  reasoning: string
  screenshot?: string
  domSnapshot?: string
  timestamp: string
  durationMs?: number
}

const stepLog: StepLog[] = []
let stepCounter = 0

export function getStepLog() { return stepLog }
export function resetStepLog() { stepLog.length = 0; stepCounter = 0 }

export function ensureDirs() {
  for (const d of [RESULTS_DIR, STEPS_DIR, SNAPSHOTS_DIR]) {
    if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true })
  }
}

export async function recordStep(
  page: Page,
  pageName: string,
  action: string,
  passed: boolean,
  reasoning: string,
  extra?: { durationMs?: number },
) {
  stepCounter++
  const step = stepCounter
  const tag = String(step).padStart(3, '0')
  const screenshotPath = `test-results/e2e-steps/step-${tag}.png`
  const domPath = `test-results/e2e-snapshots/step-${tag}.html`

  try { await page.screenshot({ path: screenshotPath, fullPage: false }) } catch {}
  try {
    const html = await page.evaluate(() => document.documentElement.outerHTML)
    fs.writeFileSync(domPath, html, 'utf8')
  } catch {}

  const entry: StepLog = {
    step, page: pageName, action, passed, reasoning,
    screenshot: screenshotPath, domSnapshot: domPath,
    timestamp: new Date().toISOString(),
    durationMs: extra?.durationMs,
  }
  stepLog.push(entry)

  const icon = passed ? '\u2713' : '\u2717'
  const dur = extra?.durationMs ? ` (${extra.durationMs}ms)` : ''
  console.log(`  ${icon} [${pageName}] Step ${step}: ${action} \u2014 ${reasoning}${dur}`)
  return entry
}

export async function navigateTo(page: Page, pageKey: string): Promise<boolean> {
  const navBtn = page.locator('button.nav-item').filter({ hasText: new RegExp(pageKey, 'i') })
  const count = await navBtn.count()
  if (count > 0) {
    await navBtn.first().click()
    await page.waitForTimeout(600)
    return true
  }
  return false
}

export const PAGE_KEYS: Record<string, string> = {
  dashboard: '\u9996\u9875',
  sites: '\u7f51\u7ad9',
  database: '\u6570\u636e\u5e93',
  software: '\u8f6f\u4ef6',
  logs: '\u65e5\u5fd7',
  settings: '\u8bbe\u7f6e',
}

export async function navigateToPage(page: Page, key: string): Promise<boolean> {
  const label = PAGE_KEYS[key] || key
  return navigateTo(page, label)
}

export async function waitForAppReady(page: Page) {
  await page.waitForSelector('.app-window', { timeout: 15_000 })
  await page.waitForTimeout(1500)
}

export async function waitForModal(page: Page, timeout = 5000) {
  const modal = page.locator('.overlay.show:visible .modal, .modal:visible')
  await modal.first().waitFor({ state: 'visible', timeout }).catch(() => null)
  return modal.first()
}

export async function closeModal(page: Page) {
  const closeBtn = page.locator('.modal:visible .icon-btn, .modal:visible .btn-icon, .modal:visible button:has-text("\u53d6\u6d88"), .modal:visible button:has-text("\u5173\u95ed")').first()
  if (await closeBtn.isVisible().catch(() => false)) {
    await closeBtn.click()
    await page.waitForTimeout(300)
  } else {
    const overlay = page.locator('.overlay.show').first()
    if (await overlay.isVisible().catch(() => false)) {
      await overlay.click({ position: { x: 5, y: 5 } })
      await page.waitForTimeout(300)
    }
  }
}

export async function assertElementExists(page: Page, pageName: string, selector: string, label: string) {
  const count = await page.locator(selector).count()
  await recordStep(page, pageName, `Check ${label}`, count > 0,
    count > 0 ? `Found ${count} "${label}" elements` : `No "${label}" elements found`)
  return count
}

export async function assertElementVisible(page: Page, pageName: string, selector: string, label: string) {
  const visible = await page.locator(selector).first().isVisible().catch(() => false)
  await recordStep(page, pageName, `Check ${label} visible`, visible,
    visible ? `"${label}" is visible` : `"${label}" is NOT visible`)
  return visible
}

export async function assertTextContent(page: Page, pageName: string, selector: string, expected: string | RegExp, label: string) {
  const el = page.locator(selector).first()
  const text = await el.textContent().catch(() => '')
  const match = typeof expected === 'string' ? text?.includes(expected) : expected.test(text || '')
  await recordStep(page, pageName, `Check ${label} text`, !!match,
    match ? `"${label}" matches` : `"${label}" text is "${text?.slice(0, 80)}"`)
  return !!match
}

export async function clickAndRecord(page: Page, pageName: string, selector: string, label: string) {
  const start = Date.now()
  const el = page.locator(selector).first()
  const exists = await el.isVisible().catch(() => false)
  if (!exists) {
    await recordStep(page, pageName, `Click ${label}`, false, `Element not found: ${selector}`)
    return false
  }
  await el.click()
  const duration = Date.now() - start
  await page.waitForTimeout(400)
  await recordStep(page, pageName, `Click ${label}`, true, `Clicked "${label}"`, { durationMs: duration })
  return true
}

export async function fillAndRecord(page: Page, pageName: string, selector: string, value: string, label: string) {
  const el = page.locator(selector).first()
  const exists = await el.isVisible().catch(() => false)
  if (!exists) {
    await recordStep(page, pageName, `Fill ${label}`, false, `Input not found: ${selector}`)
    return false
  }
  await el.fill(value)
  await recordStep(page, pageName, `Fill ${label}`, true, `Filled "${label}" with "${value}"`)
  return true
}

export async function selectAndRecord(page: Page, pageName: string, selector: string, value: string, label: string) {
  const el = page.locator(selector).first()
  const exists = await el.isVisible().catch(() => false)
  if (!exists) {
    await recordStep(page, pageName, `Select ${label}`, false, `Select not found: ${selector}`)
    return false
  }
  await el.selectOption(value)
  await recordStep(page, pageName, `Select ${label}`, true, `Selected "${value}" for "${label}"`)
  return true
}

export async function checkForToast(page: Page, pageName: string, action: string) {
  const toast = page.locator('.toast-stack .toast').first()
  const hasToast = await toast.isVisible().catch(() => false)
  if (hasToast) {
    const toastText = await toast.textContent().catch(() => '')
    await recordStep(page, pageName, `Toast after ${action}`, true, `Toast: "${toastText?.slice(0, 100)}"`)
    return toastText
  }
  await recordStep(page, pageName, `No toast after ${action}`, true, 'No toast notification appeared')
  return null
}
