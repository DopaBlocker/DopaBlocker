<!--
  Tela de login. Recebe `?mode=personal` ou `?mode=parental` da URL — o
  /welcome roteia conforme o card que o usuario escolheu. Sem mode valido,
  redireciona de volta para /welcome (nao tem sentido entrar aqui sem ter
  escolhido um modo).
-->
<script lang="ts">
    import { goto } from '$app/navigation';
    import { page } from '$app/state';
    import LoginForm from '$lib/components/LoginForm.svelte';

    const VALID = new Set(['personal', 'parental']);
    const raw = $derived(page.url.searchParams.get('mode'));
    const mode = $derived<'personal' | 'parental' | null>(
        raw && VALID.has(raw) ? (raw as 'personal' | 'parental') : null,
    );

    $effect(() => {
        if (mode === null) {
            void goto('/welcome', { replaceState: true });
        }
    });
</script>

<div class="flex min-h-screen items-center justify-center bg-bg px-6">
    {#if mode}
        <LoginForm {mode} />
    {/if}
</div>
