// Store de blocklist + estado do engine. Fonte-da-verdade dos itens é o
// backend (GET/POST/DELETE /blocklist). O cache local Tauri é atualizado em
// paralelo para que o engine (DNS + WFP) tenha os dados mesmo offline e sem
// latência de rede.
//
// Padrão otimista: adicionar/remover mexe no state imediatamente e reverte
// se o backend recusar.

import { writable } from 'svelte/store';
import type { BlockedItem, BlockedType, BlockingStatus } from '../types';
import { api } from '../services/api';
import * as bridge from '../services/tauri-bridge';

export interface BlockingState {
    items: BlockedItem[];
    status: BlockingStatus;
    loading: boolean;
    error: string | null;
}

const initialStatus: BlockingStatus = {
    enabled: false,
    adult_filter_enabled: false,
    item_count: 0,
};

function createBlockingStore() {
    const { subscribe, set, update } = writable<BlockingState>({
        items: [],
        status: initialStatus,
        loading: false,
        error: null,
    });

    async function load(userId: string) {
        update((s) => ({ ...s, loading: true, error: null }));
        try {
            const items = await api.listBlocklist();
            // Espelha no cache local pra que o engine (DNS/WFP) tenha os dados
            // prontos mesmo se o backend cair depois.
            await bridge.saveBlocklist(userId, items).catch((e) => {
                console.warn('Falha ao espelhar blocklist no cache local:', e);
            });
            const status = await bridge.getBlockingStatus(userId).catch(() => ({
                enabled: false,
                adult_filter_enabled: false,
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
        bridge.cacheAddItem(created).catch((e) =>
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
                bridge.cacheRemoveItem(id, removed.user_id).catch((e) =>
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
            await bridge.setBlockingEnabled(userId, enabled);
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
        set({ items: [], status: initialStatus, loading: false, error: null });
    }

    return { subscribe, load, addItem, removeItem, toggleEngine, toggleAdultFilter, reset };
}

export const blockingStore = createBlockingStore();
