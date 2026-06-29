// Política de bloqueio: decide se e por que um domínio é bloqueado
// (`block_reason`) e mantém o filtro de conteúdo adulto (`adult_filter`,
// Bloom filter). Compartilhada entre o DNS proxy e a página de bloqueio.

pub mod adult_filter;
pub mod block_reason;
