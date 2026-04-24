// =============================================================================
// CA local do DopaBlocker — raiz auto-assinada + assinatura de leafs por SNI.
// =============================================================================
// Pra servir a página de bloqueio em HTTPS, o browser precisa confiar em
// algum certificado. A técnica padrão (mesma de Fiddler, Charles, Little
// Snitch, Pi-hole-with-block-page) é gerar uma CA local, instalar como raiz
// confiável no sistema, e emitir certs leaf por hostname sob demanda.
//
// Persistência:
//   - `dopablocker_ca.der`  — cert raiz em DER (bytes crus)
//   - `dopablocker_ca.key`  — chave PKCS#8 em PEM
//   Ambos no app_data_dir, junto do SQLCipher.
//
// Install:
//   - Windows: `certutil -addstore Root <path.cer>` (requer admin — mas o
//     engine já roda elevado pro WFP). Idempotente: checa se o thumbprint já
//     existe via `certutil -store Root <thumbprint>` antes.
//
// Sign:
//   - Recarrega params + key do DER/PEM a cada sign, reconstrói a Certificate
//     do rcgen (o cert "reconstruído" difere nos bytes da raiz instalada —
//     timestamps novos — mas subject DN é idêntico, então o browser valida
//     a cadeia ligando pelo DN até a raiz no store). Comportamento padrão
//     de CAs custom.
// =============================================================================

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::sign::CertifiedKey;
use sha1::{Digest, Sha1};
use time::{Duration, OffsetDateTime};

const CA_CERT_FILE: &str = "dopablocker_ca.der";
const CA_KEY_FILE: &str = "dopablocker_ca.key";
const CA_COMMON_NAME: &str = "DopaBlocker Local CA";
const CA_ORG: &str = "DopaBlocker";
const CA_VALIDITY_YEARS: i64 = 10;
const LEAF_VALIDITY_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStatus {
    Installed,
    AlreadyPresent,
    Failed,
}

pub struct LocalCa {
    cert_der: Vec<u8>,
    key_pem: String,
    thumbprint_hex: String,
}

impl LocalCa {
    /// Carrega a CA do disco ou gera uma nova e persiste.
    pub fn load_or_create(app_data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(app_data_dir).ok();
        let cert_path = app_data_dir.join(CA_CERT_FILE);
        let key_path = app_data_dir.join(CA_KEY_FILE);

        if cert_path.exists() && key_path.exists() {
            let cert_der = std::fs::read(&cert_path).context("ler ca.der")?;
            let key_pem = std::fs::read_to_string(&key_path).context("ler ca.key")?;
            let thumbprint_hex = thumbprint(&cert_der);
            tracing::info!(thumbprint = %thumbprint_hex, "CA local carregada do disco");
            return Ok(Self {
                cert_der,
                key_pem,
                thumbprint_hex,
            });
        }

        let this = Self::generate()?;
        std::fs::write(&cert_path, &this.cert_der).context("escrever ca.der")?;
        std::fs::write(&key_path, &this.key_pem).context("escrever ca.key")?;
        tracing::info!(thumbprint = %this.thumbprint_hex, "CA local gerada");
        Ok(this)
    }

    fn generate() -> Result<Self> {
        let mut params = CertificateParams::new(Vec::<String>::new())
            .context("criar CertificateParams vazio")?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, CA_COMMON_NAME);
        dn.push(DnType::OrganizationName, CA_ORG);
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::hours(1);
        params.not_after = now + Duration::days(365 * CA_VALIDITY_YEARS);

        let key_pair = KeyPair::generate().context("gerar KeyPair da CA")?;
        let cert = params.self_signed(&key_pair).context("self-sign da CA")?;
        let cert_der = cert.der().to_vec();
        let key_pem = key_pair.serialize_pem();
        let thumbprint_hex = thumbprint(&cert_der);

