// Store global de autenticação. Responsabilidades:
//   - Reagir ao estado do Firebase Auth (onAuthStateChanged).
//   - Sincronizar com o backend local (POST /auth/login) quando o usuário
//     existir, ou marcar "pending-register" quando o JWT é válido mas não
//     há user local ainda.
//   - Expor ações login/register/logout/loginGoogle com tratamento de erro
//     amigável (mensagens em português).

import { writable } from 'svelte/store';
import type { BlockMode, User } from '../types';
import { ApiError, api } from '../services/api';
import {
    getIdToken,
    onAuthChange,
    signInEmail,
    signInGoogle,
    signOutCurrent,
    signUpEmail,
} from '../services/firebase';

export interface AuthState {
    user: User | null;
    loading: boolean;
    /**
     * - `null` = sem erro.
     * - `"pending-register"` = Firebase autenticou mas o user local não existe.
     *    A UI deve mostrar o seletor de modo para completar o cadastro.
     * - Qualquer outra string = mensagem legível pra exibir em toast/form.
     */
    error: string | null;
}

function createAuthStore() {
    const { subscribe, set, update } = writable<AuthState>({
        user: null,
        loading: true,
        error: null,
    });

    let initialized = false;

    function init() {
        if (initialized) return;
        initialized = true;

        onAuthChange(async (fbUser) => {
            if (!fbUser) {
                set({ user: null, loading: false, error: null });
                return;
            }
            update((s) => ({ ...s, loading: true }));
            try {
                const user = await api.login();
                set({ user, loading: false, error: null });
            } catch (err) {
                if (err instanceof ApiError && err.status === 404) {
                    set({ user: null, loading: false, error: 'pending-register' });
                } else {
                    set({ user: null, loading: false, error: friendly(err) });
                }
            }
        });
    }

    async function login(email: string, password: string) {
        update((s) => ({ ...s, loading: true, error: null }));
        try {
            await signInEmail(email, password);
            // A resolução final vem via onAuthChange.
        } catch (err) {
            update((s) => ({ ...s, loading: false, error: friendly(err) }));
            throw err;
        }
    }

    async function loginGoogle() {
        update((s) => ({ ...s, loading: true, error: null }));
        try {
            await signInGoogle();
        } catch (err) {
            update((s) => ({ ...s, loading: false, error: friendly(err) }));
            throw err;
        }
    }

    async function register(
        email: string,
        password: string,
        displayName: string,
        mode: BlockMode,
    ) {
        update((s) => ({ ...s, loading: true, error: null }));
        try {
            await signUpEmail(email, password, displayName);
            await getIdToken(true);
            const user = await api.register({
                email,
                display_name: displayName,
                mode,
            });
            set({ user, loading: false, error: null });
        } catch (err) {
            update((s) => ({ ...s, loading: false, error: friendly(err) }));
            throw err;
        }
    }

    async function logout() {
        try {
            await signOutCurrent();
        } finally {
            set({ user: null, loading: false, error: null });
        }
    }

    /// Limpa a mensagem de erro sem mexer no restante do estado. Usar após
    /// mostrar um toast para não re-exibir no próximo render.
    function clearError() {
        update((s) => ({ ...s, error: null }));
    }

    return { subscribe, init, login, loginGoogle, register, logout, clearError };
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
                return 'Este email já está cadastrado';
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
