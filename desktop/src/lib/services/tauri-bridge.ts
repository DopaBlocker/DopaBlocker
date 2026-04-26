// Wrappers tipados para os comandos Tauri (desktop/src-tauri/src/commands.rs).
// O frontend consome o backend REST via `api.ts` e espelha o resultado no
// cache local via estas funções. O engine de bloqueio (DNS + WFP) lê direto
// do cache local, portanto manter `save_blocklist` em sincronia é essencial.

import { invoke } from '@tauri-apps/api/core';
import type { BlockMode, BlockedItem, BlockingStatus, ChildSession } from '../types';

/// Contexto que o engine usa para decidir se aplica a regra do pai imune.
/// Espelha `ParentalContext` em desktop/src-tauri/src/commands.rs.
export interface ParentalContext {
    mode: BlockMode;
    is_child: boolean;
}

export function getAppVersion(): Promise<string> {
    return invoke<string>('get_app_version');
}

export function listCachedBlocklist(userId: string): Promise<BlockedItem[]> {
    return invoke<BlockedItem[]>('list_cached_blocklist', { userId });
}

export function saveBlocklist(
    userId: string,
    items: BlockedItem[],
    parental?: ParentalContext,
): Promise<void> {
    return invoke<void>('save_blocklist', { userId, items, parental });
}

export function cacheAddItem(item: BlockedItem, parental?: ParentalContext): Promise<void> {
    return invoke<void>('cache_add_item', { item, parental });
}

export function cacheRemoveItem(
    id: string,
    userId: string,
    parental?: ParentalContext,
): Promise<void> {
    return invoke<void>('cache_remove_item', { id, userId, parental });
}

export function setBlockingEnabled(
    userId: string,
    enabled: boolean,
    parental?: ParentalContext,
): Promise<void> {
    return invoke<void>('set_blocking_enabled', { userId, enabled, parental });
}

export function setAdultFilterEnabled(enabled: boolean): Promise<void> {
    return invoke<void>('set_adult_filter_enabled', { enabled });
}

export function getBlockingStatus(userId: string): Promise<BlockingStatus> {
    return invoke<BlockingStatus>('get_blocking_status', { userId });
}

// ---- child_session (sessao de filho persistida em SQLCipher) ----

export function saveChildSession(session: ChildSession): Promise<void> {
    return invoke<void>('save_child_session', {
        userId: session.user_id,
        deviceId: session.device_id,
        deviceToken: session.device_token,
        parentDeviceId: session.parent_device_id,
    });
}

export function loadChildSession(): Promise<ChildSession | null> {
    return invoke<ChildSession | null>('load_child_session');
}

export function clearChildSession(): Promise<void> {
    return invoke<void>('clear_child_session');
}
