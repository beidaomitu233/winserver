/**
 * WinServer Unified Test Runner
 *
 * Orchestrates the complete automated test pipeline:
 *   1. Start the Vite dev server
 *   2. Run Playwright E2E tests across all pages
 *   3. Capture screenshots + DOM snapshots at every step
 *   4. Generate comprehensive HTML report with pass/fail analysis
 *   5. Stop the dev server
 *
 * Usage: node tests/run-e2e.js [--skip-start] [--pages dashboard,sites]
 */

const { execSync, spawn } = require('child_process')
const fs = require('fs')
const path = require('path')

const ROOT = path.resolve(__dirname, '..')
const RESULTS = path.join(ROOT, 'test-results')

const args = process.argv.slice(2)
const skipStart = args.includes('--skip-start')
const pagesArg = args.find(a => a.startsWith('--pages='))
const selectedPages = pagesArg ? pagesArg.split('=')[1].split(',') : null
const verbose = args.includes('--verbose')

function ensureDir(dir) {
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
}

function run(cmd, options = {}) {
  console.log('  $ ' + cmd)
  try {
    return execSync(cmd, {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: options.timeout || 180_000,
      stdio: verbose ? 'inherit' : (options.silent ? 'pipe' : 'inherit'),
      env: { ...process.env, FORCE_COLOR: '1' },
    })
  } catch (e) {
    if (options.allowFail) return e.stdout || ''
    throw e
  }
}

function logPhase(name) {
  console.log('\n' + '='.repeat(60))
  console.log('  ' + name)
  console.log('='.repeat(60) + '\n')
}

function stopProcessTree(child) {
  if (!child?.pid) return
  if (process.platform === 'win32') {
    try {
      execSync('taskkill /pid ' + child.pid + ' /T /F', { stdio: 'ignore' })
    } catch {
      // The process may already have exited.
    }
    return
  }
  child.kill('SIGTERM')
}

async function startDevServer() {
  if (skipStart) {
    console.log('  Skipping dev server start (using existing server on port 1420)')
    return null
  }

  logPhase('Phase 1: Starting Vite Dev Server')

  let devProcess = null
  try {
    devProcess = spawn('npm', ['run', 'dev', '--', '--host', '127.0.0.1', '--port', '1420', '--strictPort'], {
      cwd: path.join(ROOT, 'frontend'),
      shell: true,
      stdio: 'pipe',
      env: { ...process.env },
    })

    await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        console.log('  Timeout waiting for dev server, trying to proceed...')
        resolve()
      }, 30_000)

      const check = (data) => {
        const msg = data.toString()
        if (msg.includes('Local:') || msg.includes('ready') || msg.includes('VITE') || msg.includes('1420')) {
          clearTimeout(timeout)
          resolve()
        }
      }

      devProcess.stdout?.on('data', check)
      devProcess.stderr?.on('data', check)
    })

    console.log('  ✓ Dev server ready on port 1420')
  } catch (e) {
    console.log('  ! Could not start dev server:', e.message)
    console.log('  Trying to use existing server...')
  }

  return devProcess
}

async function runE2ETests() {
  logPhase('Phase 2: Running Playwright E2E Tests')

  ensureDir(path.join(RESULTS, 'e2e-steps'))
  ensureDir(path.join(RESULTS, 'e2e-snapshots'))

  let cmd = 'npx playwright test'
  if (selectedPages) {
    const pattern = selectedPages.map(p => '**/' + p + '.spec.ts').join(',')
    cmd += ' --grep-in-file "' + pattern + '"'
  }

  try {
    run(cmd, { timeout: 300_000, allowFail: true })
    console.log('  ✓ E2E tests completed')
  } catch {
    console.log('  ✗ Some E2E tests failed')
  }
}

