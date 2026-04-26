// =============================================================================
// AuthProvider — abstracao "qual token mandar nesta request?".
// =============================================================================
// O backend aceita dois tipos de credencial (ver backend/src/middleware.rs):
//
//   1. Firebase JWT       → "Authorization: Bearer eyJhbGci..."
//   2. Device Token       → "Authorization: Bearer dt_<plain>"
//
// O cliente HTTP (api.ts) não deveria saber qual o usuário atual; ele apenas
// pergunta a um AuthProvider "qual o header?" e segue. Quando o usuário é
// uma sessão de filho (sem Firebase), trocamos o provider sem mexer no api.ts.
//
// Esta interface é o mesmo contrato que o mobile vai implementar em Dart
// (`abstract class AuthProvider` com duas concretas: FirebaseAuthProvider e
// ChildAuthProvider). Manter as duas em paridade facilita o port.
// =============================================================================

import { getIdToken } from './firebase';

export interface AuthProvider {
    /// Devolve o valor do header `Authorization`, ou `null` se não há
    /// credencial disponível (rotas públicas — register, link/confirm, etc.).
    getAuthHeader(): Promise<string | null>;

    /// Tentativa de obter um token "fresco" após uma resposta 401. Apenas o
    /// Firebase tem refresh — Device Tokens não expiram (até revogados pelo
    /// pai). Retorna `false` quando não há como atualizar — nesse caso o
    /// cliente HTTP não deve fazer retry.
    refresh(): Promise<boolean>;
}

/// Provider para contas Pessoal/Pais — busca o JWT do Firebase Auth SDK.
export const firebaseAuthProvider: AuthProvider = {
    async getAuthHeader() {
        const token = await getIdToken();
        return token ? `Bearer ${token}` : null;
    },
    async refresh() {
        const token = await getIdToken(true);
        return token !== null;
    },
};

/// Provider para sessão de filho. Recebe o `dt_<plain>` já com prefixo.
/// Sem refresh — o token vale até o pai revogar (rota POST /devices/:id/revoke).
export function childAuthProvider(deviceToken: string): AuthProvider {
    return {
        async getAuthHeader() {
            return `Bearer ${deviceToken}`;
        },
        async refresh() {
            return false;
        },
    };
}

/// Provider noop — usado em rotas públicas (register, login, email-code/*,
/// devices/link/confirm) e enquanto o auth store está em "booting" ou
/// "signed_out". Não bloqueia a request, só não manda Authorization.
export const anonymousAuthProvider: AuthProvider = {
    async getAuthHeader() {
        return null;
    },
    async refresh() {
        return false;
    },
};

// -----------------------------------------------------------------------------
// Provider corrente, mutavel.
// -----------------------------------------------------------------------------
// O auth store (stores/auth.ts) chama `setAuthProvider` quando a fase muda:
//   - signed_out / booting       → anonymousAuthProvider
//   - authenticating / authenticated / pending_local_registration → firebaseAuthProvider
//   - child_session              → childAuthProvider(token)
// O api.ts le este provider em cada request.

let current: AuthProvider = anonymousAuthProvider;

export function setAuthProvider(provider: AuthProvider): void {
    current = provider;
}

export function currentAuthProvider(): AuthProvider {
    return current;
}
