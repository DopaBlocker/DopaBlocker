import type { User as FirebaseSdkUser } from 'firebase/auth';
import { writable } from 'svelte/store';

import { ApiError, api } from '../services/api';
import {
    anonymousAuthProvider,
    childAuthProvider,
    firebaseAuthProvider,
    setAuthProvider,
} from '../services/auth-provider';
import {
    currentFirebaseUser,
    getIdToken,
    onAuthChange,
    signInEmail,
    signInGoogle,
    signOutCurrent,
    signUpEmail,
} from '../services/firebase';
import {
    clearChildSession as clearChildSessionDb,
    loadChildSession as loadChildSessionDb,
    saveChildSession as saveChildSessionDb,
} from '../services/tauri-bridge';
import type { BlockMode, ChildSession, ConfirmLinkRequest, User } from '../types';

export type AuthPhase =
    | 'booting'
    | 'signed_out'
    | 'authenticating'
    | 'pending_local_registration'
    | 'backend_unavailable'
    | 'authenticated'
    | 'child_session';

export interface FirebaseIdentity {
    uid: string;
    email: string;
    display_name: string;
    provider_id?: string;
}

export interface AuthState {
    phase: AuthPhase;
    user: User | null;
    firebase_user: FirebaseIdentity | null;
    /** Preenchido apenas quando phase === 'child_session' — sessão de filho
     * sem conta Firebase. Contém o `dt_<token>` (já com prefixo). */
    child: ChildSession | null;
    loading: boolean;
    error: string | null;
}

export const AUTH_BOOTING_STATE: AuthState = {
    phase: 'booting',
    user: null,
    firebase_user: null,
    child: null,
    loading: true,
    error: null,
};

const RESOLVE_ON_LOGIN = new Set<AuthPhase>([
    'authenticated',
    'pending_local_registration',
]);
const REJECT_ON_LOGIN = new Set<AuthPhase>(['backend_unavailable', 'signed_out']);

interface PendingAction {
    resolveOn: Set<AuthPhase>;
    rejectOn: Set<AuthPhase>;
    resolve: (state: AuthState) => void;
    reject: (error: Error) => void;
}

