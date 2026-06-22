<!--
  Layout raiz: app shell + guard de rota.
  - Inicializa authStore uma vez (onMount) para registrar o listener do Firebase.
  - Redireciona para /login quando não autenticado em rota protegida, e para /
    quando autenticado em /login.
  - Sidebar minimalista com logo mark, link ativo destacado por barra lateral
    e SVGs inline (sem dependência externa).
-->
<script lang="ts">
    import { goto } from '$app/navigation';
    import { page } from '$app/state';
    import { onMount } from 'svelte';
    import { AUTH_BOOTING_STATE, authStore, type AuthState } from '$lib/stores/auth';
    import { blockingStore } from '$lib/stores/blocking';
    import OnboardingModal from '$lib/components/OnboardingModal.svelte';
    import ConfirmModal from '$lib/components/ui/ConfirmModal.svelte';
    import ToastContainer from '$lib/components/ui/Toast.svelte';
    import BrandMark from '$lib/components/ui/BrandMark.svelte';
    import '../app.css';

    const PUBLIC_ROUTES = ['/welcome', '/login', '/onboarding/child'];
    const CHILD_ROUTE = '/child-blocked';
    const ONBOARDING_KEY_PREFIX = 'dopablocker:onboarding:';

    let { children } = $props();
    let auth: AuthState = $state({ ...AUTH_BOOTING_STATE });
    let onboardingOpen = $state(false);

    // User cujo auto-sync da blocklist está ativo. Só (re)inicia o poll quando
    // o usuário muda, evitando reiniciar a cada emissão do auth store.
    let autoSyncUserId: string | null = null;

    onMount(() => {
        authStore.init();
        const unsub = authStore.subscribe((s) => {
            auth = s;
            // Onboarding: primeira vez que um user aparece autenticado neste
            // localStorage, abre o modal. Escopo por-user pra que múltiplas
            // contas na mesma máquina vejam cada uma uma vez.
            if (s.user) {
                const key = ONBOARDING_KEY_PREFIX + s.user.id;
                if (localStorage.getItem(key) !== 'done') {
                    onboardingOpen = true;
                }
            }

            // Auto-sync da blocklist para sessões Firebase (pessoal/pai): mantém
            // o cache local (de onde o engine lê) em dia com mudanças feitas em
            // OUTRO device. O filho tem o próprio poll em /child-blocked.
            const activeUserId = s.phase === 'authenticated' ? (s.user?.id ?? null) : null;
            if (activeUserId !== autoSyncUserId) {
                autoSyncUserId = activeUserId;
                if (activeUserId) blockingStore.startAutoSync(activeUserId);
                else blockingStore.stopAutoSync();
            }
        });
        return () => {
            blockingStore.stopAutoSync();
            unsub();
        };
    });

    function completeOnboarding() {
        onboardingOpen = false;
        if (auth.user) {
            localStorage.setItem(ONBOARDING_KEY_PREFIX + auth.user.id, 'done');
        }
    }

    function isPublicRoute(path: string): boolean {
        return PUBLIC_ROUTES.some((r) => path === r || path.startsWith(r + '/'));
    }

    // Conta autenticada cobre tanto sessão Firebase (Pessoal/Pais) quanto
    // sessão de filho (sem Firebase, com Device Token).
    const isAuthenticated = $derived(
        auth.phase === 'authenticated' || auth.phase === 'child_session',
    );
    const isChild = $derived(auth.phase === 'child_session');
    const isParental = $derived(!isChild && auth.user?.mode === 'parental');

    $effect(() => {
        if (auth.phase === 'booting' || auth.phase === 'authenticating') return;
        const path = page.url.pathname;
        const publicRoute = isPublicRoute(path);

        if (isChild) {
            // Sessão de filho fica AMARRADA em /child-blocked — sem dashboard,
            // sem /blocking, sem nada. Só a tela "Bloqueado".
            if (path !== CHILD_ROUTE) {
                goto(CHILD_ROUTE, { replaceState: true });
            }
            return;
        }

        if (auth.phase === 'authenticated') {
            // Firebase: bloqueia rotas públicas e a rota de filho.
            if (publicRoute || path === CHILD_ROUTE) {
                goto('/', { replaceState: true });
            }
            return;
        }

        // signed_out / qualquer outro estado terminal: redireciona pra /welcome
        // exceto se já está numa rota pública (incluindo /child-blocked, que
        // pode aparecer brevemente antes do logout do filho propagar).
        if (!publicRoute && path !== CHILD_ROUTE) {
            goto('/welcome', { replaceState: true });
        }
    });

    let logoutConfirmOpen = $state(false);

    function requestLogout() {
        logoutConfirmOpen = true;
    }

    function confirmLogout() {
        logoutConfirmOpen = false;
        void authStore.logout();
    }

    type NavIcon = 'dashboard' | 'shield' | 'settings' | 'parental';
    const navLinks = $derived<{ href: string; label: string; icon: NavIcon }[]>([
        { href: '/', label: 'Início', icon: 'dashboard' },
        { href: '/blocking', label: 'Bloqueios', icon: 'shield' },
        ...(isParental
            ? [{ href: '/parental', label: 'Filhos', icon: 'parental' as const }]
            : []),
        { href: '/settings', label: 'Conta', icon: 'settings' },
    ]);

    function isActive(href: string, path: string) {
        if (href === '/') return path === '/';
        return path === href || path.startsWith(href + '/');
    }

    // Texto exibido no card da sidebar — varia entre sessão Firebase e child.
    const sidebarPrimary = $derived(
        isChild
            ? 'Dispositivo vinculado'
            : (auth.user?.display_name || auth.user?.email || ''),
    );
    const sidebarSecondary = $derived(
        isChild ? 'Modo Filho (read-only)' : (auth.user?.email ?? ''),
    );
