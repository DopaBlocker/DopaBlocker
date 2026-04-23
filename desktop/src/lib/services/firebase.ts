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
    getAuth,
    onAuthStateChanged,
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
