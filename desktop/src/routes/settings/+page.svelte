<!--
  Configurações. Mostra dados da conta e toggles rápidos. O toggle do engine
  vive na página /blocking; aqui só mostra conta + versão + atalho para sair.
-->
<script lang="ts">
    import { onMount } from 'svelte';
    import { AUTH_BOOTING_STATE, authStore, type AuthState } from '$lib/stores/auth';
    import { getAppVersion } from '$lib/services/tauri-bridge';
    import ConfirmModal from '$lib/components/ui/ConfirmModal.svelte';

    let auth: AuthState = $state({ ...AUTH_BOOTING_STATE });
    let appVersion: string | null = $state(null);
    let confirmOpen = $state(false);

    onMount(() => {
        const unsub = authStore.subscribe((s) => (auth = s));
        void getAppVersion()
            .then((v) => (appVersion = v))
            .catch(() => (appVersion = null));
        return unsub;
    });

    function initials(name: string): string {
        return name
            .split(' ')
            .filter(Boolean)
            .slice(0, 2)
            .map((p) => p[0]!.toUpperCase())
            .join('');
    }

    function confirmLogout() {
        confirmOpen = false;
        void authStore.logout();
    }
</script>

<div class="flex flex-col gap-6">
    <header>
        <div class="field-label">Configurações</div>
        <h1 class="mt-1 text-2xl font-semibold tracking-tight text-text">Conta</h1>
    </header>

    <div class="card-padded flex items-center gap-4">
        <div
            class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full text-sm font-semibold text-white"
            style="background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-secondary) 100%)"
        >
            {auth.user ? initials(auth.user.display_name || auth.user.email) : '?'}
        </div>
        <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium text-text">
                {auth.user?.display_name || '—'}
            </div>
            <div class="truncate text-xs text-text-muted">{auth.user?.email || '—'}</div>
        </div>
        <span class="badge-primary">
            {auth.user?.mode === 'parental' ? 'Parental' : 'Pessoal'}
        </span>
    </div>

    <div class="card-padded">
        <div class="field-label mb-3">Detalhes</div>
        <dl class="flex flex-col gap-3 text-sm">
            <div class="flex items-center justify-between">
                <dt class="text-text-muted">Nome</dt>
                <dd class="text-text">{auth.user?.display_name ?? '—'}</dd>
            </div>
            <div class="h-px bg-border"></div>
            <div class="flex items-center justify-between">
                <dt class="text-text-muted">Email</dt>
                <dd class="text-text">{auth.user?.email ?? '—'}</dd>
            </div>
            <div class="h-px bg-border"></div>
            <div class="flex items-center justify-between">
                <dt class="text-text-muted">Modo</dt>
                <dd class="text-text">
                    {auth.user?.mode === 'parental' ? 'Parental' : 'Pessoal'}
                </dd>
            </div>
            <div class="h-px bg-border"></div>
            <div class="flex items-center justify-between">
                <dt class="text-text-muted">Versão</dt>
                <dd class="text-text-dim">{appVersion ?? '—'}</dd>
            </div>
        </dl>
    </div>

    <div class="flex justify-end">
        <button type="button" onclick={() => (confirmOpen = true)} class="btn-danger">
            Sair da conta
        </button>
    </div>
</div>

<ConfirmModal
    open={confirmOpen}
    title="Sair da conta?"
    message="Você vai precisar entrar de novo pra continuar usando. O bloqueio ativo continua rodando se estiver ligado."
    confirmLabel="Sair"
    cancelLabel="Ficar"
    danger
    onconfirm={confirmLogout}
    oncancel={() => (confirmOpen = false)}
/>
