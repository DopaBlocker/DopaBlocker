<!--
  Login/cadastro. Abas Entrar/Cadastrar.
  - Entrar: email+senha OU Google.
  - Cadastrar: nome+email+senha + ModeSelector. Clicar em "Pessoal" dispara o
    cadastro; Pais/Filhos mostram banner "Em breve" e não avançam.
-->
<script lang="ts">
    import { authStore } from '$lib/stores/auth';
    import ModeSelector from './ModeSelector.svelte';

    type Tab = 'signin' | 'signup';

    let tab: Tab = $state('signin');
    let email = $state('');
    let password = $state('');
    let displayName = $state('');
    let formError: string | null = $state(null);
    let infoMessage: string | null = $state(null);
    let submitting = $state(false);

    function resetFeedback() {
        formError = null;
        infoMessage = null;
    }

    function switchTab(next: Tab) {
        tab = next;
        resetFeedback();
    }

    async function handleSignIn(e: Event) {
        e.preventDefault();
        if (submitting) return;
        resetFeedback();
        submitting = true;
        try {
            await authStore.login(email.trim(), password);
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            submitting = false;
        }
    }

    async function handleGoogle() {
        if (submitting) return;
        resetFeedback();
        submitting = true;
        try {
            await authStore.loginGoogle();
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            submitting = false;
        }
    }

    async function handleModeSelect(mode: 'personal' | 'parent' | 'child') {
        if (submitting) return;
        resetFeedback();

        if (mode !== 'personal') {
            infoMessage = 'Esse modo chega na v0.2 — por ora só o Pessoal está disponível.';
            return;
        }

        const name = displayName.trim();
        const mail = email.trim();
        if (!name || !mail || !password) {
            formError = 'Preencha nome, email e senha antes de escolher o modo.';
            return;
        }

        submitting = true;
        try {
            await authStore.register(mail, password, name, 'personal');
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            submitting = false;
        }
    }
</script>

<div class="w-full max-w-md">
    <!-- Logo mark + wordmark. -->
    <div class="mb-8 flex flex-col items-center gap-3 text-center">
        <div
            class="flex h-10 w-10 items-center justify-center rounded-lg"
            style="background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-secondary) 100%)"
        >
            <div class="h-4 w-4 rounded-sm bg-white/90"></div>
        </div>
        <div>
            <h1 class="text-lg font-semibold tracking-tight text-text">DopaBlocker</h1>
            <p class="mt-1 text-xs text-text-muted">
                Bloqueie distrações. Mantenha o foco.
            </p>
        </div>
    </div>

    <div class="mb-6 flex rounded-md border border-border bg-surface-2 p-1">
        <button
            type="button"
            onclick={() => switchTab('signin')}
            class="flex-1 rounded px-3 py-1.5 text-sm font-medium transition-colors"
            class:bg-primary={tab === 'signin'}
            class:text-white={tab === 'signin'}
            class:text-text-muted={tab !== 'signin'}
            class:hover:text-text={tab !== 'signin'}
        >
            Entrar
        </button>
        <button
            type="button"
            onclick={() => switchTab('signup')}
            class="flex-1 rounded px-3 py-1.5 text-sm font-medium transition-colors"
            class:bg-primary={tab === 'signup'}
            class:text-white={tab === 'signup'}
            class:text-text-muted={tab !== 'signup'}
            class:hover:text-text={tab !== 'signup'}
        >
            Cadastrar
        </button>
    </div>

    {#if tab === 'signin'}
        <form class="flex flex-col gap-4" onsubmit={handleSignIn}>
            <label class="flex flex-col gap-1.5">
                <span class="field-label">Email</span>
                <input
                    type="email"
                    required
                    autocomplete="email"
                    bind:value={email}
                    class="input"
                    placeholder="voce@exemplo.com"
                />
            </label>
            <label class="flex flex-col gap-1.5">
                <span class="field-label">Senha</span>
                <input
                    type="password"
                    required
                    autocomplete="current-password"
                    bind:value={password}
                    class="input"
                    placeholder="••••••••"
                />
            </label>
            <button type="submit" disabled={submitting} class="btn-primary mt-2 w-full">
                {submitting ? 'Entrando…' : 'Entrar'}
            </button>
        </form>

        <div class="my-5 flex items-center gap-3">
            <div class="h-px flex-1 bg-border"></div>
            <span class="text-[10px] uppercase tracking-widest text-text-dim">ou</span>
            <div class="h-px flex-1 bg-border"></div>
        </div>

        <button
            type="button"
            onclick={handleGoogle}
            disabled={submitting}
            class="btn-secondary w-full"
        >
            <svg viewBox="0 0 24 24" class="h-4 w-4">
                <path fill="#4285F4"
                    d="M22.5 12.3c0-.8-.1-1.6-.2-2.3H12v4.4h5.9c-.3 1.4-1 2.6-2.2 3.4v2.8h3.5c2.1-1.9 3.3-4.7 3.3-8.3z" />
                <path fill="#34A853"
                    d="M12 23c3 0 5.5-1 7.3-2.7L15.8 18c-1 .7-2.3 1.1-3.8 1.1-2.9 0-5.3-2-6.2-4.6H2.2v2.9C4 20.9 7.7 23 12 23z" />
                <path fill="#FBBC05"
                    d="M5.8 14.5c-.2-.7-.4-1.4-.4-2.2 0-.8.1-1.5.4-2.2V7.2H2.2a11 11 0 000 9.7l3.6-2.4z" />
                <path fill="#EA4335"
                    d="M12 5.4c1.6 0 3.1.6 4.2 1.7l3.1-3.1C17.4 2.3 14.9 1 12 1 7.7 1 4 3.1 2.2 6.3l3.6 2.8C6.7 7.4 9.1 5.4 12 5.4z" />
            </svg>
            Entrar com Google
        </button>
    {:else}
        <form
            class="flex flex-col gap-4"
            onsubmit={(e) => {
                e.preventDefault();
                void handleModeSelect('personal');
            }}
        >
            <label class="flex flex-col gap-1.5">
                <span class="field-label">Nome</span>
                <input
                    type="text"
                    required
                    autocomplete="name"
                    bind:value={displayName}
                    class="input"
                    placeholder="Seu nome"
                />
            </label>
            <label class="flex flex-col gap-1.5">
                <span class="field-label">Email</span>
                <input
                    type="email"
                    required
                    autocomplete="email"
                    bind:value={email}
                    class="input"
                    placeholder="voce@exemplo.com"
                />
            </label>
            <label class="flex flex-col gap-1.5">
                <span class="field-label">Senha</span>
                <input
                    type="password"
                    required
                    minlength={6}
                    autocomplete="new-password"
                    bind:value={password}
                    class="input"
                    placeholder="Mínimo 6 caracteres"
                />
            </label>

            <div class="mt-2">
                <div class="field-label mb-2">Escolha o modo</div>
                <ModeSelector onselect={handleModeSelect} />
            </div>
        </form>
    {/if}

    {#if formError}
        <div
            class="mt-4 rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-xs text-danger"
        >
            {formError}
        </div>
    {/if}
    {#if infoMessage}
        <div
            class="mt-4 rounded-md border border-secondary/50 bg-secondary/10 px-3 py-2 text-xs text-secondary"
        >
            {infoMessage}
        </div>
    {/if}
</div>
