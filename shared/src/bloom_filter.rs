// Bloom Filter para checagem eficiente de domínios adultos.
// Implementar: struct BloomFilter com métodos new(expected_items, false_positive_rate),
// insert(domain), contains(domain). Carregar listas open-source (Steven Black / OISD)
// e popular o filtro na inicialização do app.

use serde::{Deserialize, Serialize};

/// `Clone` habilita espalhar via `Arc` em leitores concorrentes; `Serialize`/
/// `Deserialize` permite persistir o filtro já populado em disco (bincode)
/// pra pular a reconstrução em boots seguintes.
#[derive(Clone, Serialize, Deserialize)]
pub struct BloomFilter {
    bit_array: Vec<bool>,
    num_hashes: usize,
}

fn fnv1a_hash(data: &str) -> u64 {
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME:  u64 = 1099511628211;

    let mut hash = FNV_OFFSET;

    for byte in data.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME)
    }
    hash
}

fn calculate_size(expected_items: usize, false_positive_rate: f64) -> usize {
    let n_f = expected_items as f64;
    let ln2 = 2.0_f64.ln();
    let bits_size = -(n_f * false_positive_rate.ln()) / (ln2 * ln2);

    bits_size.ceil() as usize
}

fn calculate_num_hashes(bits_size: usize, expected_items: usize) -> usize {
    let b_s = bits_size as f64;
    let e_i = expected_items as f64;
    let ln2 = 2.0_f64.ln();

    let hashes_number = (b_s / e_i) * ln2;

    hashes_number.ceil() as usize
}

fn get_positions(item: &str, num_hashes: usize, array_size: usize) -> Vec<usize> {
    let hash1 = fnv1a_hash(item);
    let hash2 = fnv1a_hash(&format!("seed2_{}", item));

    let mut positions = Vec::with_capacity(num_hashes);
    for i in 0..num_hashes {
        let position = hash1.wrapping_add((i as u64).wrapping_mul(hash2)) % array_size as u64;
        positions.push(position as usize);
    }
    positions
}

impl BloomFilter {
    pub fn new (expected_items: usize, false_positive_rate: f64) -> Self {
        let size = calculate_size(expected_items, false_positive_rate);
        let num_hashes = calculate_num_hashes(size, expected_items);
        let bit_array: Vec<bool> = vec![false; size];
        BloomFilter { bit_array, num_hashes }
    }
    
    pub fn insert(&mut self, item: &str) {
        let positions = get_positions(item, self.num_hashes, self.bit_array.len());
        for pos in &positions {
            self.bit_array[*pos] = true;
        }
    }

    pub fn contains(&self, item: &str) -> bool {
        let positions = get_positions(item, self.num_hashes, self.bit_array.len());
        positions.iter().all(|&pos| self.bit_array[pos])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_contains_returns_true() {
        let mut bf = BloomFilter::new(1_000, 0.001);
        bf.insert("pornhub.com");
        bf.insert("xvideos.com");
        assert!(bf.contains("pornhub.com"));
        assert!(bf.contains("xvideos.com"));
    }

    #[test]
    fn contains_returns_false_for_absent_item() {
        let mut bf = BloomFilter::new(1_000, 0.001);
        bf.insert("pornhub.com");
        assert!(!bf.contains("google.com"));
        assert!(!bf.contains("youtube.com"));
    }

    #[test]
    fn empty_filter_contains_nothing() {
        let bf = BloomFilter::new(100, 0.01);
        assert!(!bf.contains("anything.com"));
    }

    #[test]
    fn false_positive_rate_stays_within_budget() {
        // Insere 1k itens e confere se ≤ 2% dos 10k "estranhos" dão falso positivo.
        // Budget generoso (2× do alvo 1%) para evitar flakes com hash FNV simples.
        let mut bf = BloomFilter::new(1_000, 0.01);
        for i in 0..1_000 {
            bf.insert(&format!("inserted-{i}.com"));
        }
        let mut fps = 0;
        for i in 0..10_000 {
            if bf.contains(&format!("stranger-{i}.net")) {
                fps += 1;
            }
        }
        assert!(fps < 200, "false positives = {fps} (>= 2% de 10k)");
    }
}

