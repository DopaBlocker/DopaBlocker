// Toast global — feedback transiente, não-bloqueante. Substitui banner-de-erro
// inline que várias páginas faziam na mão. Um contêiner em `+layout.svelte`
// assina o store e renderiza todos os toasts ativos.

import { writable } from 'svelte/store';

export type ToastKind = 'success' | 'error' | 'info';

export interface Toast {
    id: number;
    message: string;
    kind: ToastKind;
    duration: number;
}

function createToastStore() {
    const { subscribe, update } = writable<Toast[]>([]);
    let nextId = 1;

    function show(message: string, kind: ToastKind, duration: number) {
        const id = nextId++;
        update((list) => [...list, { id, message, kind, duration }]);
        if (duration > 0) {
            setTimeout(() => dismiss(id), duration);
        }
        return id;
    }

    function dismiss(id: number) {
        update((list) => list.filter((t) => t.id !== id));
    }

    return {
        subscribe,
        success: (msg: string, duration = 3000) => show(msg, 'success', duration),
        error: (msg: string, duration = 5000) => show(msg, 'error', duration),
        info: (msg: string, duration = 3500) => show(msg, 'info', duration),
        dismiss,
    };
}

export const toast = createToastStore();
