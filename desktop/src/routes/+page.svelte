<!--
  Dashboard. Mostra estado real do engine + stats e dois atalhos. Os dados de
  bloqueio vêm do backend na primeira montagem; estado do engine vem do cache
  local (Tauri).
-->
<script lang="ts">
    import { goto } from '$app/navigation';
    import { onMount } from 'svelte';
    import { AUTH_BOOTING_STATE, authStore, type AuthState } from '$lib/stores/auth';
    import { blockingStore, type BlockingState } from '$lib/stores/blocking';
    import { getAppVersion } from '$lib/services/tauri-bridge';

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
    let appVersion: string | null = $state(null);

    onMount(() => {
        const unsubA = authStore.subscribe((s) => {
            auth = s;
            if (s.user) void blockingStore.load(s.user.id);
        });
        const unsubB = blockingStore.subscribe((s) => (block = s));
        void getAppVersion()
            .then((v) => (appVersion = v))
            .catch(() => (appVersion = null));
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
</script>

<div class="flex flex-col gap-8">
    <header>
        <div class="field-label">Dashboard</div>
        <h1 class="mt-1 text-2xl font-semibold tracking-tight text-text">
            {greeting()}, {auth.user?.display_name?.split(' ')[0] || 'por aí'}
        </h1>
        <p class="mt-1 text-sm text-text-muted">
            Visão geral do seu bloqueio e dos próximos passos.
        </p>
    </header>

    <!-- Status grande + ações rápidas. -->
    <div class="card-padded flex items-center justify-between gap-6">
        <div class="flex items-center gap-4">
            <div
                class="flex h-12 w-12 items-center justify-center rounded-full border"
                class:border-success={block.status.enabled}
                class:bg-success-subtle={block.status.enabled}
                class:border-border={!block.status.enabled}
                class:bg-surface-2={!block.status.enabled}
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    class="h-6 w-6"
                    class:text-success={block.status.enabled}
                    class:text-text-muted={!block.status.enabled}
                >
                    <path
                        d="M12 3l8 4v5a9 9 0 01-8 9 9 9 0 01-8-9V7l8-4z"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                    {#if block.status.enabled}
                        <path
                            d="M9 12l2 2 4-4"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    {/if}
                </svg>
            </div>
            <div>
                <div class="text-xs uppercase tracking-wider text-text-muted">
                    Bloqueio
                </div>
                <div class="mt-0.5 text-lg font-semibold text-text">
                    {block.status.enabled ? 'Ativo' : 'Pausado'}
                </div>
            </div>
        </div>
        <button type="button" onclick={() => goto('/blocking')} class="btn-primary">
            Gerenciar
        </button>
    </div>

    <!-- Grid de métricas. -->
    <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
        <div class="card-padded">
            <div class="field-label">Itens bloqueados</div>
            <div class="mt-2 text-2xl font-semibold text-text">
                {block.status.item_count}
            </div>
            <p class="mt-1 text-xs text-text-muted">
                {block.status.item_count === 0
                    ? 'Nenhum item adicionado ainda.'
                    : 'Sites, apps e palavras-chave.'}
            </p>
        </div>
        <div class="card-padded">
            <div class="field-label">Filtro adulto</div>
            <div class="mt-2 flex items-center gap-2">
                <span
                    class="inline-block h-2 w-2 rounded-full"
                    class:bg-success={block.status.adult_filter_enabled}
                    class:bg-text-dim={!block.status.adult_filter_enabled}
                ></span>
                <span class="text-lg font-semibold text-text">
                    {block.status.adult_filter_enabled ? 'Ligado' : 'Desligado'}
                </span>
            </div>
            <p class="mt-1 text-xs text-text-muted">
                Lista curada de domínios adultos.
            </p>
        </div>
        <div class="card-padded">
            <div class="field-label">Modo</div>
            <div class="mt-2 text-lg font-semibold text-text">
                {auth.user?.mode === 'parental' ? 'Parental' : 'Pessoal'}
            </div>
            <p class="mt-1 text-xs text-text-muted">
                Para gerenciar filhos, aguarde a v0.2.
            </p>
        </div>
    </div>

    <div class="mt-auto pt-4 text-[11px] text-text-dim">
        {#if appVersion}DopaBlocker desktop v{appVersion}{/if}
    </div>
</div>
