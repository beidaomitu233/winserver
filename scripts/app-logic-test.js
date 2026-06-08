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
  return { autoServices, suiteRunning };
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

console.log("App logic test passed");
