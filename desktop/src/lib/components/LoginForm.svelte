<script lang="ts">
    import { onMount } from 'svelte';

    import {
        AUTH_BOOTING_STATE,
        authStore,
        type AuthState,
    } from '$lib/stores/auth';
    import { api } from '$lib/services/api';
    import { sendPasswordReset } from '$lib/services/firebase';

    type Tab = 'signin' | 'signup';
    type SignupStep = 'form' | 'verify';

    interface Props {
        /** Vem da URL (`?mode=personal` ou `?mode=parental`). Filhos não
         *  passam por aqui — vão direto para /onboarding/child. */
        mode: 'personal' | 'parental';
    }

    let { mode }: Props = $props();

    let auth: AuthState = $state({ ...AUTH_BOOTING_STATE });
    let tab: Tab = $state('signin');
    let email = $state('');
    let password = $state('');
    let confirmPassword = $state('');
    let displayName = $state('');
    let signupStep: SignupStep = $state('form');
    let verificationEmail = $state('');
    let verificationCode = $state('');
    let resendAvailableAt = $state(0);
    let resendRemaining = $state(0);
    let formError: string | null = $state(null);
    let infoMessage: string | null = $state(null);
    let submitting = $state(false);
    let lastFirebaseIdentityUid: string | null = $state(null);

    const pendingLocalRegistration = $derived(auth.phase === 'pending_local_registration');
    const backendUnavailable = $derived(auth.phase === 'backend_unavailable');
    const firebaseIdentity = $derived(auth.firebase_user);
    const pendingProviderSkipsCode = $derived(
        pendingLocalRegistration && firebaseIdentity?.provider_id === 'google.com',
    );

    onMount(() => {
        const unsubscribe = authStore.subscribe((state) => {
            auth = state;

            const shouldAdoptFirebaseIdentity =
                state.phase === 'pending_local_registration' ||
                state.firebase_user?.uid !== lastFirebaseIdentityUid;

            if (
                state.firebase_user?.email &&
                (!email.trim() || shouldAdoptFirebaseIdentity)
            ) {
                email = state.firebase_user.email;
            }

            if (
                state.firebase_user?.display_name &&
                (!displayName.trim() || shouldAdoptFirebaseIdentity)
            ) {
                displayName = state.firebase_user.display_name;
            }

            lastFirebaseIdentityUid = state.firebase_user?.uid ?? null;

            if (state.phase === 'pending_local_registration') {
                tab = 'signup';
                password = '';
                confirmPassword = '';
                infoMessage =
                    'Sua conta já entrou no Firebase. Falta concluir o cadastro local para entrar no app.';
                formError = state.error;
                return;
            }

            if (state.phase === 'backend_unavailable') {
                infoMessage =
                    'O Firebase autenticou, mas o backend local não respondeu. Tente sincronizar novamente.';
                formError = state.error;
                return;
            }

            if (state.error) {
                formError = state.error;
            }
        });

        const timer = window.setInterval(refreshResendRemaining, 1000);
        return () => {
            unsubscribe();
            window.clearInterval(timer);
        };
    });

    function resetFeedback() {
        formError = null;
        infoMessage = null;
        authStore.clearError();
    }

    function resetVerificationStep() {
        signupStep = 'form';
        verificationEmail = '';
        verificationCode = '';
        resendAvailableAt = 0;
        resendRemaining = 0;
    }

    function setResendCooldown(seconds: number) {
        resendAvailableAt = Date.now() + seconds * 1000;
        refreshResendRemaining();
    }

    function refreshResendRemaining() {
        if (!resendAvailableAt) {
            resendRemaining = 0;
            return;
        }

        resendRemaining = Math.max(
            0,
            Math.ceil((resendAvailableAt - Date.now()) / 1000),
        );
    }

    function switchTab(next: Tab) {
        if (pendingLocalRegistration && next === 'signin') {
            tab = 'signup';
            infoMessage = 'Conclua o cadastro local abaixo para destravar o app.';
            return;
        }

        tab = next;
        resetFeedback();
        if (next === 'signin') resetVerificationStep();
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

    async function handleForgotPassword() {
        if (submitting) return;
        resetFeedback();

        const target = email.trim();
        if (!target) {
            formError = 'Digite seu email para receber o link de redefinição.';
            return;
        }

        submitting = true;
        try {
            await sendPasswordReset(target);
            infoMessage = `Enviamos um link de redefinição para ${target}. Confira sua caixa de entrada.`;
        } catch (err) {
            // Por segurança, o Firebase não distingue email inexistente — qualquer
            // erro vira a mensagem genérica. Logamos no console para debug local.
            console.warn('sendPasswordReset', err);
            const code = (err as { code?: string }).code;
            if (code === 'auth/invalid-email') {
                formError = 'Email inválido.';
            } else if (code === 'auth/network-request-failed') {
                formError = 'Sem conexão com a internet.';
            } else {
                infoMessage = `Se ${target} estiver cadastrado, você receberá um email com o link.`;
            }
        } finally {
            submitting = false;
        }
    }

    async function handleRetryBackendSync() {
        if (submitting) return;

        resetFeedback();
        submitting = true;
        try {
            await authStore.retryBackendSync();
        } finally {
            submitting = false;
        }
    }

    function resolveSignupIdentity() {
        const name =
            displayName.trim() ||
            firebaseIdentity?.display_name ||
            email.trim().split('@')[0] ||
            '';
        const mail = firebaseIdentity?.email || email.trim();

        if (!mail) {
            formError = 'Não conseguimos ler o email da sua sessão. Entre novamente.';
            return null;
        }

        if (!name) {
            formError = 'Preencha seu nome antes de continuar.';
            return null;
        }

        if (!pendingLocalRegistration && !password) {
            formError = 'Preencha nome, email e senha antes de escolher o modo.';
            return null;
        }

        if (!pendingLocalRegistration && password.length < 6) {
            formError = 'A senha precisa ter pelo menos 6 caracteres.';
            return null;
        }

        if (!pendingLocalRegistration && !confirmPassword) {
            formError = 'Confirme sua senha antes de continuar.';
            return null;
        }

        if (!pendingLocalRegistration && password !== confirmPassword) {
            formError = 'As senhas não conferem.';
            return null;
        }

        return { mail, name };
    }

    /// Inicia o cadastro: envia código de verificação por email. O `mode` já
    /// vem da prop (escolhido em /welcome), então não precisamos mais de lógica
    /// de seleção aqui — basta validar o form, mandar código, ir para o step
    /// de verificação.
    ///
    /// Para login Google já autenticado mas sem registro local
    /// (`pendingLocalRegistration`), pulamos o código: o email já foi
    /// verificado pelo Google.
    async function handleSignupSubmit(e: Event) {
        e.preventDefault();
        if (submitting) return;

        resetFeedback();
        const identity = resolveSignupIdentity();
        if (!identity) return;

        // Caminho Google → só falta criar o user local.
        if (pendingProviderSkipsCode) {
            submitting = true;
            try {
                await authStore.completeLocalRegistration(mode, identity.name);
            } catch (err) {
                formError = err instanceof Error ? err.message : String(err);
            } finally {
                submitting = false;
            }
            return;
        }

        // Caminho normal (email + senha) → manda código.
        submitting = true;
        try {
            const response = await api.startEmailVerification({ email: identity.mail });
            verificationEmail = identity.mail;
            signupStep = 'verify';
            setResendCooldown(response.resend_after_seconds);
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            submitting = false;
        }
    }

    async function handleVerifyAndRegister(e?: Event) {
        e?.preventDefault();
        if (submitting) return;

        resetFeedback();

        const identity = resolveSignupIdentity();
        if (!identity) return;

        const code = verificationCode.trim();
        if (!code) {
            formError = 'Digite o código enviado por email.';
            return;
        }

        submitting = true;
        try {
            const verified = await api.verifyEmailCode({
                email: verificationEmail || identity.mail,
                code,
            });

            if (pendingLocalRegistration) {
                await authStore.completeLocalRegistration(
                    mode,
                    identity.name,
                    verified.email_verification_token,
                );
            } else {
                await authStore.register(
                    verificationEmail || identity.mail,
                    password,
                    identity.name,
                    mode,
                    verified.email_verification_token,
                );
            }
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            submitting = false;
        }
    }

    async function handleResendCode() {
        if (submitting || resendRemaining > 0) return;
        // Reaproveita o handler — joga em `signupStep='form'` virtualmente
        // chamando o mesmo fluxo de envio de código (sem perder o código
        // digitado, mas resetando o cooldown).
        resetFeedback();
        const identity = resolveSignupIdentity();
        if (!identity) return;

        submitting = true;
        try {
            const response = await api.startEmailVerification({ email: identity.mail });
            verificationEmail = identity.mail;
            setResendCooldown(response.resend_after_seconds);
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            submitting = false;
        }
    }

    function handleEditSignupEmail() {
        resetFeedback();
        resetVerificationStep();
    }

    async function handleUseAnotherAccount() {
        if (submitting) return;

        resetFeedback();
        submitting = true;
        try {
            await authStore.logout();
            email = '';
            password = '';
            confirmPassword = '';
            displayName = '';
            resetVerificationStep();
        } finally {
            submitting = false;
        }
    }
</script>

<div class="w-full max-w-md">
    <div class="mb-8 flex flex-col items-center gap-3 text-center">
        <div
            class="flex h-10 w-10 items-center justify-center rounded-lg"
            style="background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-secondary) 100%)"
        >
            <div class="h-4 w-4 rounded-sm bg-white/90"></div>
        </div>
        <div>
            <h1 class="text-lg font-semibold tracking-tight text-gradient">DopaBlocker</h1>
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
            disabled={pendingLocalRegistration}
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
            {pendingLocalRegistration ? 'Concluir cadastro' : 'Cadastrar'}
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
                    placeholder="********"
                />
            </label>
            <button type="submit" disabled={submitting} class="btn-primary mt-2 w-full">
                {submitting ? 'Entrando...' : 'Entrar'}
            </button>
            <button
                type="button"
                onclick={handleForgotPassword}
                disabled={submitting}
                class="self-end text-xs text-text-muted underline-offset-2 transition-colors hover:text-text hover:underline disabled:opacity-50"
            >
                Esqueci minha senha
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
                if (signupStep === 'verify') {
                    void handleVerifyAndRegister(e);
                } else {
                    void handleSignupSubmit(e);
                }
            }}
        >
            {#if signupStep === 'verify'}
                <div
                    class="rounded-md border border-secondary/50 bg-secondary/10 px-3 py-2 text-xs text-secondary"
                >
                    Código enviado para {verificationEmail || email.trim()}.
                </div>
                <label class="flex flex-col gap-1.5">
                    <span class="field-label">Código de verificação</span>
                    <input
                        type="text"
                        required
                        inputmode="numeric"
                        autocomplete="one-time-code"
                        maxlength={6}
                        pattern={'[0-9]{6}'}
                        bind:value={verificationCode}
                        class="input"
                        placeholder="000000"
                    />
                </label>
                <button type="submit" disabled={submitting} class="btn-primary mt-2 w-full">
                    {submitting ? 'Validando...' : 'Validar email e entrar'}
                </button>
                <div class="grid grid-cols-2 gap-2">
                    <button
                        type="button"
                        onclick={handleResendCode}
                        disabled={submitting || resendRemaining > 0}
                        class="btn-secondary w-full justify-center"
                    >
                        {resendRemaining > 0
                            ? `Reenviar em ${resendRemaining}s`
                            : 'Reenviar código'}
                    </button>
                    <button
                        type="button"
                        onclick={handleEditSignupEmail}
                        disabled={submitting}
                        class="btn-ghost w-full justify-center"
                    >
                        Editar email
                    </button>
                </div>
            {:else}
                {#if pendingLocalRegistration}
                    <div
                        class="rounded-md border border-secondary/50 bg-secondary/10 px-3 py-2 text-xs text-secondary"
                    >
                        Seu login Firebase já está pronto. Falta criar o registro local desta máquina.
                    </div>
                {/if}

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
                        readonly={pendingLocalRegistration && !!firebaseIdentity?.email}
                    />
                </label>

                {#if !pendingLocalRegistration}
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
                    <label class="flex flex-col gap-1.5">
                        <span class="field-label">Confirmar senha</span>
                        <input
                            type="password"
                            required
                            minlength={6}
                            autocomplete="new-password"
                            bind:value={confirmPassword}
                            class="input"
                            placeholder="Digite a senha novamente"
                        />
                    </label>
                {/if}

                <div
                    class="rounded-md border border-border bg-surface-2 px-3 py-2 text-xs text-text-muted"
                >
                    Modo: <strong class="text-text">
                        {mode === 'parental' ? 'Pais' : 'Pessoal'}
                    </strong>
                    <a
                        href="/welcome"
                        class="ml-1 text-primary underline-offset-2 hover:underline"
                    >
                        trocar
                    </a>
                </div>

                <button type="submit" disabled={submitting} class="btn-primary mt-2 w-full">
                    {submitting
                        ? 'Enviando…'
                        : mode === 'parental'
                          ? 'Criar conta de Pai'
                          : 'Criar conta Pessoal'}
                </button>
            {/if}
        </form>
    {/if}

    {#if backendUnavailable && firebaseIdentity}
        <button
            type="button"
            onclick={handleRetryBackendSync}
            disabled={submitting}
            class="btn-secondary mt-4 w-full"
        >
            {submitting ? 'Sincronizando...' : 'Tentar novamente com o backend local'}
        </button>
    {/if}

    {#if firebaseIdentity && auth.phase !== 'authenticated'}
        <button
            type="button"
            onclick={handleUseAnotherAccount}
            disabled={submitting}
            class="btn-ghost mt-3 w-full justify-center"
        >
            {submitting ? 'Limpando sessão...' : 'Entrar com outra conta'}
        </button>
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