function createAuthStore() {
    const { subscribe, set } = writable<AuthState>(AUTH_BOOTING_STATE);

    let initialized = false;
    let snapshot: AuthState = AUTH_BOOTING_STATE;
    let authSyncVersion = 0;
    let pendingAction: PendingAction | null = null;
    let nextSignedOutError: string | null = null;

    function commit(
        next: Omit<AuthState, 'loading' | 'child'> & {
            loading?: boolean;
            child?: ChildSession | null;
        },
    ) {
        const child = next.child ?? null;
        snapshot = {
            phase: next.phase,
            user: next.user,
            firebase_user: next.firebase_user,
            child,
            error: next.error,
            loading: next.loading ?? isLoadingPhase(next.phase),
        };
        // Mantém o AuthProvider corrente sincronizado com a fase. Qualquer
        // request via api.ts vai usar o provider que escolhemos aqui.
        syncAuthProvider(snapshot);
        set(snapshot);
        settlePendingAction(snapshot);
    }

    function syncAuthProvider(state: AuthState) {
        if (state.phase === 'child_session' && state.child) {
            setAuthProvider(childAuthProvider(state.child.device_token));
        } else if (
            state.phase === 'authenticated' ||
            state.phase === 'authenticating' ||
            state.phase === 'pending_local_registration' ||
            state.phase === 'backend_unavailable'
        ) {
            setAuthProvider(firebaseAuthProvider);
        } else {
            // booting, signed_out → sem credencial.
            setAuthProvider(anonymousAuthProvider);
        }
    }

    function beginPendingAction(resolveOn: Set<AuthPhase>, rejectOn: Set<AuthPhase>) {
        pendingAction?.reject(new Error('Fluxo de autenticação interrompido.'));
        return new Promise<AuthState>((resolve, reject) => {
            pendingAction = { resolveOn, rejectOn, resolve, reject };
        });
    }

    function settlePendingAction(state: AuthState) {
        if (!pendingAction) return;

        if (pendingAction.resolveOn.has(state.phase)) {
            const current = pendingAction;
            pendingAction = null;
            current.resolve(state);
            return;
        }

        if (pendingAction.rejectOn.has(state.phase)) {
            const current = pendingAction;
            pendingAction = null;
            current.reject(new Error(state.error ?? fallbackPhaseMessage(state.phase)));
        }
    }

    function init() {
        if (initialized) return;
        initialized = true;

        // Antes de escutar Firebase, tenta restaurar sessão de filho do
        // SQLCipher. Se existir e for válida, fica em `child_session`;
        // qualquer evento Firebase posterior (que não deveria acontecer
        // porque o filho não loga no Firebase) é ignorado pelo
        // `authSyncVersion`.
        void hydrateFromChildSession();

        try {
            onAuthChange((fbUser) => {
                // Filho não toca Firebase — se já restauramos child_session,
                // ignoramos eventos do Firebase SDK.
                if (snapshot.phase === 'child_session') return;
                void hydrateFromFirebase(fbUser);
            });
        } catch (err) {
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: friendly(err),
            });
        }
    }

    async function hydrateFromChildSession() {
        try {
            const session = await loadChildSessionDb();
            if (!session) return;

            // Coloca o provider antes da primeira request — a chamada de
            // validação precisa do header `dt_<token>`.
            setAuthProvider(childAuthProvider(session.device_token));
            const syncVersion = ++authSyncVersion;

            try {
                // Validar que o token ainda funciona. `/blocklist` é read-only
                // e aceita Device Token, então é o smoke test mais barato.
                await api.listBlocklist();
                if (syncVersion !== authSyncVersion) return;

                commit({
                    phase: 'child_session',
                    user: null,
                    firebase_user: null,
                    child: session,
                    error: null,
                });
            } catch (err) {
                if (syncVersion !== authSyncVersion) return;

                if (isUnauthorized(err)) {
                    // Pai revogou — limpa storage e cai em signed_out.
                    await clearChildSessionDb().catch(() => undefined);
                    commit({
                        phase: 'signed_out',
                        user: null,
                        firebase_user: null,
                        error: 'Este dispositivo foi desvinculado. Peça um novo código ao responsável.',
                    });
                } else {
                    // Backend offline — mantém child_session, UI mostra erro
                    // e próxima request tentará de novo.
                    commit({
                        phase: 'child_session',
                        user: null,
                        firebase_user: null,
                        child: session,
                        error: friendly(err),
                    });
                }
            }
        } catch (err) {
            console.warn('hydrateFromChildSession', err);
        }
    }

    async function hydrateFromFirebase(fbUser: FirebaseSdkUser | null) {
        const syncVersion = ++authSyncVersion;

        if (!fbUser) {
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: consumeSignedOutError(),
            });
            return snapshot;
        }

        const firebaseIdentity = toFirebaseIdentity(fbUser);
        commit({
            phase: 'authenticating',
            user: null,
            firebase_user: firebaseIdentity,
            error: null,
        });

        try {
            const user = await api.login();
            if (syncVersion !== authSyncVersion) return snapshot;

            commit({
                phase: 'authenticated',
                user,
                firebase_user: identityFromUser(user),
                error: null,
            });
        } catch (err) {
            if (syncVersion !== authSyncVersion) return snapshot;

            if (err instanceof ApiError && err.status === 404) {
                commit({
                    phase: 'pending_local_registration',
                    user: null,
                    firebase_user: firebaseIdentity,
                    error: null,
                });
                return snapshot;
            }

            if (isUnauthorized(err)) {
                await expireFirebaseSession(
                    'Sua sessão Firebase expirou, foi removida ou ficou inválida. Entre novamente.',
                );
                return snapshot;
            }

            commit({
                phase: 'backend_unavailable',
                user: null,
                firebase_user: firebaseIdentity,
                error: friendly(err),
            });
        }

        return snapshot;
    }

    async function login(email: string, password: string) {
        commit({
            phase: 'authenticating',
            user: snapshot.user,
            firebase_user: snapshot.firebase_user,
            error: null,
        });

        const completion = beginPendingAction(RESOLVE_ON_LOGIN, REJECT_ON_LOGIN);

        try {
            await signInEmail(email, password);
        } catch (err) {
            const message = friendly(err);
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: message,
            });
            throw asError(err, message);
        }

        await completion;
    }

    async function loginGoogle() {
        commit({
            phase: 'authenticating',
            user: snapshot.user,
            firebase_user: snapshot.firebase_user,
            error: null,
        });

        const completion = beginPendingAction(RESOLVE_ON_LOGIN, REJECT_ON_LOGIN);

        try {
            await signInGoogle();
        } catch (err) {
            const message = friendly(err);
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: message,
            });
            throw asError(err, message);
        }

        await completion;
    }

    async function register(
        email: string,
        password: string,
        displayName: string,
        mode: BlockMode,
        emailVerificationToken?: string,
    ) {
        commit({
            phase: 'authenticating',
            user: null,
            firebase_user: snapshot.firebase_user,
            error: null,
        });

        try {
            await signUpEmail(email, password, displayName);
            await completeLocalRegistration(mode, displayName, emailVerificationToken);
        } catch (err) {
            if (snapshot.phase === 'authenticating') {
                const message = friendly(err);
                const fbUser = currentFirebaseUser();
                const keepsCurrentAttempt = sameEmail(fbUser?.email, email);

                if (fbUser && !keepsCurrentAttempt) {
                    await signOutCurrent().catch(() => undefined);
                }

                commit({
                    phase: fbUser && keepsCurrentAttempt
                        ? 'pending_local_registration'
                        : 'signed_out',
                    user: null,
                    firebase_user: fbUser && keepsCurrentAttempt
                        ? toFirebaseIdentity(fbUser)
                        : null,
                    error: message,
                });
            }
            throw asError(err, friendly(err));
        }
    }

    async function completeLocalRegistration(
        mode: BlockMode,
        displayName?: string,
        emailVerificationToken?: string,
    ) {
        const fbUser = currentFirebaseUser();
        if (!fbUser) {
            const message = 'Sua sessão expirou. Entre novamente para concluir o cadastro.';
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: message,
            });
            throw new Error(message);
        }

        const firebaseIdentity = toFirebaseIdentity(fbUser);
        const resolvedDisplayName =
            displayName?.trim() ||
            firebaseIdentity.display_name ||
            fallbackDisplayName(firebaseIdentity.email);
        const syncVersion = ++authSyncVersion;

        commit({
            phase: 'authenticating',
            user: null,
            firebase_user: firebaseIdentity,
            error: null,
        });

        try {
            await getIdToken(true);
            const user = await api.register({
                email: firebaseIdentity.email,
                display_name: resolvedDisplayName,
                mode,
                ...(emailVerificationToken
                    ? { email_verification_token: emailVerificationToken }
                    : {}),
            });

            if (syncVersion !== authSyncVersion) return snapshot.user;

            commit({
                phase: 'authenticated',
                user,
                firebase_user: identityFromUser(user),
                error: null,
            });
            return user;
        } catch (initialErr) {
            let resolvedError: unknown = initialErr;

            if (initialErr instanceof ApiError && initialErr.status === 409) {
                try {
                    const user = await api.login();
                    if (syncVersion !== authSyncVersion) return snapshot.user;

                    commit({
                        phase: 'authenticated',
                        user,
                        firebase_user: identityFromUser(user),
                        error: null,
                    });
                    return user;
                } catch (loginErr) {
                    resolvedError = loginErr;
                }
            }

            if (syncVersion !== authSyncVersion) {
                throw asError(resolvedError, friendly(resolvedError));
            }

            if (isUnauthorized(resolvedError)) {
                await expireFirebaseSession(
                    'Sua sessão Firebase expirou, foi removida ou ficou inválida. Entre novamente.',
                );
                throw asError(
                    resolvedError,
                    'Sua sessão Firebase expirou, foi removida ou ficou inválida. Entre novamente.',
                );
            }

            const message = friendly(resolvedError);
            commit({
                phase: 'pending_local_registration',
                user: null,
                firebase_user: firebaseIdentity,
                error: message,
            });
            throw asError(resolvedError, message);
        }
    }

    async function retryBackendSync() {
        const fbUser = currentFirebaseUser();
        if (!fbUser) {
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: 'Entre novamente para continuar.',
            });
            return;
        }

        await hydrateFromFirebase(fbUser);
    }

    async function logout() {
        ++authSyncVersion;
        nextSignedOutError = null;
        // Cobre os dois cenários: sessão Firebase (Pessoal/Pais) e sessão
        // de filho. Limpamos os dois sempre — os erros são silenciados porque
        // queremos terminar em `signed_out` independente.
        try {
            if (snapshot.phase === 'child_session') {
                await clearChildSessionDb().catch(() => undefined);
            } else {
                await signOutCurrent();
            }
        } finally {
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: null,
            });
        }
    }

    /// Fluxo "Filhos": sem Firebase, sem email/senha. Recebe o código de 6
    /// dígitos que o pai gerou, chama POST /devices/link/confirm (rota pública),
    /// recebe o `dt_<token>`, persiste em SQLCipher e entra em child_session.
    async function confirmChildCode(payload: ConfirmLinkRequest) {
        ++authSyncVersion;
        commit({
            phase: 'authenticating',
            user: null,
            firebase_user: null,
            error: null,
        });

        try {
            // Garante que NÃO mandamos credencial nesta request — o backend
            // exige rota pública (sem header).
            setAuthProvider(anonymousAuthProvider);
            const resp = await api.confirmLinkCode(payload);

            const session: ChildSession = {
                user_id: resp.user_id,
                device_id: resp.device_id,
                device_token: resp.device_token,
                parent_device_id: resp.parent_device_id,
            };
            await saveChildSessionDb(session);

            commit({
                phase: 'child_session',
                user: null,
                firebase_user: null,
                child: session,
                error: null,
            });
        } catch (err) {
            const message = friendly(err);
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: message,
            });
            throw asError(err, message);
        }
    }

    function clearError() {
        commit({
            phase: snapshot.phase,
            user: snapshot.user,
            firebase_user: snapshot.firebase_user,
            error: null,
            loading: snapshot.loading,
        });
    }

    function consumeSignedOutError() {
        const error = nextSignedOutError;
        nextSignedOutError = null;
        return error;
    }

    async function expireFirebaseSession(message: string) {
        nextSignedOutError = message;
        try {
            await signOutCurrent();
        } catch {
            nextSignedOutError = null;
            commit({
                phase: 'signed_out',
                user: null,
                firebase_user: null,
                error: message,
            });
        }
    }

    return {
        subscribe,
        init,
        login,
        loginGoogle,
        register,
        completeLocalRegistration,
        confirmChildCode,
        retryBackendSync,
        logout,
        clearError,
    };
}

