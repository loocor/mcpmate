import { spawn } from "node:child_process";

const args = process.argv.slice(2);
const commandArgs = args.length > 0 ? args : ["test"];

const env = { ...process.env };
delete env.http_proxy;
delete env.https_proxy;
delete env.all_proxy;
delete env.HTTP_PROXY;
delete env.HTTPS_PROXY;
delete env.ALL_PROXY;
env.NO_PROXY = "localhost,127.0.0.1,::1";
env.no_proxy = "localhost,127.0.0.1,::1";

const isWindows = process.platform === "win32";
const command = isWindows ? "playwright.cmd" : "playwright";

const child = spawn(command, commandArgs, {
  stdio: "inherit",
  env,
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});

child.on("error", (error) => {
  console.error("Failed to launch Playwright:", error.message);
  process.exit(1);
});
