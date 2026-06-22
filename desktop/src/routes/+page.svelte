<!--
  Início — hub de status honesto. Espelha a estrutura do mobile (hero de
  proteção + camadas ativas + métricas reais + resumo parental). Só dado real:
  estado do engine (cache local Tauri) + blocklist (backend). Sem estatísticas.
-->
<script lang="ts">
    import { goto } from '$app/navigation';
    import { onMount } from 'svelte';
    import { AUTH_BOOTING_STATE, authStore, type AuthState } from '$lib/stores/auth';
    import { blockingStore, type BlockingState } from '$lib/stores/blocking';

    let auth: AuthState = $state({ ...AUTH_BOOTING_STATE });
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

    onMount(() => {
        const unsubA = authStore.subscribe((s) => {
            auth = s;
            if (s.user) void blockingStore.load(s.user.id);
        });
        const unsubB = blockingStore.subscribe((s) => (block = s));
        return () => {
            unsubA();
            unsubB();
        };
    });

    const greeting = $derived(() => {
        const h = new Date().getHours();
        if (h < 5) return 'Boa madrugada';
        if (h < 12) return 'Bom dia';
        if (h < 18) return 'Boa tarde';
        return 'Boa noite';
    });

    const isParental = $derived(auth.user?.mode === 'parental');
    const enabled = $derived(block.status.enabled);
</script>

<div class="flex flex-col gap-8">
    <header>
        <div class="field-label">Início</div>
        <h1 class="mt-1 text-2xl font-semibold tracking-tight text-text">
            {greeting()}, {auth.user?.display_name?.split(' ')[0] || 'por aí'}
        </h1>
        <p class="mt-1 text-sm text-text-muted">
            Estado da sua proteção neste computador.
        </p>
    </header>

    <!-- Hero de proteção. -->
    <div
        class="flex items-center justify-between gap-6 rounded-lg p-5"
        class:card-highlight={enabled}
        class:card-padded={!enabled}
    >
        <div class="flex items-center gap-4">
            <div
                class="flex h-14 w-14 items-center justify-center rounded-full transition-colors duration-200"
                class:bg-success-subtle={enabled}
                class:bg-surface-2={!enabled}
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    class="h-7 w-7"
                    class:text-success={enabled}
                    class:text-warning={!enabled}
                >
                    <path
                        d="M12 3l8 4v5a9 9 0 01-8 9 9 9 0 01-8-9V7l8-4z"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                    {#if enabled}
                        <path d="M9 12l2 2 4-4" stroke-linecap="round" stroke-linejoin="round" />
                    {/if}
                </svg>
            </div>
            <div>
                <div class="text-lg font-semibold text-text">
                    {enabled ? 'Protegido' : 'Proteção pausada'}
                </div>
                <div class="mt-0.5 text-sm text-text-muted">
                    {enabled
                        ? 'Bloqueio em execução neste computador.'
                        : 'O bloqueio está pausado. Reative em Bloqueios.'}
                </div>
            </div>
        </div>
        <button type="button" onclick={() => goto('/blocking')} class="btn-primary shrink-0">
            Gerenciar
        </button>
    </div>

    <!-- Camadas ativas. -->
    <section class="flex flex-col gap-3">
        <div class="field-label">Camadas ativas</div>
        <div class="card divide-y divide-border">
            <div class="flex items-center gap-3 px-4 py-3.5">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"
                    class="h-5 w-5 text-text-muted">
                    <circle cx="12" cy="12" r="9" />
                    <path d="M3 12h18M12 3c2.5 2.5 2.5 15 0 18M12 3c-2.5 2.5-2.5 15 0 18" />
                </svg>
                <div class="flex-1">
                    <div class="text-sm font-medium text-text">Bloqueio de sites</div>
                    <div class="text-xs text-text-dim">Sinkhole de DNS + WFP</div>
                </div>
                {#if enabled}
                    <span class="badge-success">Ativo</span>
                {:else}
                    <span class="badge-neutral">Pausado</span>
                {/if}
            </div>
            <div class="flex items-center gap-3 px-4 py-3.5">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"
                    class="h-5 w-5 text-text-muted">
                    <path d="M12 3l8 4v5a9 9 0 01-8 9 9 9 0 01-8-9V7l8-4z"
                        stroke-linecap="round" stroke-linejoin="round" />
                </svg>
                <div class="flex-1">
                    <div class="text-sm font-medium text-text">Filtro adulto</div>
                    <div class="text-xs text-text-dim">Lista curada de domínios adultos</div>
                </div>
                {#if block.status.adult_filter_building}
                    <span class="badge-warning">Construindo…</span>
                {:else if block.status.adult_filter_enabled}
                    <span class="badge-success">Ligado</span>
                {:else}
                    <span class="badge-neutral">Desligado</span>
                {/if}
            </div>
        </div>
    </section>

    <!-- Métricas reais. -->
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div class="card-padded">
            <div class="field-label">Itens bloqueados</div>
            <div class="num mt-2 text-2xl font-semibold text-text">{block.status.item_count}</div>
            <p class="mt-1 text-xs text-text-muted">
                {block.status.item_count === 0
                    ? 'Nenhum item adicionado ainda.'
                    : 'Sites, apps e palavras-chave.'}
            </p>
        </div>
        <div class="card-padded">
            <div class="field-label">Modo</div>
            <div class="mt-2 text-2xl font-semibold text-text">
                {isParental ? 'Pais' : 'Pessoal'}
            </div>
            <p class="mt-1 text-xs text-text-muted">
                {isParental
                    ? 'Você gerencia a blocklist dos dispositivos vinculados.'
                    : 'Você controla seus próprios bloqueios.'}
            </p>
        </div>
    </div>

    <!-- Resumo parental. -->
    {#if isParental}
        <button
            type="button"
            onclick={() => goto('/parental')}
            class="card-padded flex items-center gap-4 text-left transition-all hover:bg-surface-hover active:scale-[0.99] motion-reduce:active:scale-100"
        >
            <div
                class="flex h-11 w-11 items-center justify-center rounded-lg"
                style="background: var(--color-primary-subtle)"
            >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"
                    class="h-5 w-5 text-primary">
                    <circle cx="8" cy="8" r="3" />
                    <circle cx="16" cy="8" r="3" />
                    <path d="M3 19c0-2.5 2.7-4.5 5.5-4.5M21 19c0-2.5-2.7-4.5-5.5-4.5"
                        stroke-linecap="round" />
                </svg>
            </div>
            <div class="flex-1">
                <div class="text-sm font-medium text-text">Filhos vinculados</div>
                <div class="text-xs text-text-muted">Gerar código, ver dispositivos e alertas.</div>
            </div>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"
                class="h-5 w-5 text-text-muted">
                <path d="M9 6l6 6-6 6" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
        </button>
    {/if}
</div>