        Ok(Self {
            cert_der,
            key_pem,
            thumbprint_hex,
        })
    }

    pub fn thumbprint(&self) -> &str {
        &self.thumbprint_hex
    }

    #[cfg(test)]
    pub fn cert_der(&self) -> &[u8] {
        &self.cert_der
    }

    /// Assina um leaf para `hostname` e devolve um `CertifiedKey` pronto pro
    /// rustls. Custo: ~alguns ms por chamada (geração de keypair novo +
    /// assinatura). O chamador deve cachear por SNI.
    pub fn sign_leaf(&self, hostname: &str) -> Result<CertifiedKey> {
        let ca_key = KeyPair::from_pem(&self.key_pem).context("parse ca.key PEM")?;
        let ca_der = CertificateDer::from(self.cert_der.clone());
        let ca_params = CertificateParams::from_ca_cert_der(&ca_der).context("reparse ca.der")?;
        let ca_cert = ca_params
            .self_signed(&ca_key)
            .context("reconstruir Certificate da CA")?;

        let mut leaf_params = CertificateParams::new(vec![hostname.to_string()])
            .context("criar CertificateParams do leaf")?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, hostname);
        leaf_params.distinguished_name = dn;
        leaf_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let now = OffsetDateTime::now_utc();
        leaf_params.not_before = now - Duration::hours(1);
        leaf_params.not_after = now + Duration::days(LEAF_VALIDITY_DAYS);

        let leaf_key = KeyPair::generate().context("gerar KeyPair do leaf")?;
        let leaf = leaf_params
            .signed_by(&leaf_key, &ca_cert, &ca_key)
            .context("assinar leaf")?;

        let cert_chain = vec![
            CertificateDer::from(leaf.der().to_vec()),
            CertificateDer::from(self.cert_der.clone()),
        ];
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));
        let signing_key = rustls::crypto::ring::sign::any_ecdsa_type(&key_der)
            .map_err(|e| anyhow!("any_ecdsa_type: {e}"))?;

        Ok(CertifiedKey::new(cert_chain, signing_key))
    }

    /// Instala o cert raiz no Windows `LocalMachine\Root` via certutil.
    /// Idempotente — checa o thumbprint antes.
    #[cfg(target_os = "windows")]
    pub fn install_in_windows_root(&self) -> InstallStatus {
        let tmp_dir = std::env::temp_dir();
        let cer_path = tmp_dir.join("dopablocker_ca.cer");
        if let Err(e) = std::fs::write(&cer_path, &self.cert_der) {
            tracing::warn!(error = %e, "não foi possível escrever cert temp");
            return InstallStatus::Failed;
        }

        // Probe: já está no store?
        let probe = std::process::Command::new("certutil")
            .args(["-store", "Root", &self.thumbprint_hex])
            .output();
        if let Ok(out) = probe {
            if out.status.success() {
                tracing::info!(
                    thumbprint = %self.thumbprint_hex,
                    "CA já presente no Windows Root store",
                );
                let _ = std::fs::remove_file(&cer_path);
                return InstallStatus::AlreadyPresent;
            }
        }

        // Install
        let install = std::process::Command::new("certutil")
            .args(["-addstore", "Root", cer_path.to_string_lossy().as_ref()])
            .output();
        let _ = std::fs::remove_file(&cer_path);
        match install {
            Ok(out) if out.status.success() => {
                tracing::info!(
                    thumbprint = %self.thumbprint_hex,
                    "CA instalada no Windows Root store",
                );
                InstallStatus::Installed
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                tracing::warn!(
                    code = out.status.code(),
                    stderr = %stderr.trim(),
                    stdout = %stdout.trim(),
                    "certutil -addstore Root falhou (precisa de admin?)",
                );
                InstallStatus::Failed
            }
            Err(e) => {
                tracing::warn!(error = %e, "spawn certutil falhou");
                InstallStatus::Failed
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn install_in_windows_root(&self) -> InstallStatus {
        InstallStatus::Failed
    }
}

fn thumbprint(cert_der: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(cert_der);
    hex::encode_upper(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("dopablocker_ca_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn generates_and_persists_ca() {
        let dir = tmp_dir();
        let ca = LocalCa::load_or_create(&dir).expect("gera");
        assert!(!ca.cert_der().is_empty());
        assert_eq!(ca.thumbprint().len(), 40); // SHA-1 hex = 40 chars
        assert!(dir.join(CA_CERT_FILE).exists());
        assert!(dir.join(CA_KEY_FILE).exists());
    }

    #[test]
    fn reloads_same_ca_from_disk() {
        let dir = tmp_dir();
        let a = LocalCa::load_or_create(&dir).unwrap();
        let b = LocalCa::load_or_create(&dir).unwrap();
        assert_eq!(a.thumbprint(), b.thumbprint());
        assert_eq!(a.cert_der(), b.cert_der());
    }

    #[test]
    fn signs_leaf_for_hostname() {
        let dir = tmp_dir();
        let ca = LocalCa::load_or_create(&dir).unwrap();
        let ck = ca.sign_leaf("instagram.com").expect("sign leaf");
        assert_eq!(ck.cert.len(), 2); // leaf + CA
        assert!(!ck.cert[0].as_ref().is_empty());
    }
}
