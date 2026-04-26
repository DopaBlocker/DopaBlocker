<!--
  Tela "Filhos": vincula este dispositivo a uma conta de pai usando o codigo
  de 6 digitos. Sem email, sem senha, sem Firebase. Apos confirmar, o backend
  retorna um Device Token que e persistido em SQLCipher e o store entra em
  `child_session`.

  Esta tela nao deve aparecer para usuarios ja autenticados — o `+layout.svelte`
  cuida de redirecionar.
-->
<script lang="ts">
    import { goto } from '$app/navigation';
    import ChildCodeInput from '$lib/components/ChildCodeInput.svelte';
    import { authStore } from '$lib/stores/auth';

    let submitting = $state(false);
    let error: string | null = $state(null);
    let inputRef: { clear: () => void } | undefined = $state();

    async function handleSubmit(code: string) {
        if (submitting) return;
        error = null;
        submitting = true;
        try {
            await authStore.confirmChildCode({
                code,
                device_name: detectDeviceName(),
                platform: 'windows',
            });
            await goto('/');
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            inputRef?.clear();
        } finally {
            submitting = false;
        }
    }

    function detectDeviceName(): string {
        // Sem API nativa de hostname no Tauri 2 sem plugin extra; usar um
        // identificador generico — o pai pode renomear depois (futuro).
        return 'Computador do filho';
    }
</script>

<div class="flex min-h-screen items-center justify-center bg-surface p-6">
    <div class="w-full max-w-md">
        <div class="mb-8 flex flex-col items-center gap-3 text-center">
            <div
                class="flex h-10 w-10 items-center justify-center rounded-lg"
                style="background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-secondary) 100%)"
            >
                <div class="h-4 w-4 rounded-sm bg-white/90"></div>
            </div>
            <div>
                <h1 class="text-lg font-semibold tracking-tight text-gradient">
                    Vincular este dispositivo
                </h1>
                <p class="mt-1 text-xs text-text-muted">
                    Digite o codigo de 6 digitos que o seu responsavel gerou.
                </p>
            </div>
        </div>

        <ChildCodeInput
            bind:this={inputRef}
            disabled={submitting}
            {error}
            onsubmit={handleSubmit}
        />

        <button
            type="button"
            onclick={() => goto('/login')}
            disabled={submitting}
            class="btn-ghost mt-6 w-full justify-center text-xs"
        >
            Nao e voce? Voltar
        </button>
    </div>
</div>
