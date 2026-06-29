// Enforcement no nível do sistema operacional (Windows): filtros WFP contra
// bypass de DNS (`wfp`) e controle/restauração do DNS do sistema via netsh
// (`system_dns`, incluindo o restore síncrono usado em panic/shutdown).

pub mod system_dns;
pub mod wfp;
