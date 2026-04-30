# DopaBlocker — Escopo do Protótipo (v0.2)

> **Status:** este documento descreve o **objetivo do protótipo v0.2**. A v0.2
> ainda não está pronta. Um item só deve ser marcado como entregue quando
> desktop e mobile estiverem funcionando, os modos Pessoal/Pais/Filhos
> estiverem completos, e os fluxos de contas e bloqueio estiverem sem erros
> conhecidos bloqueantes e testados no golden path.

## Objetivo da v0.2

Entregar um protótipo cross-platform realmente usável: **desktop Windows
(Tauri/Svelte)** e **mobile Android (Flutter)** devem abrir, autenticar,
sincronizar regras e aplicar bloqueios. A v0.2 fecha o ciclo que a v0.1 deixou
como direção: não basta ter backend e desktop; o protótipo precisa provar o
fluxo completo entre responsável, filho e dispositivos.

### Definição de pronto

- Desktop e mobile rodam sem erro de build/check/análise.
- Modo Pessoal funciona em desktop e mobile.
- Modo Pais funciona em desktop e mobile.
- Modo Filhos funciona em desktop e mobile.
- Sistema de contas funciona com email/senha, Google OAuth onde suportado, e
  Device Token para filhos.
- Sistema de bloqueio funciona no desktop via DNS Proxy/WFP e no mobile via
  VPN/serviços Android.
- Sync de blocklist entre conta, pai e filhos está testado.
- Fluxos principais estão cobertos por testes automatizados e smoke tests
  manuais documentados.

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
Toda blocklist é armazenada em `blocked_items` vinculada ao `user_id`. No modo
parental, **todos os devices filhos da mesma conta compartilham a mesma
blocklist**. Não é necessário ter regras diferentes para cada filho na v0.2,
salvo se isso virar requisito explícito antes do fechamento.

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

Na v0.2, a sincronização precisa estar funcionando e testada entre desktop e
mobile. O mecanismo pode continuar sendo polling curto via backend se o golden
path ficar estável; listeners real-time ficam fora do escopo obrigatório até
virarem necessidade comprovada.

---

## Escopo-alvo do Protótipo v0.2

### Sistema de Contas
- Registro com email/senha (Pessoal e Pais)
- Login com Google OAuth (Pessoal e Pais)
- Login com email/senha (Pessoal e Pais)
- Firebase Authentication (só para Pessoal e Pais)
- **Device Token** para filhos (sem Firebase)
- Persistência segura da sessão no desktop e no mobile
- Logout, revogação de filho e exclusão de conta sem deixar sessão local órfã

### Bloqueio de Sites/Apps
- Adicionar/remover sites na blocklist
- Adicionar/remover apps na blocklist (Android)
- Botão de bloquear/desbloquear (imediato no modo pessoal)
- Sincronização cross-device via backend
- Desktop: bloqueio DNS por proxy local, proteção anti-bypass por WFP e página
  local de bloqueio
- Mobile: bloqueio por VPN local no Android e integração nativa suficiente para
  aplicar a blocklist ativa

### Filtro de Conteúdo Adulto
- Lista de domínios open-source (Steven Black / OISD)
- Bloom filter para lookup eficiente
- Toggle on/off

### Modo Pessoal
- Bloqueio no dispositivo local em desktop e mobile
- Sincronização de regras entre desktop e mobile na mesma conta
- Desbloqueio imediato ao clicar
- Reabertura do app preserva sessão, lista e estado do bloqueio

### Modo Pais
- Conta Firebase do pai
- Blocklist gerenciada pelo pai e aplicada nos devices filhos
- Pai fica imune aos próprios blocks (ver "Regras importantes")
- Geração de código de vinculação de 6 dígitos com TTL de 5 minutos
- Lista de filhos vinculados
- Revogação de filhos pela UI
- Alterações feitas pelo pai chegam aos filhos no desktop e no mobile

### Modo Filhos
- Nenhum cadastro/login — apenas o código de 6 dígitos
- Device token em vez de Firebase JWT
- Blocklist read-only (gerenciada pelo pai)
- Aplicação dos bloqueios via DNS Proxy (desktop) ou VPN (mobile)
- Sessão de filho persistida com segurança e validada no boot
- Filho revogado pelo pai perde acesso no próximo ciclo de validação/sync

### Plataformas
- Windows (desktop via Tauri)
- Android (mobile via Flutter)
- O protótipo v0.2 só fecha quando as duas plataformas estiverem verificadas
  no golden path.

---

## Critérios de Aceite e Testes

### Checks automatizados mínimos
- `cargo test` na raiz do monorepo
- `pnpm --dir desktop check`
- `flutter analyze` em `mobile/`
- `flutter test` em `mobile/` quando houver widgets/providers testáveis

### Smoke tests obrigatórios
- Criar conta Pessoal, fazer login, adicionar item, ativar bloqueio, confirmar
  bloqueio, pausar e confirmar desbloqueio.
- Criar conta Pais, gerar código, vincular um dispositivo Filho e confirmar que
  o filho entra sem Firebase.
- No modo Pais, adicionar/remover item e confirmar que o filho recebe a regra.
- Confirmar que o device do pai em modo parental continua imune aos próprios
  bloqueios.
- Revogar um filho e confirmar que o Device Token deixa de funcionar.
- Reabrir desktop e mobile e confirmar restauração correta de sessão,
  blocklist e estado do bloqueio.
- Rodar o fluxo em desktop Windows e Android mobile antes de considerar a v0.2
  concluída.

---

## Fora do Protótipo v0.2 (Futuro)

- macOS / iOS
- Linux
- Sistema de tarefas/checklist para desbloquear
- Horários programados de bloqueio
- Relatórios de uso
- Notificações push
- **Blocklists diferentes por filho** (v0.2 mantém uma blocklist por conta,
  aplicada a todos os filhos)
- **Pai se auto-bloqueando no mesmo app** (hoje requer conta Pessoal separada)
- **Rotação automática de device tokens** (hoje são válidos até o pai revogar)
- Sincronização real-time dedicada, caso o polling via backend seja suficiente
  para o protótipo
