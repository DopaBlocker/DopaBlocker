<!--
  Dashboard do modo parental — visão do pai. Lista os filhos vinculados
  (devices com is_child=true) e permite revogar cada um.

  A geração de código de vinculação fica em DeviceLinkCode.svelte (componente
  separado) — esta tela é só leitura + revogação.
-->
<script lang="ts">
    import { onMount } from 'svelte';
    import { api, ApiError } from '$lib/services/api';
    import { toast } from '$lib/stores/toast';
    import type { Device } from '$lib/types';
    import Skeleton from '$lib/components/ui/Skeleton.svelte';
    import EmptyState from '$lib/components/ui/EmptyState.svelte';

    let devices: Device[] = $state([]);
    let loading = $state(true);
    let revokingId: string | null = $state(null);

    async function reload() {
        loading = true;
        try {
            const all = await api.listDevices();
            // Pai só quer ver os filhos aqui — devices próprios do pai
            // aparecem em /settings.
            devices = all.filter((d) => d.is_child);
        } catch (err) {
            toast.error(friendly(err));
        } finally {
            loading = false;
        }
    }

    async function handleRevoke(device: Device) {
        if (revokingId) return;
        revokingId = device.id;
        try {
            await api.revokeDevice(device.id);
            devices = devices.filter((d) => d.id !== device.id);
            toast.success(`${device.device_name} desvinculado`);
        } catch (err) {
            toast.error(friendly(err));
        } finally {
            revokingId = null;
        }
    }

    function friendly(err: unknown): string {
        if (err instanceof ApiError) return err.message;
        if (err instanceof Error) return err.message;
        return String(err);
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
        return `${d} d atrás`;
    }

    onMount(reload);
</script>

<section class="flex flex-col gap-3">
    <div class="flex items-baseline justify-between">
        <h2 class="text-sm font-semibold text-text">Filhos vinculados</h2>
        <button type="button" onclick={reload} class="text-xs text-text-muted hover:text-text">
            Atualizar
        </button>
    </div>

    {#if loading}
        <div class="card flex flex-col gap-3 p-5">
            <Skeleton class="h-10 w-full" />
            <Skeleton class="h-10 w-full" />
        </div>
    {:else if devices.length === 0}
        <div class="card">
            <EmptyState
                title="Nenhum filho vinculado"
                description="Gere um código acima e peça para o filho digitar no app dele."
            >
                {#snippet icon()}
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                        stroke-width="1.5" class="h-5 w-5">
                        <circle cx="5" cy="5" r="2" />
                        <circle cx="11" cy="5" r="2" />
                        <path d="M2 13c0-1.66 1.79-3 3.5-3 .83 0 1.59.31 2.16.82M14 13c0-1.66-1.79-3-3.5-3-.83 0-1.59.31-2.16.82"
                            stroke-linecap="round" />
                    </svg>
                {/snippet}
            </EmptyState>
        </div>
    {:else}
        <ul class="card divide-y divide-border overflow-hidden">
            {#each devices as device (device.id)}
                <li class="flex items-center justify-between gap-4 px-5 py-3">
                    <div class="min-w-0">
                        <div class="truncate text-sm font-medium text-text">
                            {device.device_name}
                        </div>
                        <div class="mt-0.5 text-xs text-text-dim">
                            {device.platform === 'windows' ? 'Windows' : 'Android'}
                            · vinculado {relativeTime(device.created_at)}
                        </div>
                    </div>
                    <button
                        type="button"
                        onclick={() => handleRevoke(device)}
                        disabled={revokingId === device.id}
                        class="btn-ghost text-xs text-danger hover:bg-danger/10 disabled:opacity-50"
                    >
                        {revokingId === device.id ? 'Desvinculando…' : 'Desvincular'}
                    </button>
                </li>
            {/each}
        </ul>
    {/if}
</section>
