// Wrappers tipados para os comandos Tauri (desktop/src-tauri/src/commands.rs).
// O frontend consome o backend REST via `api.ts` e espelha o resultado no
// cache local via estas funções. O engine de bloqueio (DNS + WFP) lê direto
// do cache local, portanto manter `save_blocklist` em sincronia é essencial.

import { invoke } from '@tauri-apps/api/core';
import type { BlockedItem, BlockingStatus } from '../types';

export function getAppVersion(): Promise<string> {
    return invoke<string>('get_app_version');
}

export function listCachedBlocklist(userId: string): Promise<BlockedItem[]> {
    return invoke<BlockedItem[]>('list_cached_blocklist', { userId });
}

export function saveBlocklist(userId: string, items: BlockedItem[]): Promise<void> {
    return invoke<void>('save_blocklist', { userId, items });
}

export function cacheAddItem(item: BlockedItem): Promise<void> {
    return invoke<void>('cache_add_item', { item });
}

export function cacheRemoveItem(id: string, userId: string): Promise<void> {
    return invoke<void>('cache_remove_item', { id, userId });
}

export function setBlockingEnabled(userId: string, enabled: boolean): Promise<void> {
    return invoke<void>('set_blocking_enabled', { userId, enabled });
}

export function setAdultFilterEnabled(enabled: boolean): Promise<void> {
    return invoke<void>('set_adult_filter_enabled', { enabled });
}

export function getBlockingStatus(userId: string): Promise<BlockingStatus> {
    return invoke<BlockingStatus>('get_blocking_status', { userId });
}
