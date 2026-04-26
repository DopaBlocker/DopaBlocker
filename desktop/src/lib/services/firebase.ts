// Inicialização do Firebase Auth SDK + wrappers tipados.
//
// O app não faz nada com Firestore — apenas Auth. Firestore/sync cloud é
// responsabilidade exclusiva do backend Axum.
//
// Providers habilitados nesta etapa: email/senha e Google (popup). Outros
// providers ficam pra v0.2.
//
// Em Tauri 2, `signInWithPopup` depende de o domínio do app estar listado
// em Firebase Console → Authentication → Settings → Authorized domains. Em
// dev é `localhost`; em bundle, o esquema `tauri://localhost` precisa ser
// adicionado manualmente.

import { initializeApp, type FirebaseApp } from 'firebase/app';
import {
    GoogleAuthProvider,
    browserLocalPersistence,
    createUserWithEmailAndPassword,
    deleteUser,
    getAuth,
    onAuthStateChanged,
    sendPasswordResetEmail,
    setPersistence,
    signInWithEmailAndPassword,
    signInWithPopup,
    signOut,
    updateProfile,
    type Auth,
    type User as FirebaseUser,
} from 'firebase/auth';

const firebaseConfig = {
    apiKey: import.meta.env.VITE_FIREBASE_API_KEY ?? '',
    authDomain: import.meta.env.VITE_FIREBASE_AUTH_DOMAIN ?? '',
    projectId: import.meta.env.VITE_FIREBASE_PROJECT_ID ?? '',
    appId: import.meta.env.VITE_FIREBASE_APP_ID ?? '',
    messagingSenderId: import.meta.env.VITE_FIREBASE_MSG_SENDER_ID ?? '',
};

let app: FirebaseApp | null = null;
let authInstance: Auth | null = null;

function getFirebaseAuth(): Auth {
    if (authInstance) return authInstance;
    if (!firebaseConfig.apiKey) {
        throw new Error(
            'Firebase não configurado. Copie desktop/.env.example para desktop/.env e preencha VITE_FIREBASE_*.',
        );
    }
    app = initializeApp(firebaseConfig);
    authInstance = getAuth(app);
    // Persistência local padrão já funciona, mas explicitar ajuda o usuário
    // continuar logado ao reabrir a janela do Tauri.
    setPersistence(authInstance, browserLocalPersistence).catch((err) => {
        console.warn('setPersistence falhou:', err);
    });
    return authInstance;
}

export async function signInEmail(email: string, password: string): Promise<FirebaseUser> {
    const cred = await signInWithEmailAndPassword(getFirebaseAuth(), email, password);
    return cred.user;
}

export async function signUpEmail(
    email: string,
    password: string,
    displayName: string,
): Promise<FirebaseUser> {
    const cred = await createUserWithEmailAndPassword(getFirebaseAuth(), email, password);
    if (displayName) {
        await updateProfile(cred.user, { displayName });
    }
    return cred.user;
}

export async function signInGoogle(): Promise<FirebaseUser> {
    const provider = new GoogleAuthProvider();
    const cred = await signInWithPopup(getFirebaseAuth(), provider);
    return cred.user;
}

export async function signOutCurrent(): Promise<void> {
    await signOut(getFirebaseAuth());
}

/// Dispara o email "reset de senha" do Firebase. O usuário clica no link,
/// define a nova senha numa página hospedada pelo Firebase, e depois volta
/// ao app para fazer login com a nova senha. Backend não precisa saber.
export async function sendPasswordReset(email: string): Promise<void> {
    await sendPasswordResetEmail(getFirebaseAuth(), email);
}

/// Apaga o user atual no Firebase Auth. Para o backend ser limpo, chame
/// `api.deleteAccount()` ANTES — o backend usa o JWT do user (que expira
/// junto com o user) para autorizar.
///
/// Pode falhar com `auth/requires-recent-login` se o último login foi há
/// muito tempo (política do Firebase). O caller deve capturar e pedir
/// reauth.
export async function deleteCurrentUser(): Promise<void> {
    const user = getFirebaseAuth().currentUser;
    if (!user) {
        throw new Error('Não há usuário Firebase autenticado.');
    }
    await deleteUser(user);
}

export function onAuthChange(cb: (user: FirebaseUser | null) => void): () => void {
    return onAuthStateChanged(getFirebaseAuth(), cb);
}

/// Retorna o ID token atual do Firebase. `force=true` dispara refresh — usar
/// quando o backend responder 401 (token provavelmente expirado).
export async function getIdToken(force = false): Promise<string | null> {
    const user = getFirebaseAuth().currentUser;
    if (!user) return null;
    return user.getIdToken(force);
}

export function currentFirebaseUser(): FirebaseUser | null {
    return getFirebaseAuth().currentUser;
}
