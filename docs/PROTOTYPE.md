# DopaBlocker — Escopo do Protótipo (v0.1)

## Fluxo de Onboarding (3 opções na tela inicial)

Ao abrir o app pela primeira vez (desktop ou mobile), o usuário vê **três opções** no mesmo nível:

```
┌──────────────────────────────────┐
│        Como você vai usar?       │
│                                  │
│   [ Pessoal ]                    │
│   [ Pais    ]                    │
│   [ Filhos  ]                    │
└──────────────────────────────────┘
```

### Opção 1 — Pessoal
1. Usuário clica em **Pessoal**
2. Tela de cadastro: email + senha (ou login com Google)
3. Backend cria `User` com `mode = 'personal'` e `Device` com `is_child = 0`
4. Ao final, o usuário cai na tela inicial do app e pode gerenciar a própria blocklist
5. Todos os devices da mesma conta pessoal sincronizam a blocklist entre si (ex: PC + celular do mesmo usuário)

### Opção 2 — Pais
1. Usuário clica em **Pais**
2. Tela de cadastro: email + senha (ou login com Google)
3. Backend cria `User` com `mode = 'parental'` e `Device` com `is_child = 0`
4. Na tela inicial, o pai tem acesso a:
   - Botão "Gerar código de vinculação" → gera código de 6 dígitos (TTL de 5 minutos)
   - Lista de dispositivos filhos já vinculados
   - Gerenciamento da blocklist que será aplicada nos filhos
5. **O device do pai não aplica os próprios bloqueios** (pai fica imune — ver "Regras importantes" abaixo)

### Opção 3 — Filhos
1. Usuário clica em **Filhos**
2. Tela com **input de 6 dígitos** (sem campos de cadastro/login)
3. Usuário digita o código que o pai gerou
4. O backend:
   - Valida o código (pending, não expirado)
   - Resolve o `user_id` do pai a partir do `parent_device_id`
   - Cria um novo `Device` sob a conta do pai com `is_child = 1`
   - Atualiza o `parental_link` para `status = 'active'` e preenche o `child_device_id`
   - Gera um **device token** (ver [ARCHITECTURE.md](ARCHITECTURE.md) → "Autenticação dual") e retorna para o app do filho
5. O device do filho guarda o token em secure storage e entra na tela "read-only de filho":
   - Exibe a blocklist atual (definida pelo pai)
   - Não permite editar nada
   - Aplica os bloqueios via DNS Proxy / VPN

> **Importante:** o filho **não cria conta Firebase**. Todo o controle passa pela conta do pai. Um único par `user_id + email` serve tanto para o pai quanto para todos os filhos vinculados.

---

## Regras importantes

### Uma conta, uma blocklist
Toda blocklist é armazenada em `blocked_items` vinculada ao `user_id`. No modo parental, **todos os devices filhos da mesma conta compartilham a mesma blocklist**. Não é possível ter regras diferentes para cada filho no v0.1.

### Pai fica imune aos próprios bloqueios
Quando o `User.mode = 'parental'`, o blocking engine **do device do pai** consulta o próprio `Device.is_child` antes de aplicar a blocklist:
- Se `is_child = 0` (device do pai) → **não aplica** os bloqueios
- Se `is_child = 1` (device do filho) → aplica normalmente

**Consequência:** se o pai quiser se auto-bloquear, precisa criar uma **conta separada** em modo Pessoal no próprio device. No modo Pais, a blocklist vale apenas para os filhos.

### Sincronização cross-device
Dispositivos da mesma conta (pai + filhos, ou múltiplos devices pessoais) sincronizam:
- A blocklist (via backend)
- O estado do filtro adulto (on/off)
- A lista de dispositivos vinculados

No v0.1 a sincronização usa polling a cada 30 segundos (ver [DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md) → Fase B5).

---

## Incluído no Protótipo

### Sistema de Contas
- Registro com email/senha (Pessoal e Pais)
- Login com Google OAuth (Pessoal e Pais)
- Login com email/senha (Pessoal e Pais)
- Firebase Authentication (só para Pessoal e Pais)
- **Device Token** para filhos (sem Firebase)

### Bloqueio de Sites/Apps
- Adicionar/remover sites na blocklist
- Adicionar/remover apps na blocklist (Android)
- Botão de bloquear/desbloquear (imediato no modo pessoal)
- Sincronização cross-device via backend

### Filtro de Conteúdo Adulto
- Lista de domínios open-source (Steven Black / OISD)
- Bloom filter para lookup eficiente
- Toggle on/off

### Modo Pessoal
- Bloqueio no dispositivo local
- Sincronização de regras entre desktop e mobile (mesma conta)
- Desbloqueio imediato ao clicar

### Modo Pais
- Conta Firebase do pai
- Blocklist gerenciada pelo pai e aplicada nos devices filhos
- Pai fica imune aos próprios blocks (ver "Regras importantes")
- Geração de código de vinculação de 6 dígitos com TTL de 5 minutos
- Lista de filhos vinculados

### Modo Filhos
- Nenhum cadastro/login — apenas o código de 6 dígitos
- Device token em vez de Firebase JWT
- Blocklist read-only (gerenciada pelo pai)
- Aplicação dos bloqueios via DNS Proxy (desktop) ou VPN (mobile)

### Plataformas
- Windows (desktop via Tauri)
- Android (mobile via Flutter)

---

## Fora do Protótipo (Futuro)

- macOS / iOS
- Linux
- Sistema de tarefas/checklist para desbloquear
- Horários programados de bloqueio
- Relatórios de uso
- Notificações push
- **Blocklists diferentes por filho** (hoje todos compartilham a mesma)
- **Pai se auto-bloqueando no mesmo app** (hoje requer conta Pessoal separada)
- **Rotação automática de device tokens** (hoje são válidos até o pai revogar)
- Listeners real-time do Firestore (hoje usa polling)
