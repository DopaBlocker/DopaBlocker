// Cliente HTTP para o backend Axum. Responsabilidades:
//   1. Injetar o Firebase JWT em `Authorization: Bearer ...` em cada request.
//   2. Retry single-shot em 401 (token possivelmente expirado) após refresh.
//   3. Parsear erros do formato `{ "error": "..." }` e expor via `ApiError`.
//   4. Tipar retornos com base em `types.ts`.

import { getIdToken } from './firebase';
import type {
    AdultFilterSettings,
    BlockMode,
    BlockedItem,
    CreateBlockedItemRequest,
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

async function request<T>(
    method: string,
    path: string,
    body?: unknown,
    retriedOnce = false,
): Promise<T> {
    const token = await getIdToken();
    const headers: Record<string, string> = {};
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
    if (body !== undefined) headers['Content-Type'] = 'application/json';
    if (token) headers['Authorization'] = `Bearer ${token}`;

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
                'O backend demorou demais para responder. Verifique se a API local estah rodando.',
            );
        }
        throw err;
    } finally {
        window.clearTimeout(timeoutId);
    }

    if (res.status === 401 && !retriedOnce && token) {
        await getIdToken(true);
        return request<T>(method, path, body, true);
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

export const api = {
    register: (payload: { email: string; display_name: string; mode: BlockMode }) =>
        request<User>('POST', '/auth/register', payload),

    login: () => request<User>('POST', '/auth/login'),

    me: () => request<User>('GET', '/auth/me'),

    listBlocklist: () => request<BlockedItem[]>('GET', '/blocklist'),

    createBlockedItem: (payload: CreateBlockedItemRequest) =>
        request<BlockedItem>('POST', '/blocklist', payload),

    deleteBlockedItem: (id: string) =>
        request<{ message: string }>('DELETE', `/blocklist/${id}`),

    setAdultFilter: (enabled: boolean) =>
        request<AdultFilterSettings>('PUT', '/blocklist/adult-filter', { enabled }),
};
