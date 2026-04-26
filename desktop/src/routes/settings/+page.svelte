<!--
  Configurações. Mostra dados da conta e toggles rápidos. O toggle do engine
  vive na página /blocking; aqui só mostra conta + versão + atalho para sair.
-->
<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { AUTH_BOOTING_STATE, authStore, type AuthState } from '$lib/stores/auth';
    import { api } from '$lib/services/api';
    import { deleteCurrentUser } from '$lib/services/firebase';
    import { getAppVersion } from '$lib/services/tauri-bridge';
    import { toast } from '$lib/stores/toast';
    import ConfirmModal from '$lib/components/ui/ConfirmModal.svelte';
    import DeleteAccountModal from '$lib/components/DeleteAccountModal.svelte';

    let auth: AuthState = $state({ ...AUTH_BOOTING_STATE });
    let appVersion: string | null = $state(null);
    let confirmOpen = $state(false);
    let deleteOpen = $state(false);

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

    /// Ordem importa: Firebase primeiro (porque pode falhar com
    /// `auth/requires-recent-login` antes de mexer no backend), só depois
    /// o backend, só depois logout local. Se o Firebase OK mas o backend
    /// falhar, ficamos com Firebase apagado e backend órfão — log de erro
    /// e seguimos para o logout. O órfão morre quando o user tentar logar
    /// (auth/login retorna 404 → re-cadastro).
    async function handleDeleteAccount() {
        await deleteCurrentUser();
        try {
            await api.deleteAccount();
        } catch (err) {
            console.warn('Backend delete falhou após Firebase deletar:', err);
        }
        await authStore.logout();
        toast.info('Conta excluída.');
        await goto('/welcome');
    }

    /// Reauth: o user clica "Fazer login de novo". Logout volta para
    /// /welcome; o user escolhe o modo, faz login e volta em /settings.
    async function handleReauth() {
        deleteOpen = false;
        await authStore.logout();
        await goto('/welcome');
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

    <!-- Zona de perigo. Ações irreversíveis ficam isoladas visualmente para
         o usuário não confundir com o "Sair" comum. -->
    <div class="card-padded border-danger/30">
        <div class="flex items-center justify-between gap-4">
            <div class="min-w-0">
                <div class="text-sm font-medium text-danger">
                    Excluir conta permanentemente
                </div>
                <p class="mt-1 text-xs text-text-muted">
                    Apaga sua conta, todos os bloqueios, todos os filhos
                    vinculados (se houver) e o login no Firebase. Não dá para
                    desfazer.
                </p>
            </div>
            <button
                type="button"
                onclick={() => (deleteOpen = true)}
                class="shrink-0 rounded-md border border-danger px-3 py-1.5 text-xs font-medium text-danger transition-colors hover:bg-danger hover:text-white"
            >
                Excluir conta
            </button>
        </div>
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

<DeleteAccountModal
    open={deleteOpen}
    onclose={() => (deleteOpen = false)}
    onconfirm={handleDeleteAccount}
    onreauth={handleReauth}
/>
