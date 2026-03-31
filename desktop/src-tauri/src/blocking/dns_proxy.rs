// DNS proxy local para bloqueio de domínios.
// Implementar: mini servidor DNS (UDP porta 53 ou porta alta) que intercepta
// queries DNS. Se o domínio está na blocklist, retorna 0.0.0.0 (NXDOMAIN).
// Caso contrário, encaminha para o DNS upstream (ex: 8.8.8.8).
// Configurar o sistema para usar este DNS proxy como resolver primário.
