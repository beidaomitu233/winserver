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
const source = fs.readFileSync(path.join(root, "server.js"), "utf8");
const helpers = new Function(`
  ${extractFunction(source, "toSlash")}
  ${extractFunction(source, "normalizePathText")}
  ${extractFunction(source, "parseWmicProcessRows")}
  ${extractFunction(source, "parsePowerShellProcessRows")}
  ${extractFunction(source, "processRowMatchesExecutable")}
  return { parseWmicProcessRows, parsePowerShellProcessRows, processRowMatchesExecutable };
`)();

const wmicRows = helpers.parseWmicProcessRows([
  "Node,ExecutablePath,ProcessId",
  "DEVBOX,C:\\phpstudy_pro\\Extensions\\MySQL8.0.12\\bin\\mysqld.exe,4321",
  "DEVBOX,C:\\Other\\MySQL\\bin\\mysqld.exe,9876",
  ""
].join("\r\n"));

assert(wmicRows.length === 2, "WMIC parser should return process rows");
assert(wmicRows[0].pid === 4321, "WMIC parser should read the process id");
assert(wmicRows[0].executablePath.includes("MySQL8.0.12"), "WMIC parser should read the executable path");

const psRows = helpers.parsePowerShellProcessRows(JSON.stringify([
  { ProcessId: 111, ExecutablePath: "D:\\redis\\redis-server.exe" },
  { ProcessId: 222, ExecutablePath: "D:\\other\\redis-server.exe" }
]));

assert(psRows.length === 2, "PowerShell parser should return array rows");
assert(psRows[1].pid === 222, "PowerShell parser should read the process id");
assert(helpers.parsePowerShellProcessRows("").length === 0, "PowerShell parser should handle empty output");
assert(helpers.parsePowerShellProcessRows("null").length === 0, "PowerShell parser should handle null output");

assert(
  helpers.processRowMatchesExecutable(
    { pid: 4321, executablePath: "D:\\PHPSTUDY_PRO\\Extensions\\MySQL8.0.12\\bin\\mysqld.exe" },
    "d:/phpstudy_pro/extensions/mysql8.0.12/bin/mysqld.exe"
  ),
  "process matching should be case-insensitive and slash-insensitive"
);
assert(
  !helpers.processRowMatchesExecutable(
    { pid: 9876, executablePath: "D:\\Other\\MySQL\\bin\\mysqld.exe" },
    "D:/phpstudy_pro/Extensions/MySQL8.0.12/bin/mysqld.exe"
  ),
  "process matching should reject the same process name from a different path"
);

console.log("Server logic test passed");
