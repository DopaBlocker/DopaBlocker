<!--
  Gestão de bloqueios.
  - Header com título e CTA de adicionar.
  - Card master toggle (engine on/off) + toggle do filtro adulto.
  - Lista de itens bloqueados (BlockList).
  - Modal de adicionar (AddBlockModal).

  O engine fica parado até a etapa 7 — o toggle persiste a flag no cache
  local via `set_blocking_enabled`, sem efeito de rede ainda.
-->
<script lang="ts">
    import { onMount } from 'svelte';
    import { authStore, type AuthState } from '$lib/stores/auth';
    import { blockingStore, type BlockingState } from '$lib/stores/blocking';
    import BlockList from '$lib/components/BlockList.svelte';
    import AddBlockModal from '$lib/components/AddBlockModal.svelte';
    import type { BlockedType } from '$lib/types';

    let block: BlockingState = $state({
        items: [],
        status: { enabled: false, adult_filter_enabled: false, item_count: 0 },
        loading: false,
        error: null,
    });
    let currentUserId: string | null = $state(null);
    let modalOpen = $state(false);
    let pageError: string | null = $state(null);

    onMount(() => {
        const unsubA = authStore.subscribe((s: AuthState) => {
            currentUserId = s.user?.id ?? null;
            if (s.user) void blockingStore.load(s.user.id);
        });
        const unsubB = blockingStore.subscribe((s) => (block = s));
        return () => {
            unsubA();
            unsubB();
        };
    });

    async function handleAdd(type: BlockedType, value: string) {
        await blockingStore.addItem(type, value);
    }

    async function handleRemove(id: string) {
        pageError = null;
        try {
            await blockingStore.removeItem(id);
        } catch (err) {
            pageError = err instanceof Error ? err.message : String(err);
        }
    }

    async function handleToggleEngine() {
        if (!currentUserId) return;
        pageError = null;
        try {
            await blockingStore.toggleEngine(currentUserId, !block.status.enabled);
        } catch (err) {
            pageError = err instanceof Error ? err.message : String(err);
        }
    }

    async function handleToggleAdult() {
        pageError = null;
        try {
            await blockingStore.toggleAdultFilter(!block.status.adult_filter_enabled);
        } catch (err) {
            pageError = err instanceof Error ? err.message : String(err);
        }
    }
</script>

<div class="flex flex-col gap-6">
    <header class="flex items-start justify-between gap-4">
        <div>
            <div class="field-label">Bloqueios</div>
            <h1 class="mt-1 text-2xl font-semibold tracking-tight text-text">
                Lista de bloqueios
            </h1>
            <p class="mt-1 text-sm text-text-muted">
                Sites, aplicativos e palavras-chave que não podem ser acessados
                enquanto o bloqueio está ativo.
            </p>
        </div>
        <button type="button" onclick={() => (modalOpen = true)} class="btn-primary">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.75"
                class="h-4 w-4">
                <path d="M8 3v10M3 8h10" stroke-linecap="round" />
            </svg>
            Adicionar
        </button>
    </header>

    {#if pageError}
        <div
            class="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-xs text-danger"
        >
            {pageError}
        </div>
    {/if}

    <!-- Engine master toggle. -->
    <div class="card-padded flex items-center justify-between gap-4">
        <div class="flex items-center gap-3">
            <span
                class="inline-block h-2.5 w-2.5 rounded-full"
                class:bg-success={block.status.enabled}
                class:bg-text-dim={!block.status.enabled}
            ></span>
            <div>
                <div class="text-sm font-medium text-text">
                    Bloqueio {block.status.enabled ? 'ativo' : 'pausado'}
                </div>
                <div class="mt-0.5 text-xs text-text-muted">
                    {block.status.item_count}
                    {block.status.item_count === 1 ? 'item' : 'itens'} na lista
                </div>
            </div>
        </div>
        <button
            type="button"
            onclick={handleToggleEngine}
            class={block.status.enabled ? 'btn-secondary' : 'btn-primary'}
        >
            {block.status.enabled ? 'Pausar' : 'Ativar bloqueio'}
        </button>
    </div>

    <!-- Filtro adulto. -->
    <div class="card-padded flex items-center justify-between gap-4">
        <div>
            <div class="text-sm font-medium text-text">Filtro de conteúdo adulto</div>
            <div class="mt-0.5 text-xs text-text-muted">
                Adiciona uma lista curada de domínios adultos ao bloqueio.
            </div>
        </div>
        <button
            type="button"
            onclick={handleToggleAdult}
            class="relative h-6 w-11 rounded-full border transition-colors"
            class:bg-primary={block.status.adult_filter_enabled}
            class:border-primary={block.status.adult_filter_enabled}
            class:bg-surface-2={!block.status.adult_filter_enabled}
            class:border-border={!block.status.adult_filter_enabled}
            aria-pressed={block.status.adult_filter_enabled}
            aria-label="Filtro adulto"
        >
            <span
                class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all"
                class:left-5={block.status.adult_filter_enabled}
                class:left-0.5={!block.status.adult_filter_enabled}
            ></span>
        </button>
    </div>

    {#if block.loading}
        <div class="py-8 text-center text-xs text-text-muted">Carregando…</div>
    {:else}
        <BlockList items={block.items} onremove={handleRemove} />
    {/if}
</div>

<AddBlockModal
    open={modalOpen}
    onclose={() => (modalOpen = false)}
    onsubmit={handleAdd}
/>
