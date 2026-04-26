<!--
  Lista de itens bloqueados. Cada linha tem tipo, valor, quando foi adicionado
  e botão de remover que só aparece no hover pra não poluir o visual.
  Mostra empty state quando não há itens.
-->
<script lang="ts">
    import type { BlockedItem } from '$lib/types';

    interface Props {
        items: BlockedItem[];
        onremove: (id: string) => void;
        /** True esconde o botao de remover (modo Filhos — read-only). */
        readOnly?: boolean;
    }

    let { items, onremove, readOnly = false }: Props = $props();

    function typeLabel(t: BlockedItem['item_type']) {
        switch (t) {
            case 'domain':
                return 'Site';
            case 'app':
                return 'App';
            case 'keyword':
                return 'Palavra';
        }
    }

    function relativeTime(iso: string): string {
        const now = Date.now();
        const then = new Date(iso).getTime();
        if (Number.isNaN(then)) return '';
        const diff = Math.max(0, now - then);
        const min = Math.floor(diff / 60000);
        if (min < 1) return 'agora';
        if (min < 60) return `${min} min atrás`;
        const hr = Math.floor(min / 60);
        if (hr < 24) return `${hr} h atrás`;
        const d = Math.floor(hr / 24);
        if (d < 30) return `${d} d atrás`;
        const mo = Math.floor(d / 30);
        return `${mo} mês${mo > 1 ? 'es' : ''} atrás`;
    }
</script>

{#if items.length === 0}
    <div
        class="card flex flex-col items-center justify-center gap-3 px-6 py-16 text-center"
    >
        <div
            class="flex h-12 w-12 items-center justify-center rounded-full border border-border bg-surface-2"
        >
            <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                class="h-6 w-6 text-text-muted"
            >
                <path
                    d="M12 3l8 4v5a9 9 0 01-8 9 9 9 0 01-8-9V7l8-4z"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                />
            </svg>
        </div>
        <div>
            <h3 class="text-sm font-medium text-text">Nenhum bloqueio ainda</h3>
            <p class="mt-1 text-xs text-text-muted">
                Comece pelos sites que mais te distraem.
            </p>
        </div>
    </div>
{:else}
    <div class="card overflow-hidden">
        <ul class="divide-y divide-border">
            {#each items as item (item.id)}
                <li
                    class="group flex items-center gap-4 px-5 py-3 transition-colors hover:bg-surface-2"
                >
                    <span class="badge-neutral shrink-0">{typeLabel(item.item_type)}</span>
                    <span class="flex-1 truncate text-sm text-text">{item.value}</span>
                    <span class="hidden text-xs text-text-dim sm:block">
                        {relativeTime(item.created_at)}
                    </span>
                    {#if !readOnly}
                        <button
                            type="button"
                            aria-label="Remover {item.value}"
                            onclick={() => onremove(item.id)}
                            class="btn-icon text-text-dim opacity-0 transition-opacity hover:text-danger group-hover:opacity-100 focus-visible:opacity-100"
                        >
                            <svg
                                viewBox="0 0 16 16"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                                class="h-4 w-4"
                            >
                                <path
                                    d="M3 4h10M6.5 4V3a1 1 0 011-1h1a1 1 0 011 1v1M5 4l.5 9a1 1 0 001 1h3a1 1 0 001-1L11 4"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                            </svg>
                        </button>
                    {/if}
                </li>
            {/each}
        </ul>
    </div>
{/if}
