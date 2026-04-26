// =============================================================================
// AuthProvider — abstracao "qual token mandar nesta request?".
// =============================================================================
// O backend aceita dois tipos de credencial (ver backend/src/middleware.rs):
//
//   1. Firebase JWT       → "Authorization: Bearer eyJhbGci..."
//   2. Device Token       → "Authorization: Bearer dt_<plain>"
//
// O cliente HTTP (api.ts) nao deveria saber qual o usuario atual; ele apenas
// pergunta a um AuthProvider "qual o header?" e segue. Quando o usuario eh
// uma sessao de filho (sem Firebase), trocamos o provider sem mexer no api.ts.
//
// Esta interface eh o mesmo contrato que o mobile vai implementar em Dart
// (`abstract class AuthProvider` com duas concretas: FirebaseAuthProvider e
// ChildAuthProvider). Manter as duas em paridade facilita o port.
// =============================================================================

import { getIdToken } from './firebase';

export interface AuthProvider {
    /// Devolve o valor do header `Authorization`, ou `null` se nao ha
    /// credencial disponivel (rotas publicas — register, link/confirm, etc.).
    getAuthHeader(): Promise<string | null>;

    /// Tentativa de obter um token "fresco" apos uma resposta 401. Apenas o
    /// Firebase tem refresh — Device Tokens nao expiram (ate revogados pelo
    /// pai). Retorna `false` quando nao ha como atualizar — nesse caso o
    /// cliente HTTP nao deve fazer retry.
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

/// Provider para sessao de filho. Recebe o `dt_<plain>` ja com prefixo.
/// Sem refresh — o token vale ate o pai revogar (rota POST /devices/:id/revoke).
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

/// Provider noop — usado em rotas publicas (register, login, email-code/*,
/// devices/link/confirm) e enquanto o auth store esta em "booting" ou
/// "signed_out". Nao bloqueia a request, so nao manda Authorization.
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
