<!--
  Tela exibida quando o auth.phase === 'child_session'. Minimal por design:
  o filho nao deve ter onde clicar nem o que ler alem do essencial.

  Faz polling a cada 30s para detectar revogacao do pai. Em 401, dispara
  logout — o `+layout.svelte` redireciona automaticamente para /welcome
  quando phase passa a `signed_out`.

  O DNS proxy continua rodando em background; o bloqueio efetivo nao depende
  desta tela estar aberta.
-->
<script lang="ts">
    import { onDestroy, onMount } from 'svelte';
    import { ApiError, api } from '$lib/services/api';
    import { authStore } from '$lib/stores/auth';
    import { getAppVersion } from '$lib/services/tauri-bridge';

    const POLL_INTERVAL_MS = 30_000;

    let appVersion: string | null = $state(null);
    let pollTimer: number | null = null;

    async function checkRevoked() {
        try {
            // /blocklist e a rota mais barata aceita por Device Token. Se
            // o pai revogou, vem 401 e o ApiError propaga.
            await api.listBlocklist();
        } catch (err) {
            if (err instanceof ApiError && err.status === 401) {
                // Limpa SQLCipher + atualiza store. O layout redireciona.
                await authStore.logout();
            }
            // Outros erros (rede, 5xx) — ignora; tenta de novo no proximo tick.
        }
    }

    onMount(() => {
        void getAppVersion()
            .then((v) => (appVersion = v))
            .catch(() => (appVersion = null));

        // Primeira checagem imediata, depois a cada 30s.
        void checkRevoked();
        pollTimer = window.setInterval(() => void checkRevoked(), POLL_INTERVAL_MS);
    });

    onDestroy(() => {
        if (pollTimer !== null) window.clearInterval(pollTimer);
    });
</script>

<div class="relative flex min-h-screen flex-col items-center justify-center bg-bg p-6">
    <div class="flex flex-col items-center gap-6 text-center">
        <!-- Logo mark -->
        <div
            class="flex h-16 w-16 items-center justify-center rounded-lg"
            style="background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-secondary) 100%)"
        >
            <div class="h-7 w-7 rounded-sm bg-white/90"></div>
        </div>

        <div class="flex flex-col gap-2">
            <h1 class="text-3xl font-semibold tracking-tight text-text">Bloqueado</h1>
            <p class="max-w-sm text-sm text-text-muted">
                Este dispositivo esta vinculado a conta de um responsavel. O
                bloqueio esta ativo em segundo plano.
            </p>
        </div>
    </div>

    <!-- Versao discreta no rodape. -->
    <div class="absolute bottom-4 text-[10px] text-text-dim">
        DopaBlocker desktop {appVersion ? `v${appVersion}` : ''}
    </div>
</div>
