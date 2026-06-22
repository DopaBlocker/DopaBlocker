// Cliente HTTP para o backend Axum. Responsabilidades:
//   1. Pedir ao AuthProvider corrente o header `Authorization` (Firebase JWT
//      ou Device Token, dependendo do estado do auth store).
//   2. Retry single-shot em 401 (token possivelmente expirado) — apenas se o
//      provider suporta refresh (Firebase). Device Token não tenta de novo.
//   3. Parsear erros do formato `{ "error": "..." }` e expor via `ApiError`.
//   4. Tipar retornos com base em `types.ts`.

import { currentAuthProvider } from './auth-provider';
import type {
    AdultFilterSettings,
    BlockedItem,
    BlockMode,
    ConfirmLinkRequest,
    ConfirmLinkResponse,
    CreateBlockedItemRequest,
    Device,
    EmailCodeStartRequest,
    EmailCodeStartResponse,
    EmailCodeVerifyRequest,
    EmailCodeVerifyResponse,
    GenerateLinkCodeResponse,
    RegisterDeviceRequest,
    RegisterRequest,
    SuccessResponse,
    User,
} from '../types';

const BASE_URL = (import.meta.env.VITE_API_URL ?? 'http://localhost:3000').replace(/\/$/, '');
const REQUEST_TIMEOUT_MS = Number(import.meta.env.VITE_API_TIMEOUT_MS ?? 12000);

export class ApiError extends Error {
    constructor(
        public status: number,
        message: string,
    ) {
        super(message);
        this.name = 'ApiError';
    }
}

// Último ETag visto no GET /blocklist. Usado pelo auto-sync para enviar
// If-None-Match e receber 304 quando nada mudou — evita re-popular o engine
// (e limpar o cache DNS) a cada tick. Resetado ao (re)iniciar o auto-sync,
// pois o ETag é por-usuário.
let blocklistEtag: string | null = null;

export function resetBlocklistEtag(): void {
    blocklistEtag = null;
}

async function request<T>(
    method: string,
    path: string,
    body?: unknown,
    retriedOnce = false,
): Promise<T> {
    const provider = currentAuthProvider();
    const authHeader = await provider.getAuthHeader();
    const headers: Record<string, string> = {};
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
    if (body !== undefined) headers['Content-Type'] = 'application/json';
    if (authHeader) headers['Authorization'] = authHeader;

    let res: Response;
    try {
        res = await fetch(`${BASE_URL}${path}`, {
            method,
            headers,
            body: body !== undefined ? JSON.stringify(body) : undefined,
            signal: controller.signal,
        });
    } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
            throw new ApiError(
                0,
                'O backend demorou demais para responder. Verifique se a API local está rodando.',
            );
        }
        throw err;
    } finally {
        window.clearTimeout(timeoutId);
    }

    // Retry só se há provider que sabe refrescar (Firebase). Device Token
    // não expira — se voltar 401, é porque o pai revogou e não adianta tentar.
    if (res.status === 401 && !retriedOnce && authHeader) {
        const refreshed = await provider.refresh();
        if (refreshed) {
            return request<T>(method, path, body, true);
        }
    }

    if (!res.ok) {
        const text = await res.text();
        let msg = text || res.statusText;
        try {
            const parsed = JSON.parse(text);
            if (typeof parsed?.error === 'string') msg = parsed.error;
        } catch {
            /* corpo não-JSON, usa text cru */
        }
        throw new ApiError(res.status, msg);
    }

    if (res.status === 204) return undefined as T;
    return res.json() as Promise<T>;
}

// GET /blocklist condicional (If-None-Match). Devolve `null` em 304 (nada
// mudou) e a lista nova em 200, atualizando o ETag. Mantém o retry single-shot
// em 401 do Firebase (igual ao `request`). Usado pelo poll de auto-sync.
async function listBlocklistConditional(retriedOnce = false): Promise<BlockedItem[] | null> {
    const provider = currentAuthProvider();
    const authHeader = await provider.getAuthHeader();
    const headers: Record<string, string> = {};
    if (authHeader) headers['Authorization'] = authHeader;
    if (blocklistEtag) headers['If-None-Match'] = blocklistEtag;

    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
    let res: Response;
    try {
        res = await fetch(`${BASE_URL}/blocklist`, {
            method: 'GET',
            headers,
            signal: controller.signal,
        });
    } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
            throw new ApiError(
                0,
                'O backend demorou demais para responder. Verifique se a API local está rodando.',
            );
        }
        throw err;
    } finally {
        window.clearTimeout(timeoutId);
    }

    if (res.status === 401 && !retriedOnce && authHeader) {
        const refreshed = await provider.refresh();
        if (refreshed) return listBlocklistConditional(true);
    }

    if (res.status === 304) return null;

    if (!res.ok) {
        const text = await res.text();
        let msg = text || res.statusText;
        try {
            const parsed = JSON.parse(text);
            if (typeof parsed?.error === 'string') msg = parsed.error;
        } catch {
            /* corpo não-JSON, usa text cru */
        }
        throw new ApiError(res.status, msg);
    }

    const etag = res.headers.get('ETag');
    if (etag) blocklistEtag = etag;
    return (await res.json()) as BlockedItem[];
}

export const api = {
    // ---- auth ----
    startEmailVerification: (payload: EmailCodeStartRequest) =>
        request<EmailCodeStartResponse>('POST', '/auth/email-code/start', payload),

    verifyEmailCode: (payload: EmailCodeVerifyRequest) =>
        request<EmailCodeVerifyResponse>('POST', '/auth/email-code/verify', payload),

    register: (payload: RegisterRequest) =>
        request<User>('POST', '/auth/register', payload),

    login: () => request<User>('POST', '/auth/login'),

    me: () => request<User>('GET', '/auth/me'),

    // Troca o modo da conta (personal↔parental) sem recriá-la. Só Firebase JWT.
    updateMode: (mode: BlockMode) => request<User>('PUT', '/auth/me', { mode }),

    deleteAccount: () => request<SuccessResponse>('DELETE', '/auth/me'),

    // ---- blocklist ----
    listBlocklist: () => request<BlockedItem[]>('GET', '/blocklist'),

    // GET condicional usado pelo auto-sync: `null` quando nada mudou (304).
    listBlocklistIfChanged: () => listBlocklistConditional(),

    createBlockedItem: (payload: CreateBlockedItemRequest) =>
        request<BlockedItem>('POST', '/blocklist', payload),

    deleteBlockedItem: (id: string) =>
        request<{ message: string }>('DELETE', `/blocklist/${id}`),

    setAdultFilter: (enabled: boolean) =>
        request<AdultFilterSettings>('PUT', '/blocklist/adult-filter', { enabled }),

    // ---- devices / parental ----
    listDevices: () => request<Device[]>('GET', '/devices'),

    registerDevice: (payload: RegisterDeviceRequest) =>
        request<Device>('POST', '/devices/register', payload),

    generateLinkCode: () =>
        request<GenerateLinkCodeResponse>('POST', '/devices/link/generate'),

    confirmLinkCode: (payload: ConfirmLinkRequest) =>
        request<ConfirmLinkResponse>('POST', '/devices/link/confirm', payload),

    revokeDevice: (id: string) =>
        request<SuccessResponse>('POST', `/devices/${id}/revoke`),
};
