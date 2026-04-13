// Bloom Filter para checagem eficiente de domínios adultos.
// Implementar: struct BloomFilter com métodos new(expected_items, false_positive_rate),
// insert(domain), contains(domain). Carregar listas open-source (Steven Black / OISD)
// e popular o filtro na inicialização do app.

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
        let mut all_positions = true;
        for pos in &positions {
            if !self.bit_array[*pos] {
                all_positions = false;
            }
        } 
        all_positions
            
    }
}

