// Tipos compartilhados pelo frontend. Espelham os structs do crate `shared`
// (shared/src/models.rs) e os DTOs do backend (backend/src/models.rs).
// Se um campo mudar no Rust, mude aqui também.

export type BlockMode = 'personal' | 'parental';
export type Platform = 'windows' | 'android';
export type BlockedType = 'domain' | 'app' | 'keyword';
export type LinkStatus = 'pending' | 'active' | 'revoked';

export interface User {
    id: string;
    firebase_uid: string;
    email: string;
    display_name: string;
    mode: BlockMode;
    created_at: string;
}

export interface Device {
    id: string;
    user_id: string;
    device_name: string;
    platform: Platform;
    is_child: boolean;
    created_at: string;
}

export interface BlockedItem {
    id: string;
    user_id: string;
    item_type: BlockedType;
    value: string;
    is_active: boolean;
    created_at: string;
}

export interface AdultFilterSettings {
    id: string;
    user_id: string;
    is_enabled: boolean;
    last_list_update: string | null;
}

export interface BlockingStatus {
    enabled: boolean;
    adult_filter_enabled: boolean;
    /** True quando o filtro adulto está ligado mas o Bloom ainda está sendo baixado/populado. */
    adult_filter_building: boolean;
    item_count: number;
}

// Request DTOs — espelham backend/src/models.rs.
export interface RegisterRequest {
    email: string;
    display_name: string;
    mode: BlockMode;
    email_verification_token?: string;
}

export interface EmailCodeStartRequest {
    email: string;
}

export interface EmailCodeStartResponse {
    expires_at: string;
    resend_after_seconds: number;
}

export interface EmailCodeVerifyRequest {
    email: string;
    code: string;
}

export interface EmailCodeVerifyResponse {
    email_verification_token: string;
}

export interface CreateBlockedItemRequest {
    item_type: BlockedType;
    value: string;
}

// ---- devices / parental ----

export interface RegisterDeviceRequest {
    device_name: string;
    platform: Platform;
}

export interface GenerateLinkCodeResponse {
    code: string;
    expires_at: string;
}

export interface ConfirmLinkRequest {
    code: string;
    device_name: string;
    platform: Platform;
}

export interface ConfirmLinkResponse {
    /** Formato "dt_<plain>". O cliente do filho guarda em SQLCipher local e
     * envia em todo header Authorization. */
    device_token: string;
    device_id: string;
    user_id: string;
    parent_device_id: string;
}

export interface SuccessResponse {
    message: string;
}

/** Espelha a struct ChildSession persistida em SQLCipher (db.rs). */
export interface ChildSession {
    user_id: string;
    device_id: string;
    device_token: string;
    parent_device_id: string;
}
