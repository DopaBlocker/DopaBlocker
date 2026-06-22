<!--
  Lista de itens bloqueados. Cada linha tem tipo, valor, quando foi adicionado
  e botão de remover que só aparece no hover pra não poluir o visual.
  Mostra empty state quando não há itens.
-->
<script lang="ts">
    import type { BlockedItem } from '$lib/types';
    import EmptyState from '$lib/components/ui/EmptyState.svelte';

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
    <div class="card">
        <EmptyState
            title="Nenhum bloqueio ainda"
            description="Comece pelos sites que mais te distraem."
        >
            {#snippet icon()}
                <svg
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    class="h-5 w-5"
                >
                    <path
                        d="M8 2l5 2v4a6 6 0 01-5 6 6 6 0 01-5-6V4l5-2z"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                </svg>
            {/snippet}
        </EmptyState>
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
