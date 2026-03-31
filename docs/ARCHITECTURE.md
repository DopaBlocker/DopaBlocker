# DopaBlocker - Arquitetura

## Visao Geral

Monorepo com 4 sub-projetos: backend, desktop, mobile e shared.

## Fluxo de Dados

```
[Mobile App (Flutter)] --HTTP/JWT--> [Backend API (Axum)] --Firestore--> [Firebase Cloud]
[Desktop App (Tauri)]  --HTTP/JWT--> [Backend API (Axum)] --Firestore--> [Firebase Cloud]
```

## Tecnicas de Bloqueio

### Windows (Desktop)
- **WFP (Windows Filtering Platform)**: filtros de rede a nivel de kernel
- **DNS Proxy**: resolver local que retorna NXDOMAIN para dominios bloqueados
- **Bloom Filter**: lookup rapido de dominios adultos (Steven Black / OISD)

### Android (Mobile)
- **VPN Service**: intercepta trafego DNS via TUN interface
- **Accessibility Service**: detecta e bloqueia abertura de apps
- **Boot Receiver**: reinicia VPN automaticamente apos reboot

## Sincronizacao

- SQLite local como cache offline em cada dispositivo
- Firestore como fonte de verdade para sincronizacao cross-device
- Backend API como intermediario para validacao e logica de negocios

## Controle Parental

- Uma conta, multiplos dispositivos (pai + filhos)
- Vinculacao via codigo 6 digitos com TTL de 5 minutos
- Pai gerencia blocklist que propaga para dispositivos filhos
- Nao requer app separado nem conta separada