function isLoadingPhase(phase: AuthPhase): boolean {
    return phase === 'booting' || phase === 'authenticating';
}

function toFirebaseIdentity(fbUser: FirebaseSdkUser): FirebaseIdentity {
    return {
        uid: fbUser.uid,
        email: fbUser.email?.trim() ?? '',
        display_name: fbUser.displayName?.trim() || fallbackDisplayName(fbUser.email),
        provider_id: fbUser.providerData[0]?.providerId,
    };
}

function identityFromUser(user: User): FirebaseIdentity {
    return {
        uid: user.firebase_uid,
        email: user.email,
        display_name: user.display_name,
    };
}

function fallbackDisplayName(email: string | null | undefined): string {
    const localPart = email?.split('@')[0]?.trim();
    return localPart || 'Usuário';
}

function fallbackPhaseMessage(phase: AuthPhase): string {
    switch (phase) {
        case 'backend_unavailable':
            return 'Não foi possível falar com o backend local.';
        case 'signed_out':
            return 'Entre novamente para continuar.';
        default:
            return 'Não foi possível concluir a autenticação.';
    }
}

function asError(err: unknown, fallback: string): Error {
    return err instanceof Error ? err : new Error(fallback);
}

function isUnauthorized(err: unknown): boolean {
    return err instanceof ApiError && err.status === 401;
}

function sameEmail(a: string | null | undefined, b: string | null | undefined): boolean {
    return (a?.trim().toLowerCase() ?? '') === (b?.trim().toLowerCase() ?? '');
}

function friendly(err: unknown): string {
    if (err instanceof ApiError) return err.message;

    if (err instanceof Error) {
        const code = (err as { code?: string }).code;
        switch (code) {
            case 'auth/invalid-credential':
            case 'auth/wrong-password':
            case 'auth/user-not-found':
                return 'Email ou senha incorretos';
            case 'auth/email-already-in-use':
                return 'Este email já existe no Firebase. Entre e conclua o cadastro local.';
            case 'auth/weak-password':
                return 'A senha precisa ter pelo menos 6 caracteres';
            case 'auth/invalid-email':
                return 'Email inválido';
            case 'auth/popup-closed-by-user':
            case 'auth/cancelled-popup-request':
                return 'Login cancelado';
            case 'auth/network-request-failed':
                return 'Sem conexão com a internet';
            default:
                return err.message;
        }
    }

    return String(err);
}

export const authStore = createAuthStore();
