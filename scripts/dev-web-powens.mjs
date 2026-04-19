#!/usr/bin/env node
import { spawn } from "node:child_process";
import { readFileSync, existsSync } from "node:fs";
import net from "node:net";
import { resolve } from "node:path";

function loadDotenvFile(file) {
  const p = resolve(process.cwd(), file);
  if (!existsSync(p)) return;
  const content = readFileSync(p, "utf8");
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq === -1) continue;
    const key = line.slice(0, eq).trim();
    let value = line.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) {
      process.env[key] = value;
    }
  }
}

loadDotenvFile(".env.web");

process.env.BUILD_TARGET = "web";

const children = new Map();
let exiting = false;

function spawnNamed(name, cmd, args, opts = {}) {
  const child = spawn(cmd, args, { stdio: "inherit", shell: false, ...opts });
  children.set(name, child);
  child.on("exit", (code, signal) => {
    if (exiting) return;
    exiting = true;
    for (const [n, c] of children.entries()) {
      if (c.pid && n !== name) {
        try {
          process.kill(c.pid, "SIGTERM");
        } catch (e) {
          void e;
        }
      }
    }
    setTimeout(() => {
      for (const [n, c] of children.entries()) {
        if (c.pid && n !== name) {
          try {
            process.kill(c.pid, "SIGKILL");
          } catch (e) {
            void e;
          }
        }
      }
      process.exit(code === null ? (signal ? 128 : 1) : code);
    }, 500);
  });
  return child;
}

function shutdownAndExit(code = 0) {
  if (exiting) return;
  exiting = true;
  for (const [, c] of children.entries()) {
    if (c.pid) {
      try {
        process.kill(c.pid, "SIGTERM");
      } catch (e) {
        void e;
      }
    }
  }
  setTimeout(() => {
    for (const [, c] of children.entries()) {
      if (c.pid) {
        try {
          process.kill(c.pid, "SIGKILL");
        } catch (e) {
          void e;
        }
      }
    }
    process.exit(code);
  }, 500);
}

process.on("SIGINT", () => shutdownAndExit(130));
process.on("SIGTERM", () => shutdownAndExit(143));

function viteProxyTarget() {
  if (process.env.VITE_API_TARGET || process.env.WF_API_TARGET) {
    return process.env.VITE_API_TARGET || process.env.WF_API_TARGET;
  }

  const listenAddr = process.env.WF_LISTEN_ADDR || "127.0.0.1:8080";
  const port = listenAddr.includes(":")
    ? listenAddr.slice(listenAddr.lastIndexOf(":") + 1)
    : "8080";
  return `http://127.0.0.1:${port}`;
}

function waitForBackend(target) {
  const url = new URL(target);
  const host =
    url.hostname === "0.0.0.0" || url.hostname === "::" ? "127.0.0.1" : url.hostname;
  const port = Number(url.port || (url.protocol === "https:" ? 443 : 80));

  return new Promise((resolveWait) => {
    const tryConnect = () => {
      if (exiting) return;
      const socket = net.createConnection({ host, port });
      socket.setTimeout(1000);
      socket.once("connect", () => {
        socket.destroy();
        resolveWait();
      });
      socket.once("error", () => {
        socket.destroy();
        setTimeout(tryConnect, 500);
      });
      socket.once("timeout", () => {
        socket.destroy();
        setTimeout(tryConnect, 500);
      });
    };

    tryConnect();
  });
}

// Start all three: Rust backend, provider-sync-service, and Vite frontend.
process.env.WF_ENABLE_VITE_PROXY = "true";

spawnNamed("server", "cargo", ["run", "--manifest-path", "apps/server/Cargo.toml"]);

spawnNamed("provider-sync", "pnpm", ["--filter", "@wealthfolio/provider-sync-service", "start:dev"]);

const apiTarget = viteProxyTarget();
console.log(`Waiting for backend at ${apiTarget} before starting Vite...`);
waitForBackend(apiTarget).then(() => {
  if (!exiting) {
    spawnNamed("vite", "pnpm", ["--filter", "frontend", "dev"]);
  }
});
