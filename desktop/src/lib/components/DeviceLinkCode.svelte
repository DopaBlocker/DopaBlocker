<!--
  Botão "Gerar código de vinculação" + display do código de 6 dígitos com
  countdown até o `expires_at`. Só existe no modo pai — o filho usa o
  componente irmão `ChildCodeInput.svelte` para INSERIR o código.
-->
<script lang="ts">
    import { onDestroy } from 'svelte';
    import { get } from 'svelte/store';
    import { api, ApiError } from '$lib/services/api';
    import { ensureOwnerDeviceRegistered } from '$lib/services/device-registration';
    import { authStore } from '$lib/stores/auth';
    import { toast } from '$lib/stores/toast';

    interface Generated {
        code: string;
        expires_at: string; // ISO 8601 UTC
    }

    let pending: Generated | null = $state(null);
    let remaining = $state(0);
    let generating = $state(false);
    let timer: number | null = null;

    async function generate() {
        if (generating) return;
        generating = true;
        try {
            const auth = get(authStore);
            if (auth.user?.id) {
                await ensureOwnerDeviceRegistered(auth.user.id);
            }
            const resp = await api.generateLinkCode();
            pending = resp;
            updateRemaining();
            startTimer();
        } catch (err) {
            toast.error(err instanceof ApiError ? err.message : 'Falha ao gerar código');
        } finally {
            generating = false;
        }
    }

    function startTimer() {
        if (timer !== null) window.clearInterval(timer);
        timer = window.setInterval(updateRemaining, 1000);
    }

    function updateRemaining() {
        if (!pending) {
            remaining = 0;
            return;
        }
        const expiresAt = new Date(pending.expires_at).getTime();
        const ms = Math.max(0, expiresAt - Date.now());
        remaining = Math.ceil(ms / 1000);
        if (remaining === 0) {
            // Código expirou — limpa para forçar gerar novo.
            pending = null;
            if (timer !== null) {
                window.clearInterval(timer);
                timer = null;
            }
        }
    }

    function formatRemaining(s: number): string {
        const m = Math.floor(s / 60);
        const r = s % 60;
        return `${m}:${r.toString().padStart(2, '0')}`;
    }

    function formatCode(c: string): string {
        // "123456" → "123 456" (mais legível para ditar em voz alta)
        return c.length === 6 ? `${c.slice(0, 3)} ${c.slice(3)}` : c;
    }

    onDestroy(() => {
        if (timer !== null) window.clearInterval(timer);
    });
</script>

<section class="card-padded flex flex-col gap-4">
    <div>
        <h2 class="text-sm font-semibold text-text">Código de vinculação</h2>
        <p class="mt-1 text-xs text-text-muted">
            Gere um código e peça para o filho digitar no app dele. Vale por 5
            minutos.
        </p>
    </div>

    {#if pending}
        <div class="flex flex-col items-center gap-2 rounded-md border border-border bg-surface-2 px-4 py-6">
            <div class="font-mono text-3xl font-semibold tracking-widest text-text">
                {formatCode(pending.code)}
            </div>
            <div class="text-xs text-text-dim">
                Expira em {formatRemaining(remaining)}
            </div>
        </div>
        <button type="button" onclick={generate} disabled={generating} class="btn-secondary">
            Gerar outro código
        </button>
    {:else}
        <button type="button" onclick={generate} disabled={generating} class="btn-primary">
            {generating ? 'Gerando…' : 'Gerar código de vinculação'}
        </button>
    {/if}
</section>
