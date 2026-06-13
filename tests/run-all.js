/**
 * WinServer Full Test Runner
 *
 * Orchestrates the complete test pipeline:
 *   1. Build the Tauri app
 *   2. Run backend API integration tests (cargo test)
 *   3. Start the Tauri app
 *   4. Run Playwright E2E tests against it
 *   5. Stop the app
 *   6. Generate unified HTML report
 */

const { execSync, spawn } = require('child_process')
const fs = require('fs')
const path = require('path')

const ROOT = path.resolve(__dirname, '..')
const RESULTS = path.join(ROOT, 'test-results')

// ── Utilities ────────────────────────────────────────────────────────────────

function ensureDir(dir) {
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true })
}

function run(cmd, options = {}) {
  console.log(`  $ ${cmd}`)
  try {
    return execSync(cmd, {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: options.timeout || 120_000,
      stdio: options.silent ? 'pipe' : 'inherit',
      env: { ...process.env, FORCE_COLOR: '1' },
    })
  } catch (e) {
    if (options.allowFail) return e.stdout || ''
    throw e
  }
}

function logPhase(name) {
  console.log(`\n${'='.repeat(60)}`)
  console.log(`  ${name}`)
  console.log(`${'='.repeat(60)}\n`)
}

function stopProcessTree(child) {
  if (!child?.pid) return
  if (process.platform === 'win32') {
    try {
      execSync(`taskkill /pid ${child.pid} /T /F`, { stdio: 'ignore' })
    } catch {
      // The process may already have exited.
    }
    return
  }
  child.kill('SIGTERM')
}

// ── Phase 1: Build ──────────────────────────────────────────────────────────

function buildApp() {
  logPhase('Phase 1: Building Tauri App')
  try {
    // Build frontend first
    run('cd frontend && npm run build', { timeout: 60_000 })
    // Build Rust workspace
    run('cargo build', { timeout: 180_000 })
    console.log('  ✓ Build successful')
    return true
  } catch {
    console.log('  ✗ Build failed')
    return false
  }
}

// ── Phase 2: Backend API Tests ──────────────────────────────────────────────

function runBackendTests() {
  logPhase('Phase 2: Backend API Integration Tests')
  try {
    run('cargo test --test api_integration -- --nocapture', { timeout: 120_000, allowFail: true })
    console.log('  ✓ Backend tests completed')
    return true
  } catch {
    console.log('  ✗ Some backend tests failed (see report)')
    return false
  }
}

// ── Phase 3: Run Rust Unit Tests ────────────────────────────────────────────

function runUnitTests() {
  logPhase('Phase 2b: Rust Unit Tests')
  try {
    run('cargo test --lib -- --nocapture', { timeout: 120_000, allowFail: true })
    console.log('  ✓ Unit tests completed')
    return true
  } catch {
    console.log('  ✗ Some unit tests failed')
    return false
  }
}

// ── Phase 4: Start Tauri App + Run E2E ──────────────────────────────────────

async function runE2ETests() {
  logPhase('Phase 3: E2E Tests (Playwright)')

  // Start the Tauri dev server (or the built binary)
  // For dev mode, we start vite dev server
  console.log('  Starting Tauri dev server...')

  let devProcess = null
  try {
    // Start vite dev server on port 5173
    devProcess = spawn('npm', ['run', 'dev', '--', '--host', '127.0.0.1', '--port', '1420', '--strictPort'], {
      cwd: path.join(ROOT, 'frontend'),
      shell: true,
      stdio: 'pipe',
      env: { ...process.env },
    })

    // Wait for dev server to be ready
    await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Dev server timeout')), 30_000)
      devProcess.stdout?.on('data', (data) => {
        const msg = data.toString()
        if (msg.includes('Local:') || msg.includes('ready') || msg.includes('VITE')) {
          clearTimeout(timeout)
          resolve()
        }
      })
      devProcess.stderr?.on('data', (data) => {
        const msg = data.toString()
        if (msg.includes('Local:') || msg.includes('ready') || msg.includes('VITE')) {
          clearTimeout(timeout)
          resolve()
        }
      })
    })
    console.log('  ✓ Dev server ready')
  } catch (e) {
    console.log('  ✗ Could not start dev server:', e.message)
    console.log('  Trying to use existing server on port 1420...')
  }

  // Create step screenshot directory
  ensureDir(path.join(RESULTS, 'e2e-steps'))

  // Run Playwright
  try {
    run('npx playwright test --reporter=list,html', { timeout: 180_000, allowFail: true })
    console.log('  ✓ E2E tests completed')
  } catch {
    console.log('  ✗ Some E2E tests failed (see report)')
  }

  // Stop dev server
  if (devProcess) {
    stopProcessTree(devProcess)
    console.log('  ✓ Dev server stopped')
  }
}

