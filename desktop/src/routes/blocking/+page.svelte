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
    import { toast } from '$lib/stores/toast';
    import BlockList from '$lib/components/BlockList.svelte';
    import AddBlockModal from '$lib/components/AddBlockModal.svelte';
    import Skeleton from '$lib/components/ui/Skeleton.svelte';
    import type { BlockedType } from '$lib/types';

    let block: BlockingState = $state({
        items: [],
        status: {
            enabled: false,
            adult_filter_enabled: false,
            adult_filter_building: false,
            item_count: 0,
        },
        loading: false,
        error: null,
    });
    let currentUserId: string | null = $state(null);
    let isChild = $state(false);
    let modalOpen = $state(false);

    onMount(() => {
        const unsubA = authStore.subscribe((s: AuthState) => {
            // Sessão de filho: o user_id vem do `auth.child` (e o user_id do
            // pai), não de `auth.user`. Carregamos a blocklist do pai mas
            // bloqueamos qualquer write a partir da UI (read-only).
            isChild = s.phase === 'child_session';
            currentUserId = s.user?.id ?? s.child?.user_id ?? null;
            if (currentUserId) void blockingStore.load(currentUserId);
        });
        const unsubB = blockingStore.subscribe((s) => (block = s));
        return () => {
            unsubA();
            unsubB();
        };
    });

    function reportError(err: unknown, fallback: string) {
        const msg = err instanceof Error ? err.message : String(err);
        toast.error(msg || fallback);
    }

    async function handleAdd(type: BlockedType, value: string) {
        await blockingStore.addItem(type, value);
        toast.success('Bloqueio adicionado');
    }

    async function handleRemove(id: string) {
        try {
            await blockingStore.removeItem(id);
            toast.info('Item removido');
        } catch (err) {
            reportError(err, 'Falha ao remover item');
        }
    }

    async function handleToggleEngine() {
        if (!currentUserId) return;
        const target = !block.status.enabled;
        try {
            await blockingStore.toggleEngine(currentUserId, target);
            toast.success(target ? 'Bloqueio ativado' : 'Bloqueio pausado');
        } catch (err) {
            reportError(err, 'Falha ao alternar bloqueio');
        }
    }

    async function handleToggleAdult() {
        const target = !block.status.adult_filter_enabled;
        try {
            await blockingStore.toggleAdultFilter(target);
            toast.info(target ? 'Filtro adulto ligado' : 'Filtro adulto desligado');
        } catch (err) {
            reportError(err, 'Falha ao alternar filtro adulto');
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
        {#if !isChild}
            <button type="button" onclick={() => (modalOpen = true)} class="btn-primary">
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.75"
                    class="h-4 w-4">
                    <path d="M8 3v10M3 8h10" stroke-linecap="round" />
                </svg>
                Adicionar
            </button>
        {/if}
    </header>

    {#if isChild}
        <div
            class="rounded-md border border-secondary/50 bg-secondary/10 px-3 py-2 text-xs text-secondary"
        >
            Você está no modo Filhos — a lista é gerenciada pelo responsável.
            Apenas visualização.
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
                    <span class="num">{block.status.item_count}</span>
                    {block.status.item_count === 1 ? 'item' : 'itens'} na lista
                </div>
            </div>
        </div>
        <button
            type="button"
            onclick={handleToggleEngine}
            disabled={isChild}
            class={block.status.enabled ? 'btn-secondary' : 'btn-primary'}
        >
            {block.status.enabled ? 'Pausar' : 'Ativar bloqueio'}
        </button>
    </div>

    <!-- Filtro adulto. -->
    <div class="card-padded flex items-center justify-between gap-4">
        <div>
            <div class="flex items-center gap-2">
                <span class="text-sm font-medium text-text">Filtro de conteúdo adulto</span>
                {#if block.status.adult_filter_building}
                    <span class="badge-secondary">Construindo…</span>
                {/if}
            </div>
            <div class="mt-0.5 text-xs text-text-muted">
                {#if block.status.adult_filter_building}
                    Baixando lista de domínios. O bloqueio começa a valer em instantes.
                {:else}
                    Adiciona uma lista curada de domínios adultos ao bloqueio.
                {/if}
            </div>
        </div>
        <button
            type="button"
            onclick={handleToggleAdult}
            disabled={isChild}
            class="relative h-6 w-11 rounded-full border transition-colors disabled:opacity-50"
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
        <div class="card divide-y divide-border overflow-hidden">
            {#each [0, 1, 2] as i (i)}
                <div class="flex items-center gap-4 px-5 py-3.5">
                    <Skeleton class="h-5 w-12 rounded-full" />
                    <Skeleton class="h-4 w-40 max-w-[45%]" />
                    <Skeleton class="ml-auto hidden h-3 w-16 sm:block" />
                </div>
            {/each}
        </div>
    {:else}
        <BlockList items={block.items} onremove={handleRemove} readOnly={isChild} />
    {/if}
</div>

<AddBlockModal
    open={modalOpen}
    onclose={() => (modalOpen = false)}
    onsubmit={handleAdd}
/>
