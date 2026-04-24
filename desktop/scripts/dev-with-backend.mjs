import { once } from 'node:events';
import { spawn, spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { setTimeout as delay } from 'node:timers/promises';

const desktopDir = process.cwd();
const repoDir = path.resolve(desktopDir, '..');
const backendDir = path.join(repoDir, 'backend');
const BACKEND_HEALTH_URL = 'http://127.0.0.1:3000/health';
const BACKEND_START_TIMEOUT_MS = 30_000;
const BACKEND_POLL_INTERVAL_MS = 1_000;

let shuttingDown = false;
let frontendProcess = null;
let backendProcess = null;
let startedBackendHere = false;

function startChild(command, args, cwd) {
    return spawn(command, args, {
        cwd,
        stdio: 'inherit',
        shell: process.platform === 'win32',
        env: process.env,
    });
}

async function isBackendHealthy() {
    try {
        const response = await fetch(BACKEND_HEALTH_URL);
        const body = await response.text();
        return response.ok && body.trim() === 'OK';
    } catch {
        return false;
    }
}

async function ensureBackend() {
    if (await isBackendHealthy()) {
        console.log('[dev] backend already running on :3000');
        return;
    }

    console.log('[dev] starting backend on :3000');
    backendProcess = startChild('cargo', ['run'], backendDir);
    startedBackendHere = true;

    const deadline = Date.now() + BACKEND_START_TIMEOUT_MS;
    while (Date.now() < deadline) {
        if (backendProcess.exitCode !== null) {
            throw new Error(`backend exited early with code ${backendProcess.exitCode}`);
        }
        if (await isBackendHealthy()) {
            console.log('[dev] backend is healthy');
            return;
        }
        await delay(BACKEND_POLL_INTERVAL_MS);
    }

    throw new Error('backend did not become healthy in time');
}

function stopChildTree(child) {
    if (!child || child.pid == null || child.exitCode !== null) return;

    if (process.platform === 'win32') {
        spawnSync('taskkill', ['/pid', String(child.pid), '/t', '/f'], {
            stdio: 'ignore',
            shell: true,
        });
        return;
    }

    child.kill('SIGTERM');
}

async function shutdown(exitCode = 0) {
    if (shuttingDown) return;
    shuttingDown = true;

    stopChildTree(frontendProcess);
    if (startedBackendHere) {
        stopChildTree(backendProcess);
    }

    process.exit(exitCode);
}

process.on('SIGINT', () => {
    void shutdown(0);
});

process.on('SIGTERM', () => {
    void shutdown(0);
});

process.on('uncaughtException', (error) => {
    console.error('[dev] uncaught exception:', error);
    void shutdown(1);
});

process.on('unhandledRejection', (reason) => {
    console.error('[dev] unhandled rejection:', reason);
    void shutdown(1);
});

async function main() {
    await ensureBackend();

    console.log('[dev] starting frontend on :5173');
    frontendProcess = startChild('pnpm', ['dev:frontend'], desktopDir);

    if (startedBackendHere && backendProcess) {
        backendProcess.on('exit', (code, signal) => {
            if (shuttingDown) return;
            const reason = signal ? `signal ${signal}` : `code ${code ?? 1}`;
            console.error(`[dev] backend exited unexpectedly (${reason})`);
            void shutdown(typeof code === 'number' ? code : 1);
        });
    }

    const [frontendCode, frontendSignal] = await once(frontendProcess, 'exit');
    if (shuttingDown) return;

    const exitCode =
        typeof frontendCode === 'number'
            ? frontendCode
            : frontendSignal
              ? 1
              : 0;

    await shutdown(exitCode);
}

void main().catch((error) => {
    console.error('[dev] failed to start development environment:', error);
    void shutdown(1);
});
