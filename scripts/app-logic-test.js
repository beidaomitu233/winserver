const fs = require("fs");
const path = require("path");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function extractFunction(source, name) {
  const start = source.indexOf(`function ${name}(`);
  if (start < 0) throw new Error(`Missing function ${name}`);
  let depth = 0;
  let seenBody = false;
  for (let index = start; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
      seenBody = true;
    }
    if (char === "}") {
      depth -= 1;
      if (seenBody && depth === 0) return source.slice(start, index + 1);
    }
  }
  throw new Error(`Could not extract function ${name}`);
}

const root = path.resolve(__dirname, "..");
const source = fs.readFileSync(path.join(root, "app.js"), "utf8");
const helpers = new Function(`
  ${extractFunction(source, "autoServices")}
  ${extractFunction(source, "suiteRunning")}
  ${extractFunction(source, "softwareInstallState")}
  return { autoServices, suiteRunning, softwareInstallState };
`)();

const services = [
  { name: "Apache", auto: true, running: false },
  { name: "MySQL", auto: true, running: false },
  { name: "Redis", auto: false, running: true }
];

assert(helpers.autoServices(services).length === 2, "autoServices should only include suite members");
assert(helpers.suiteRunning(services) === false, "suite should not be running when only a non-suite service runs");
services[0].running = true;
assert(helpers.suiteRunning(services) === true, "suite should be running when any suite member runs");

assert(helpers.softwareInstallState({ installed: true }).text === "已安装", "installed software should show installed state");
assert(helpers.softwareInstallState({ installed: false, installable: true }).canInstall === true, "installable software should be clickable");
const missingDownload = helpers.softwareInstallState({ installed: false, installable: false, installNote: "配置下载地址" });
assert(missingDownload.canInstall === false, "software without download URL should not be clickable");
assert(missingDownload.text === "需配置", "software without download URL should show configuration state");
assert(missingDownload.title === "配置下载地址", "software configuration state should expose install note");

console.log("App logic test passed");
