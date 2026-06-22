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
    import type { BlockMode } from '$lib/types';
    import ConfirmModal from '$lib/components/ui/ConfirmModal.svelte';
    import DeleteAccountModal from '$lib/components/DeleteAccountModal.svelte';

    let auth: AuthState = $state({ ...AUTH_BOOTING_STATE });
    let appVersion: string | null = $state(null);
    let confirmOpen = $state(false);
    let deleteOpen = $state(false);
    let childCount = $state(0);
    let switchingMode = $state(false);
    let modeConfirmOpen = $state(false);

    const currentMode = $derived(auth.user?.mode ?? 'personal');
    const targetMode = $derived<BlockMode>(currentMode === 'parental' ? 'personal' : 'parental');

    onMount(() => {
        const unsub = authStore.subscribe((s) => (auth = s));
        void getAppVersion()
            .then((v) => (appVersion = v))
            .catch(() => (appVersion = null));
        // Conta os filhos vinculados para avisar antes de sair do modo parental.
        void api
            .listDevices()
            .then((devices) => (childCount = devices.filter((d) => d.is_child).length))
            .catch(() => (childCount = 0));
        return unsub;
    });

    /// Trocar de modo (personal↔parental) sem recriar a conta. Se estiver saindo
    /// do parental com filhos vinculados, confirma antes (os vínculos continuam,
    /// mas o pai deixa de gerenciá-los enquanto estiver em pessoal).
    function requestSwitchMode() {
        if (currentMode === 'parental' && childCount > 0) {
            modeConfirmOpen = true;
            return;
        }
        void doSwitchMode();
    }

    async function doSwitchMode() {
        modeConfirmOpen = false;
        if (switchingMode) return;
        switchingMode = true;
        try {
            const user = await authStore.updateMode(targetMode);
            toast.info(
                user.mode === 'parental'
                    ? 'Modo alterado para Pais.'
                    : 'Modo alterado para Pessoal.',
            );
        } catch (err) {
            toast.error(err instanceof Error ? err.message : 'Não foi possível trocar o modo.');
        } finally {
            switchingMode = false;
        }
    }

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

    /// Ordem importa: BACKEND primeiro (com o token Firebase ainda válido), só
    /// depois o Firebase. Se invertêssemos, após `deleteUser()` o `getIdToken()`
    /// volta `null` e o `DELETE /auth/me` sairia sem token (401) — deixando o
    /// user órfão no backend e o email "preso" (UNIQUE), o que quebra o
    /// recadastro. Se o backend falhar, propagamos (o modal mostra o erro) e NÃO
    /// tocamos no Firebase. Se o Firebase exigir `requires-recent-login`, o
    /// backend já foi apagado (email livre) e o modal cai no passo de reauth.
    async function handleDeleteAccount() {
        await api.deleteAccount();
        await deleteCurrentUser();
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
        <div class="field-label">Conta</div>
        <h1 class="mt-1 text-2xl font-semibold tracking-tight text-text">Sua conta</h1>
    </header>

    <div class="card-padded flex items-center gap-4">
        <div
            class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full text-sm font-semibold text-white"
            style="background: linear-gradient(135deg, var(--brand-from) 0%, var(--brand-to) 100%)"
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
                <dd class="num text-text-dim">{appVersion ?? '—'}</dd>
            </div>
        </dl>
    </div>

    <!-- Trocar de modo sem recriar a conta. A regra "pai imune" e a propagação
         dos bloqueios passam a valer no próximo sync. -->
    <div class="card-padded">
        <div class="flex items-center justify-between gap-4">
            <div class="min-w-0">
                <div class="text-sm font-medium text-text">Modo de uso</div>
                <p class="mt-1 text-xs text-text-muted">
                    Você está em
                    <strong class="text-text">
                        {currentMode === 'parental' ? 'Pais' : 'Pessoal'}
                    </strong>.
                    {currentMode === 'parental'
                        ? 'No modo Pessoal os bloqueios passam a valer para você também.'
                        : 'No modo Pais você gerencia os bloqueios dos dispositivos dos filhos.'}
                </p>
            </div>
            <button
                type="button"
                onclick={requestSwitchMode}
                disabled={switchingMode}
                class="btn-secondary shrink-0"
            >
                {switchingMode
                    ? 'Trocando…'
                    : targetMode === 'parental'
                      ? 'Mudar para Pais'
                      : 'Mudar para Pessoal'}
            </button>
        </div>
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

<ConfirmModal
    open={modeConfirmOpen}
    title="Sair do modo Pais?"
    message={`Você tem ${childCount} dispositivo(s) de filho vinculado(s). No modo Pessoal você deixa de gerenciar os bloqueios deles (os vínculos continuam). Dá para voltar para Pais quando quiser.`}
    confirmLabel="Mudar para Pessoal"
    cancelLabel="Cancelar"
    onconfirm={doSwitchMode}
    oncancel={() => (modeConfirmOpen = false)}
/>

<DeleteAccountModal
    open={deleteOpen}
    onclose={() => (deleteOpen = false)}
    onconfirm={handleDeleteAccount}
    onreauth={handleReauth}
/>
