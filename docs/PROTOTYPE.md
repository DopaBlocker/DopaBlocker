# DopaBlocker - Escopo do Prototipo (v0.1)

## Incluido no Prototipo

### Sistema de Contas
- Registro com email/senha
- Login com Google OAuth
- Login com email/senha
- Firebase Authentication

### Bloqueio de Sites/Apps
- Adicionar/remover sites na blocklist
- Adicionar/remover apps na blocklist (Android)
- Botao de bloquear/desbloquear (imediato no modo pessoal)
- Sincronizacao cross-device via Firestore

### Filtro de Conteudo Adulto
- Lista de dominios open-source (Steven Black / OISD)
- Bloom filter para lookup eficiente
- Toggle on/off

### Modo Pessoal
- Bloqueio no dispositivo local
- Sincronizacao de regras entre desktop e mobile (mesma conta)
- Desbloqueio imediato ao clicar

### Modo Controle Parental
- Selecionar se dispositivo e pai ou filho
- Vincular dispositivos via codigo 6 digitos
- Pai gerencia blocklist dos filhos
- Sem necessidade de 2 contas ou 2 apps

### Plataformas
- Windows (desktop via Tauri)
- Android (mobile via Flutter)

## Fora do Prototipo (Futuro)

- macOS / iOS
- Linux
- Sistema de tarefas/checklist para desbloquear
- Horarios programados de bloqueio
- Relatorios de uso
- Notificacoes push