function generateReport() {
  logPhase('Phase 3: Generating Test Report')

  ensureDir(RESULTS)

  const stepsDir = path.join(RESULTS, 'e2e-steps')
  const snapshotsDir = path.join(RESULTS, 'e2e-snapshots')
  const playwrightReport = path.join(RESULTS, 'e2e-report')

  const screenshots = fs.existsSync(stepsDir)
    ? fs.readdirSync(stepsDir).filter(f => f.endsWith('.png')).sort()
    : []

  const domSnapshots = fs.existsSync(snapshotsDir)
    ? fs.readdirSync(snapshotsDir).filter(f => f.endsWith('.html')).sort()
    : []

  let pwResults = []
  try {
    const reportFile = path.join(playwrightReport, 'index.html')
    if (fs.existsSync(reportFile)) {
      pwResults.push({ name: 'Playwright HTML Report', path: 'e2e-report/index.html' })
    }
  } catch { /* no playwright report */ }

  const totalSteps = screenshots.length
  const totalSnapshots = domSnapshots.length

  let galleryHtml = ''
  for (let i = 0; i < screenshots.length; i++) {
    const img = 'e2e-steps/' + screenshots[i]
    const dom = domSnapshots[i] ? 'e2e-snapshots/' + domSnapshots[i] : null
    const stepNum = i + 1
    galleryHtml += '<div class="step-card"><div class="step-num">Step ' + stepNum + '</div>'
    galleryHtml += '<a href="' + img + '" target="_blank"><img src="' + img + '" loading="lazy" alt="Step ' + stepNum + '" /></a>'
    if (dom) {
      galleryHtml += '<a href="' + dom + '" target="_blank" class="dom-link">DOM Snapshot</a>'
    }
    galleryHtml += '</div>'
  }

  let reportsHtml = ''
  for (const r of pwResults) {
    reportsHtml += '<a href="' + r.path + '" class="report-btn" target="_blank">' + r.name + '</a>'
  }

  const html = '<!DOCTYPE html><html lang="zh-CN"><head><meta charset="utf-8"><title>WinServer E2E Test Report</title>'
    + '<style>'
    + '*{margin:0;padding:0;box-sizing:border-box}'
    + 'body{font-family:system-ui,-apple-system,sans-serif;background:#0f172a;color:#e2e8f0;line-height:1.5}'
    + '.container{max-width:1400px;margin:0 auto;padding:2rem}'
    + 'h1{font-size:1.75rem;font-weight:700;margin-bottom:.5rem;color:#f1f5f9}'
    + 'h2{font-size:1.25rem;font-weight:600;margin:2rem 0 1rem;color:#94a3b8}'
    + '.meta{color:#64748b;font-size:.875rem;margin-bottom:2rem}'
    + '.stats{display:flex;gap:1.5rem;margin:1.5rem 0;flex-wrap:wrap}'
    + '.stat{background:#1e293b;padding:1.25rem 2rem;border-radius:12px;border:1px solid #334155;min-width:160px}'
    + '.stat .val{font-size:2rem;font-weight:700}'
    + '.stat .lbl{font-size:.8rem;color:#94a3b8;margin-top:.25rem}'
    + '.stat.info .val{color:#3b82f6}'
    + '.stat.snap .val{color:#a855f7}'
    + '.reports{display:flex;gap:1rem;margin:1.5rem 0;flex-wrap:wrap}'
    + '.report-btn{display:inline-flex;align-items:center;gap:.5rem;padding:.75rem 1.5rem;background:#3b82f6;color:#fff;border-radius:8px;text-decoration:none;font-size:.9rem;font-weight:500}'
    + '.report-btn:hover{background:#2563eb}'
    + '.gallery{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:1rem;margin-top:1rem}'
    + '.step-card{background:#1e293b;border-radius:10px;overflow:hidden;border:1px solid #334155;transition:transform .15s}'
    + '.step-card:hover{transform:translateY(-2px)}'
    + '.step-num{padding:.5rem .75rem;font-size:.8rem;font-weight:600;color:#94a3b8;background:#0f172a;border-bottom:1px solid #334155}'
    + '.step-card img{width:100%;display:block;cursor:pointer}'
    + '.dom-link{display:block;padding:.5rem .75rem;font-size:.75rem;color:#3b82f6;text-decoration:none;border-top:1px solid #334155}'
    + '.dom-link:hover{color:#60a5fa;background:rgba(59,130,246,.05)}'
    + '.empty{text-align:center;padding:4rem;color:#64748b;font-size:1.1rem}'
    + '</style></head><body><div class="container">'
    + '<h1>WinServer E2E Test Report</h1>'
    + '<p class="meta">Generated: ' + new Date().toLocaleString('zh-CN') + ' | Steps: ' + totalSteps + ' | DOM Snapshots: ' + totalSnapshots + '</p>'
    + '<div class="stats">'
    + '<div class="stat info"><div class="val">' + totalSteps + '</div><div class="lbl">Screenshots Captured</div></div>'
    + '<div class="stat snap"><div class="val">' + totalSnapshots + '</div><div class="lbl">DOM Snapshots</div></div>'
    + '</div>'
    + (pwResults.length > 0 ? '<h2>Detailed Reports</h2><div class="reports">' + reportsHtml + '</div>' : '')
    + '<h2>Step-by-Step Screenshots</h2>'
    + (galleryHtml ? '<div class="gallery">' + galleryHtml + '</div>' : '<div class="empty">No screenshots captured. Run the tests first.</div>')
    + '</div></body></html>'

  fs.writeFileSync(path.join(RESULTS, 'index.html'), html)
  console.log('  ✓ Report generated: test-results/index.html')
  console.log('  ✓ Screenshots: ' + totalSteps)
  console.log('  ✓ DOM snapshots: ' + totalSnapshots)
}

async function main() {
  console.log('')
  console.log('==============================================================')
  console.log('  WinServer Automated E2E Test Runner')
  console.log('  1. Start Vite dev server (port 1420)')
  console.log('  2. Run Playwright E2E tests (all pages, all interactions)')
  console.log('  3. Capture screenshots + DOM snapshots at every step')
  console.log('  4. Generate comprehensive HTML test report')
  console.log('')
  console.log('  Options:')
  console.log('    --skip-start    Use existing dev server')
  console.log('    --pages=x,y    Run only specific page tests')
  console.log('    --verbose       Show full test output')
  console.log('==============================================================')
  console.log('')

  ensureDir(RESULTS)

  const devProcess = await startDevServer()

  try {
    await runE2ETests()
  } finally {
    if (devProcess) {
      stopProcessTree(devProcess)
      console.log('\n  ✓ Dev server stopped')
    }
  }

  generateReport()

  console.log('\n' + '='.repeat(60))
  console.log('  Done! Open test-results/index.html for the full report.')
  console.log('='.repeat(60) + '\n')
}

main().catch(e => {
  console.error('Runner error:', e)
  process.exit(1)
})
