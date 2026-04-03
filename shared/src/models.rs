use serde::{Deserialize, Serialize};

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