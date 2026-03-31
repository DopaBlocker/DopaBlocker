// Orquestrador de bloqueio — coordena WFP e DNS proxy.
// Implementar: struct BlockingEngine com métodos start(), stop(), is_active(),
// update_rules(blocklist). Quando ativado, inicia o WFP filter e o DNS proxy.
// Quando desativado, remove os filtros e restaura o DNS original.