</script>

{#if auth.phase === 'booting' || auth.phase === 'authenticating'}
    <div class="flex min-h-screen items-center justify-center bg-bg">
        <div class="text-xs text-text-muted">Carregando…</div>
    </div>
{:else if !isAuthenticated || isPublicRoute(page.url.pathname)}
    {@render children()}
{:else}
    <div class="flex min-h-screen bg-bg text-text">
        <aside
            class="flex w-60 flex-col border-r border-border bg-surface px-3 py-5"
        >
            <!-- Logo mark + wordmark -->
            <div class="flex items-center gap-2.5 px-2 pb-6">
                <BrandMark size="xs" />
                <div class="flex flex-col leading-tight">
                    <span class="text-[13px] font-semibold tracking-tight text-gradient">DopaBlocker</span>
                    <span class="text-[10px] uppercase tracking-widest text-text-dim">
                        Foco
                    </span>
                </div>
            </div>

            <nav class="flex flex-col gap-0.5">
                {#each navLinks as link (link.href)}
                    {@const active = isActive(link.href, page.url.pathname)}
                    <a
                        href={link.href}
                        class="group relative flex items-center gap-2.5 rounded-md px-3 py-2 text-sm transition-colors"
                        class:bg-surface-2={active}
                        class:text-text={active}
                        class:text-text-muted={!active}
                        class:hover:bg-surface-2={!active}
                        class:hover:text-text={!active}
                    >
                        <!-- Barra lateral indicadora. -->
                        {#if active}
                            <span
                                class="absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-full bg-primary"
                            ></span>
                        {/if}
                        <span class="flex h-4 w-4 items-center justify-center">
                            {#if link.icon === 'dashboard'}
                                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                                    stroke-width="1.5" class="h-4 w-4">
                                    <rect x="2" y="2" width="5" height="5" rx="1" />
                                    <rect x="9" y="2" width="5" height="5" rx="1" />
                                    <rect x="2" y="9" width="5" height="5" rx="1" />
                                    <rect x="9" y="9" width="5" height="5" rx="1" />
                                </svg>
                            {:else if link.icon === 'shield'}
                                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                                    stroke-width="1.5" class="h-4 w-4">
                                    <path d="M8 2l5 2v4a6 6 0 01-5 6 6 6 0 01-5-6V4l5-2z"
                                        stroke-linecap="round" stroke-linejoin="round" />
                                </svg>
                            {:else if link.icon === 'parental'}
                                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                                    stroke-width="1.5" class="h-4 w-4">
                                    <circle cx="5" cy="5" r="2" />
                                    <circle cx="11" cy="5" r="2" />
                                    <path d="M2 13c0-1.66 1.79-3 3.5-3 .83 0 1.59.31 2.16.82M14 13c0-1.66-1.79-3-3.5-3-.83 0-1.59.31-2.16.82"
                                        stroke-linecap="round" />
                                </svg>
                            {:else}
                                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor"
                                    stroke-width="1.5" class="h-4 w-4">
                                    <circle cx="8" cy="8" r="2.25" />
                                    <path
                                        d="M13 8c0-.3-.03-.6-.09-.9l1.26-1L13 4l-1.45.56a5 5 0 00-1.55-.9L9.75 2h-3.5l-.25 1.66a5 5 0 00-1.55.9L3 4l-1.17 2.1 1.26 1A5 5 0 003 8c0 .3.03.6.09.9l-1.26 1L3 12l1.45-.56a5 5 0 001.55.9l.25 1.66h3.5l.25-1.66a5 5 0 001.55-.9L13 12l1.17-2.1-1.26-1c.06-.3.09-.6.09-.9z"
                                        stroke-linecap="round" stroke-linejoin="round" />
                                </svg>
                            {/if}
                        </span>
                        {link.label}
                    </a>
                {/each}
            </nav>

            <div class="mt-auto flex flex-col gap-3 pt-6">
                <div class="rounded-md border border-border bg-surface-2 px-3 py-2.5">
                    <div class="truncate text-xs font-medium text-text">
                        {sidebarPrimary}
                    </div>
                    <div class="truncate text-[11px] text-text-dim">
                        {sidebarSecondary}
                    </div>
                </div>
                <button type="button" onclick={requestLogout} class="btn-ghost w-full justify-start">
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"
                        class="h-4 w-4">
                        <path d="M10 11l3-3-3-3M13 8H6M9 13H3a1 1 0 01-1-1V4a1 1 0 011-1h6"
                            stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                    Sair
                </button>
            </div>
        </aside>

        <main class="flex-1 overflow-y-auto px-10 py-8">
            <div class="mx-auto max-w-4xl">
                {@render children()}
            </div>
        </main>
    </div>

    <OnboardingModal open={onboardingOpen} onclose={completeOnboarding} />
    <ConfirmModal
        open={logoutConfirmOpen}
        title="Sair da conta?"
        message="Você vai precisar entrar de novo pra continuar usando. O bloqueio ativo continua rodando se estiver ligado."
        confirmLabel="Sair"
        cancelLabel="Ficar"
        danger
        onconfirm={confirmLogout}
        oncancel={() => (logoutConfirmOpen = false)}
    />
{/if}

<ToastContainer />
