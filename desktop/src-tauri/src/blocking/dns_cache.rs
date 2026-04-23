// =============================================================================
// Cache de respostas DNS com TTL.
// =============================================================================
// Chave: `(nome-queried, qtype, qclass)` — a tripla que identifica a query
// no nível do wire. Qtype importa porque `A google.com` e `AAAA google.com`
// são respostas distintas.
//
// Valor: `(bytes da resposta, deadline)` onde `bytes` é exatamente o que o
// upstream devolveu (com todos os registros, flags, EDNS). Na hora de
// devolver pro cliente, só reescrevemos os 2 bytes do ID — o resto fica
// idêntico ao que veio do resolver original.
//
// Eviction: on-access. Entrada expirada é removida no `get`. Há um teto
// absoluto (`MAX_ENTRIES`) para evitar crescimento ilimitado sob ataque
// com domínios aleatórios — quando estoura, limpa tudo que já está expirado
// e depois dropa o mais antigo até caber. Simples, eficaz, sem LRU "puro".
//
// Thread-safety: RwLock — get no caminho quente pega read-lock, put pega
// write-lock. Contenção é mínima porque put só acontece em MISS.
// =============================================================================

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use hickory_proto::op::Message;
use tokio::sync::RwLock;

/// Teto maior que qualquer perfil de uso pessoal razoável. 4k entradas ≈ 10MB
/// de RAM (resposta DNS média ~2-3KB). Se chegar aqui, já é abuso — então
/// limpamos tudo que venceu e cortamos o excesso.
const MAX_ENTRIES: usize = 4096;

/// Piso mínimo do TTL cacheado. Alguns CDNs devolvem TTL=0 ou 5s, o que
/// tornaria o cache inútil. Forçamos pelo menos 10s para dar algum alívio
/// ao upstream.
const MIN_TTL: u64 = 10;

/// Teto máximo do TTL cacheado. Alguns registros carregam TTL de 24h+,
/// mas pra produto de foco isso é tempo demais — se o usuário adicionar um
/// site à blocklist, ele espera que comece a bloquear em minutos, e manter
/// cache antigo atrapalha. 5min é um bom compromisso.
const MAX_TTL: u64 = 300;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Key {
    name: String,
    qtype: u16,
    qclass: u16,
}

struct Entry {
    bytes: Vec<u8>,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct DnsCache {
    inner: Arc<RwLock<HashMap<Key, Entry>>>,
}

impl DnsCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Tenta buscar uma resposta cacheada. Se existir mas estiver expirada,
    /// é removida e retorna `None`. Em caso de hit, retorna os bytes já com
    /// o ID reescrito para o `new_id` da query atual.
    pub async fn get(&self, query: &Message, new_id: u16) -> Option<Vec<u8>> {
        let key = Self::key_from_query(query)?;

        // Fast-path: read-lock, confere validade.
        {
            let map = self.inner.read().await;
            if let Some(entry) = map.get(&key) {
                if entry.expires_at > Instant::now() {
                    return Some(with_id(&entry.bytes, new_id));
                }
            } else {
                return None;
            }
        }
        // Slow-path: o que achamos estava vencido — remove com write-lock.
        let mut map = self.inner.write().await;
        map.remove(&key);
        None
    }

    /// Armazena uma resposta. O TTL é derivado do menor TTL entre todos os
    /// registros answer+authority da resposta, grampeado em `[MIN_TTL, MAX_TTL]`.
    /// Se a mensagem não tiver nenhum registro (ex: NOERROR vazio), usa MIN_TTL.
    pub async fn put(&self, query: &Message, response: &[u8]) {
        let Some(key) = Self::key_from_query(query) else { return };

        let Ok(resp_msg) = Message::from_vec(response) else { return };
        // Só cacheamos respostas NOERROR — NXDOMAIN, SERVFAIL, etc. podem ser
        // transientes e devem ser re-perguntados. (Bloqueio nosso nunca passa
        // por aqui; synthesize-a-cada-vez.)
        if resp_msg.response_code() != hickory_proto::op::ResponseCode::NoError {
            return;
        }

        let ttl = resp_msg
            .answers()
            .iter()
            .chain(resp_msg.name_servers().iter())
            .map(|r| r.ttl() as u64)
            .min()
            .unwrap_or(MIN_TTL)
            .clamp(MIN_TTL, MAX_TTL);

        let entry = Entry {
            bytes: response.to_vec(),
            expires_at: Instant::now() + Duration::from_secs(ttl),
        };

        let mut map = self.inner.write().await;
        if map.len() >= MAX_ENTRIES {
            Self::evict(&mut map);
        }
        map.insert(key, entry);
    }

    fn evict(map: &mut HashMap<Key, Entry>) {
        let now = Instant::now();
        map.retain(|_, e| e.expires_at > now);

        // Se ainda está cheio, dropa o mais antigo até voltar pra 75% do teto.
        // Evita thrashing quando chega oscilando em torno do limite.
        let target = (MAX_ENTRIES * 3) / 4;
        while map.len() > target {
            // HashMap não tem ordem — pega a primeira entrada disponível.
            // Simples o suficiente pro caso raro de estouro.
            if let Some(k) = map.keys().next().cloned() {
                map.remove(&k);
            } else {
                break;
            }
        }
    }

    fn key_from_query(msg: &Message) -> Option<Key> {
        let q = msg.queries().first()?;
        Some(Key {
            name: q.name().to_string().trim_end_matches('.').to_lowercase(),
            qtype: q.query_type().into(),
            qclass: q.query_class().into(),
        })
    }
}

impl Default for DnsCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Reescreve os 2 primeiros bytes (big-endian) para o `new_id` sem tocar
/// no resto do payload. DNS garante que ID é bytes 0-1 do header.
fn with_id(bytes: &[u8], new_id: u16) -> Vec<u8> {
    let mut out = bytes.to_vec();
    if out.len() >= 2 {
        let id_bytes = new_id.to_be_bytes();
        out[0] = id_bytes[0];
        out[1] = id_bytes[1];
    }
    out
}
