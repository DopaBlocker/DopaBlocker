// Store de blocklist + estado do engine. Fonte-da-verdade dos itens é o
// backend (GET/POST/DELETE /blocklist). O cache local Tauri é atualizado em
// paralelo para que o engine (DNS + WFP) tenha os dados mesmo offline e sem
// latência de rede.
//
// Padrão otimista: adicionar/remover mexe no state imediatamente e reverte
// se o backend recusar.

import { get, writable } from 'svelte/store';
import type { BlockedItem, BlockedType, BlockingStatus } from '../types';
import { api, resetBlocklistEtag } from '../services/api';
import * as bridge from '../services/tauri-bridge';
import { authStore } from './auth';

/// Intervalo do auto-sync da blocklist (modo pessoal/pai). ~30s, alinhado ao
/// poll do device-filho. Barato graças ao ETag/304 do backend.
const AUTO_SYNC_INTERVAL_MS = 30_000;

/// Deriva o `ParentalContext` a ser enviado aos comandos Tauri a partir do
/// estado atual do auth. O Rust usa isso para aplicar a regra do pai imune
/// (device do pai em modo parental → engine recebe lista vazia).
function parentalContext(): bridge.ParentalContext {
    const auth = get(authStore);
    if (auth.phase === 'child_session') {
        // Filho aplica todos os bloqueios (gerenciados pelo pai).
        return { mode: 'parental', is_child: true };
    }
    return {
        mode: auth.user?.mode ?? 'personal',
        is_child: false,
    };
}

export interface BlockingState {
    items: BlockedItem[];
    status: BlockingStatus;
    loading: boolean;
    error: string | null;
}

const initialStatus: BlockingStatus = {
    enabled: false,
    adult_filter_enabled: false,
    adult_filter_building: false,
    item_count: 0,
};

function createBlockingStore() {
    const { subscribe, set, update } = writable<BlockingState>({
        items: [],
        status: initialStatus,
        loading: false,
        error: null,
    });

    // Timer do auto-sync. Mantido fora do state porque é detalhe de runtime,
    // não estado reativo da UI.
    let autoSyncTimer: number | null = null;

    async function load(userId: string) {
        update((s) => ({ ...s, loading: true, error: null }));
        try {
            const items = await api.listBlocklist();
            // Espelha no cache local pra que o engine (DNS/WFP) tenha os dados
            // prontos mesmo se o backend cair depois.
            await bridge.saveBlocklist(userId, items, parentalContext()).catch((e) => {
                console.warn('Falha ao espelhar blocklist no cache local:', e);
            });
            const status = await bridge.getBlockingStatus(userId).catch(() => ({
                enabled: false,
                adult_filter_enabled: false,
                adult_filter_building: false,
                item_count: items.length,
            }));
            set({
                items,
                status: { ...status, item_count: items.length },
                loading: false,
                error: null,
            });
        } catch (err) {
            update((s) => ({
                ...s,
                loading: false,
                error: err instanceof Error ? err.message : String(err),
            }));
        }
    }

    /// Sincronização silenciosa (sem flag de loading) usada pelo auto-sync.
    /// Usa o GET condicional: se o backend responder 304 (nada mudou), não
    /// re-popula o engine nem mexe no state. Em mudança, espelha no cache local
    /// (o que reaplica as regras no engine) e atualiza os itens da UI.
    async function refresh(userId: string) {
        const items = await api.listBlocklistIfChanged();
        if (items === null) return; // 304: nada mudou.
        await bridge.saveBlocklist(userId, items, parentalContext()).catch((e) => {
            console.warn('Falha ao espelhar blocklist no cache local:', e);
        });
        update((s) => ({
            ...s,
            items,
            status: { ...s.status, item_count: items.length },
        }));
    }

    /// Liga o poll periódico que mantém o cache local (de onde o engine lê) em
    /// dia com o backend — necessário para o modo pessoal/pai, onde a mudança
    /// pode vir de OUTRO device. O device-filho tem o próprio poll em
    /// /child-blocked; aqui é só para sessões Firebase. Idempotente.
    function startAutoSync(userId: string) {
        stopAutoSync();
        resetBlocklistEtag(); // ETag é por-usuário; zera ao (re)iniciar.
        autoSyncTimer = window.setInterval(() => {
            void refresh(userId).catch(() => {
                /* rede/5xx/401: ignora e tenta no próximo tick */
            });
        }, AUTO_SYNC_INTERVAL_MS);
    }

    function stopAutoSync() {
        if (autoSyncTimer !== null) {
            window.clearInterval(autoSyncTimer);
            autoSyncTimer = null;
        }
    }

    async function addItem(itemType: BlockedType, value: string) {
        const trimmed = value.trim();
        if (!trimmed) throw new Error('Valor vazio');

        const created = await api.createBlockedItem({
            item_type: itemType,
            value: trimmed,
        });
        update((s) => ({
            ...s,
            items: [created, ...s.items],
            status: { ...s.status, item_count: s.status.item_count + 1 },
        }));
        bridge.cacheAddItem(created, parentalContext()).catch((e) =>
            console.warn('Falha ao espelhar add no cache:', e),
        );
        return created;
    }

    async function removeItem(id: string) {
        let removed: BlockedItem | undefined;
        update((s) => {
            removed = s.items.find((i) => i.id === id);
            return {
                ...s,
                items: s.items.filter((i) => i.id !== id),
                status: { ...s.status, item_count: Math.max(0, s.status.item_count - 1) },
            };
        });
        try {
            await api.deleteBlockedItem(id);
            if (removed) {
                bridge.cacheRemoveItem(id, removed.user_id, parentalContext()).catch((e) =>
                    console.warn('Falha ao espelhar remove no cache:', e),
                );
            }
        } catch (err) {
            // Rollback otimista.
            if (removed) {
                update((s) => ({
                    ...s,
                    items: [removed!, ...s.items],
                    status: { ...s.status, item_count: s.status.item_count + 1 },
                }));
            }
            throw err;
        }
    }

    async function toggleEngine(userId: string, enabled: boolean) {
        // Atualiza UI primeiro, persiste depois. Se der ruim, reverte.
        update((s) => ({ ...s, status: { ...s.status, enabled } }));
        try {
            await bridge.setBlockingEnabled(userId, enabled, parentalContext());
        } catch (err) {
            update((s) => ({ ...s, status: { ...s.status, enabled: !enabled } }));
            throw err;
        }
    }

    async function toggleAdultFilter(enabled: boolean) {
        update((s) => ({
            ...s,
            status: { ...s.status, adult_filter_enabled: enabled },
        }));
        try {
            await api.setAdultFilter(enabled);
            await bridge.setAdultFilterEnabled(enabled);
        } catch (err) {
            update((s) => ({
                ...s,
                status: { ...s.status, adult_filter_enabled: !enabled },
            }));
            throw err;
        }
    }

    function reset() {
        stopAutoSync();
        set({ items: [], status: initialStatus, loading: false, error: null });
    }

    return {
        subscribe,
        load,
        refresh,
        startAutoSync,
        stopAutoSync,
        addItem,
        removeItem,
        toggleEngine,
        toggleAdultFilter,
        reset,
    };
}

export const blockingStore = createBlockingStore();
