const http = require("http");
const net = require("net");
const path = require("path");
const { spawn } = require("child_process");

const ROOT = path.resolve(__dirname, "..");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

function request(port, pathname) {
  return new Promise((resolve, reject) => {
    const req = http.get(
      {
        host: "127.0.0.1",
        port,
        path: pathname,
        timeout: 1200
      },
      (res) => {
        let body = "";
        res.setEncoding("utf8");
        res.on("data", (chunk) => {
          body += chunk;
        });
        res.on("end", () => {
          resolve({ statusCode: res.statusCode, body });
        });
      }
    );
    req.on("timeout", () => {
      req.destroy(new Error(`Timed out requesting ${pathname}`));
    });
    req.on("error", reject);
  });
}

async function waitForState(port, child, output) {
  let lastError;
  for (let attempt = 0; attempt < 60; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`Server exited before it became ready.\n${output()}`);
    }
    try {
      const response = await request(port, "/api/state");
      if (response.statusCode === 200) return JSON.parse(response.body);
      lastError = new Error(`HTTP ${response.statusCode}`);
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  throw new Error(`Server did not become ready: ${lastError ? lastError.message : "unknown error"}\n${output()}`);
}

function stop(child) {
  return new Promise((resolve) => {
    if (child.exitCode !== null) {
      resolve();
      return;
    }
    const timer = setTimeout(resolve, 1500);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill();
  });
}

async function main() {
  const port = await getFreePort();
  const logs = [];
  const child = spawn(process.execPath, ["server.js"], {
    cwd: ROOT,
    env: { ...process.env, XPCN_PORT: String(port) },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true
  });

  const output = () => logs.join("").trim();
  child.stdout.on("data", (chunk) => logs.push(chunk.toString()));
  child.stderr.on("data", (chunk) => logs.push(chunk.toString()));

  try {
    const state = await waitForState(port, child, output);
    assert(Array.isArray(state.services), "state.services should be an array");
    assert(Array.isArray(state.software), "state.software should be an array");
    assert(Array.isArray(state.configFiles), "state.configFiles should be an array");
    assert(state.configFiles.some((item) => item.id === "php.ini"), "php.ini config entry should exist");

    const homepage = await request(port, "/");
    assert(homepage.statusCode === 200, "homepage should return HTTP 200");
    assert(homepage.body.includes("XP.CN 小皮"), "homepage should contain the product name");
    assert(homepage.body.includes("app.js"), "homepage should load app.js");

    console.log(`Smoke test passed on http://127.0.0.1:${port}`);
  } finally {
    await stop(child);
  }
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exit(1);
});
