const fs = require('fs')
const path = require('path')

const ROOT = path.resolve(__dirname, '..')
const TAURI_DIR = path.join(ROOT, 'frontend', 'src-tauri')
const TAURI_CONFIG = path.join(TAURI_DIR, 'tauri.conf.json')

function fail(message) {
  console.error('  ✗ ' + message)
  process.exitCode = 1
}

function pass(message) {
  console.log('  ✓ ' + message)
}

function readText(file) {
  return fs.readFileSync(file, 'utf8').replace(/\r\n/g, '\n')
}

function verifyTauriResources() {
  const config = JSON.parse(readText(TAURI_CONFIG))
  const resources = config.bundle?.resources || []
  if (!Array.isArray(resources) || resources.length === 0) {
    fail('tauri.conf.json has no bundle.resources entries')
    return
  }

  for (const resource of resources) {
    const absolute = path.resolve(TAURI_DIR, resource)
    if (!fs.existsSync(absolute)) {
      fail(`missing bundled resource: ${resource}`)
      continue
    }
    pass(`resource exists: ${resource}`)
  }
}

function verifyRunScripts() {
  const redisRun = path.join(ROOT, 'runtime', 'redis-7.2.4', 'run.bat')
  const minioRun = path.join(ROOT, 'runtime', 'minio', 'run.bat')

  const redis = readText(redisRun)
  if (!redis.includes('cd /d "%~dp0"') || !redis.includes('redis-server.exe redis.conf')) {
    fail('redis run.bat must start redis-server.exe redis.conf from its own directory')
  } else {
    pass('redis run.bat uses portable relative startup')
  }

  const minio = readText(minioRun)
  const hasPortableData = minio.includes('"%~dp0data"')
  const hasEnvFile = minio.includes('MINIO_CONFIG_ENV_FILE=%~dp0minio.env')
  const hasHardcodedLocalPath = /[A-Z]:\\minio/i.test(minio)
  if (!hasPortableData || !hasEnvFile || hasHardcodedLocalPath) {
    fail('minio run.bat must use %~dp0 data/env paths and avoid hardcoded local paths')
  } else {
    pass('minio run.bat uses portable relative data and env paths')
  }
}

console.log('Runtime resource verification')
console.log('=============================')
verifyTauriResources()
verifyRunScripts()

if (process.exitCode) {
  process.exit(process.exitCode)
}
