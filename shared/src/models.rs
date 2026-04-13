use serde::{Deserialize, Serialize};

/// Modo de operação da conta. Definido no cadastro (tela inicial: Pessoal ou Pais).
///
/// **Nota:** a tela inicial do app oferece 3 opções — Pessoal, Pais e Filhos — mas
/// "Filhos" NÃO é um `BlockMode`. No fluxo Filhos, nenhuma conta nova é criada:
/// o device do filho é registrado sob o `user_id` do pai (que tem `mode=Parental`)
/// com `Device.is_child = true`. Ver `docs/PROTOTYPE.md` → "Fluxo de Onboarding".
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BlockMode {
    Personal,  // → "personal"
    Parental,  // → "parental"
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,  // → "windows"
    Android,  // → "android"
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BlockedType {
    Domain,   // → "domain"  Ex: facebook.com
    App,      // → "app"     Ex: com.instagram.android
    Keyword,  // → "keyword" Ex: "cassino"
}

/// Conta de usuário. Uma única `User` pode ter múltiplos `Device`s vinculados.
///
/// **No fluxo Filhos**, nenhuma `User` nova é criada — o device do filho
/// aponta para a `User` do pai (aquele que gerou o código de vinculação).
/// Todos os filhos de uma conta compartilham o mesmo `user_id` e, por tabela,
/// a mesma blocklist em `blocked_items`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub firebase_uid: String,
    pub email: String,
    pub display_name: String,
    pub mode: BlockMode,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Device {
    pub id: String,
    pub user_id: String,
    pub device_name: String,
    pub platform: Platform,
    pub is_child: bool,
    pub created_at: String,
}

/// Item da blocklist (domínio, app ou keyword).
///
/// **Escopo no v0.1:** a blocklist é global por `user_id`. Todos os devices da
/// mesma conta compartilham a mesma lista — incluindo, no modo parental, todos
/// os filhos vinculados.
///
/// **Regra do pai imune:** embora o registro seja global, o blocking engine do
/// device do pai (`Device.is_child = false` em `User.mode = Parental`) **não
/// aplica** os blocks. Só os devices filhos (`is_child = true`) aplicam.
/// Ver `docs/ARCHITECTURE.md` → "Técnicas de Bloqueio" → "Regra do Pai imune".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedItem {
    pub id: String,
    pub user_id: String,
    pub item_type: BlockedType,
    pub value: String,          // O domínio ou package name
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkStatus {
    Pending,  // → "pending"
    Active,   // → "active"
    Revoked,  // → "revoked"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentalLink {
    pub id: String,
    pub parent_device_id: String,
    pub child_device_id: Option<String>, // None até o filho confirmar
    pub link_code: String,               // O código de 6 dígitos
    pub status: LinkStatus,
    pub expires_at: String,              // now + 5 minutos (ISO 8601)
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdultFilterSettings {
    pub id: String,
    pub user_id: String,
    pub is_enabled: bool,
    pub last_list_update: Option<String>,
}

/// Token de acesso para devices filhos, que não têm conta Firebase.
///
/// Gerado pelo backend no momento em que o filho confirma o código de
/// vinculação (`POST /devices/link/confirm`, rota pública). O device guarda
/// o token em secure storage e o envia em todas as requisições como
/// `Authorization: Bearer dt_<plain_token>`.
///
/// **Segurança:** `token_hash` é o SHA-256 do token plain, nunca o plain em si.
/// Se o banco vazar, os tokens hasheados são inúteis para um atacante.
///
/// `revoked_at = None` = token ativo. `Some(_)` = revogado pelo pai.
/// Tabela correspondente: `device_tokens` (migration 002).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToken {
    pub token_hash: String,
    pub device_id: String,
    pub user_id: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}