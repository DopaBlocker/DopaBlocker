<!--
  Tela exibida quando o auth.phase === 'child_session'. Minimal por design:
  o filho não deve ter onde clicar nem o que ler além do essencial.

  Faz polling a cada 30s para detectar revogação do pai. Em 401, dispara
  logout — o `+layout.svelte` redireciona automaticamente para /welcome
  quando phase passa a `signed_out`.

  O DNS proxy continua rodando em background; o bloqueio efetivo não depende
  desta tela estar aberta.
-->
<script lang="ts">
    import { onDestroy, onMount } from 'svelte';
    import { get } from 'svelte/store';
    import BrandMark from '$lib/components/ui/BrandMark.svelte';
    import { ApiError, api } from '$lib/services/api';
    import { authStore } from '$lib/stores/auth';
    import * as bridge from '$lib/services/tauri-bridge';
    import { getAppVersion } from '$lib/services/tauri-bridge';

    const POLL_INTERVAL_MS = 30_000;

    let appVersion: string | null = $state(null);
    let pollTimer: number | null = null;

    async function pollBlocklist() {
        const userId = get(authStore).child?.user_id;
        try {
            // /blocklist é a rota mais barata aceita por Device Token. Se o pai
            // revogou, vem 401 e o ApiError propaga (→ logout).
            const items = await api.listBlocklist();
            // B2: aplica as edições do pai ao cache local de onde o engine lê,
            // para propagarem em ~30s sem reabrir o app. Filho aplica tudo.
            if (userId) {
                await bridge
                    .saveBlocklist(userId, items, { mode: 'parental', is_child: true })
                    .catch((e) => console.warn('Falha ao espelhar blocklist no cache:', e));
            }
        } catch (err) {
            if (err instanceof ApiError && err.status === 401) {
                // Limpa SQLCipher + atualiza store. O layout redireciona.
                await authStore.logout();
            }
            // Outros erros (rede, 5xx) — ignora; tenta de novo no próximo tick.
        }
    }

    onMount(() => {
        void getAppVersion()
            .then((v) => (appVersion = v))
            .catch(() => (appVersion = null));

        // Primeira checagem imediata, depois a cada 30s.
        void pollBlocklist();
        pollTimer = window.setInterval(() => void pollBlocklist(), POLL_INTERVAL_MS);
    });

    onDestroy(() => {
        if (pollTimer !== null) window.clearInterval(pollTimer);
    });
</script>

<div class="relative flex min-h-screen flex-col items-center justify-center overflow-hidden bg-bg p-6">
    <!-- Glow de marca atrás do conteúdo. -->
    <div
        class="pointer-events-none absolute left-1/2 top-1/3 h-80 w-80 -translate-x-1/2 -translate-y-1/2 rounded-full opacity-10 blur-3xl"
        style="background: linear-gradient(135deg, var(--brand-from), var(--brand-to))"
    ></div>
    <div class="relative flex flex-col items-center gap-6 text-center">
        <BrandMark size="lg" />

        <div class="flex flex-col gap-2">
            <h1 class="text-3xl font-semibold tracking-tight text-text">Bloqueado</h1>
            <p class="max-w-sm text-sm text-text-muted">
                Este dispositivo está vinculado à conta de um responsável. O
                bloqueio está ativo em segundo plano.
            </p>
        </div>
    </div>

    <!-- Versao discreta no rodape. -->
    <div class="absolute bottom-4 text-[10px] text-text-dim">
        DopaBlocker desktop {#if appVersion}<span class="num">v{appVersion}</span>{/if}
    </div>
</div>