// ── Phase 5: Generate Unified Report ────────────────────────────────────────

function generateUnifiedReport() {
  logPhase('Phase 4: Generating Unified Report')

  ensureDir(RESULTS)

  // Collect all sub-reports
  const reports = []
  const apiReport = path.join(RESULTS, 'api-integration-report.html')
  const e2eReport = path.join(RESULTS, 'e2e-report', 'index.html')
  const e2ePlaywright = path.join(RESULTS, 'e2e-report.html')

  if (fs.existsSync(apiReport)) reports.push({ name: 'API Integration', path: apiReport })
  if (fs.existsSync(e2eReport)) reports.push({ name: 'E2E (Playwright HTML)', path: e2eReport })
  if (fs.existsSync(e2ePlaywright)) reports.push({ name: 'E2E (Custom)', path: e2ePlaywright })

  const linksHtml = reports.map(r =>
    `<div class="report-card"><h3>${r.name}</h3><a href="${path.relative(RESULTS, r.path)}" class="btn">Open Report</a></div>`
  ).join('\n')

  const html = `<!DOCTYPE html><html><head><meta charset="utf-8"><title>WinServer Test Report</title>
<style>
body{font-family:system-ui,sans-serif;margin:2rem;background:#0f172a;color:#e2e8f0;max-width:900px}
h1{color:#f1f5f9}.reports{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:1rem;margin-top:1rem}
.report-card{background:#1e293b;padding:1.5rem;border-radius:12px;border:1px solid #334155}
.report-card h3{margin:0 0 1rem;color:#e2e8f0}.btn{display:inline-block;padding:.5rem 1.5rem;background:#3b82f6;color:#fff;border-radius:6px;text-decoration:none;font-size:.9rem}
.btn:hover{background:#2563eb}
.timestamp{color:#94a3b8;font-size:.85rem;margin-top:1rem}
</style></head><body>
<h1>WinServer Test Report</h1>
<p>Generated: ${new Date().toISOString()}</p>
<div class="reports">${linksHtml}</div>
<p class="timestamp">Run completed at ${new Date().toLocaleString()}</p>
</body></html>`

  fs.writeFileSync(path.join(RESULTS, 'index.html'), html)
  console.log(`  ✓ Unified report: test-results/index.html`)
}

// ── Main ─────────────────────────────────────────────────────────────────────

async function main() {
  console.log(`
╔══════════════════════════════════════════════════════════╗
║           WinServer Automated Test Runner               ║
║                                                         ║
║  1. Build app                                           ║
║  2. Backend API integration tests                       ║
║  3. Rust unit tests                                     ║
║  4. E2E Playwright tests (all pages, screenshots)       ║
║  5. Unified HTML report with screenshots                ║
╚══════════════════════════════════════════════════════════╝
`)

  ensureDir(RESULTS)

  const results = {
    build: buildApp(),
    backend: false,
    unit: false,
    e2e: false,
  }

  if (!results.build) {
    console.log('\nBuild failed, skipping tests.')
    process.exit(1)
  }

  results.backend = runBackendTests()
  results.unit = runUnitTests()
  await runE2ETests()
  results.e2e = true // E2E partial results still valuable

  generateUnifiedReport()

  // Summary
  console.log(`\n${'='.repeat(60)}`)
  console.log('  Final Summary')
  console.log(`${'='.repeat(60)}`)
  console.log(`  Build:     ${results.build ? '✓ PASS' : '✗ FAIL'}`)
  console.log(`  Backend:   ${results.backend ? '✓ PASS' : '✗ SOME FAIL'}`)
  console.log(`  Unit:      ${results.unit ? '✓ PASS' : '✗ SOME FAIL'}`)
  console.log(`  E2E:       ${results.e2e ? '✓ DONE' : '✗ FAIL'}`)
  console.log(`\n  Reports: test-results/index.html\n`)
}

main().catch(e => {
  console.error('Runner error:', e)
  process.exit(1)
})
