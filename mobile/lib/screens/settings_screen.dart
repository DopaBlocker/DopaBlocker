import 'package:firebase_auth/firebase_auth.dart' as fb;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../channels/blocking_channel.dart';
import '../core/api_client.dart';
import '../providers/auth_provider.dart';
import '../providers/permissions_provider.dart';
import '../theme.dart';
import '../widgets/ui_kit.dart';

/// Aba "Conta" — dados da conta, permissões reais de bloqueio e logout.
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  Future<void> _confirmLogout(BuildContext context, WidgetRef ref) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: AppColors.surface,
        title: const Text('Sair da conta?'),
        content: const Text('Você precisará entrar novamente para gerenciar bloqueios.'),
        actions: [
          AppButton(
            label: 'Cancelar',
            variant: AppButtonVariant.ghost,
            fullWidth: false,
            onPressed: () => Navigator.pop(context, false),
          ),
          AppButton(
            label: 'Sair',
            variant: AppButtonVariant.danger,
            fullWidth: false,
            onPressed: () => Navigator.pop(context, true),
          ),
        ],
      ),
    );
    if (ok == true) await ref.read(authProvider.notifier).logout();
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = ref.watch(authProvider);
    final perms = ref.watch(protectionPermissionsProvider);
    final user = auth is AuthAuthenticated ? auth.user : null;
    final isParental = user?.isParental ?? false;

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 20,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: const [
            Text('CONTA', style: AppType.label),
            Text('Sua conta', style: AppType.h1),
          ],
        ),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
        children: [
          // Conta
          AppCard(
            padding: const EdgeInsets.all(14),
            child: Row(
              children: [
                InitialBadge(user?.displayName ?? 'D', size: 48),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(user?.displayName ?? 'Convidado', style: AppType.title),
                      const SizedBox(height: 2),
                      Text(user?.email ?? 'sem conta vinculada',
                          maxLines: 1, overflow: TextOverflow.ellipsis,
                          style: AppType.bodySm.copyWith(color: AppColors.textFaint)),
                    ],
                  ),
                ),
                isParental ? AppChip.accent('Pais') : const AppChip('Pessoal'),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Modo de uso — troca personal↔parental sem recriar a conta.
          const SectionLabel('Modo de uso'),
          AppCard(
            padding: const EdgeInsets.all(14),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  isParental
                      ? 'Você gerencia os bloqueios dos dispositivos dos filhos.'
                      : 'Os bloqueios valem para este dispositivo.',
                  style: AppType.bodySm.copyWith(color: AppColors.textSecondary),
                ),
                const SizedBox(height: 12),
                AppButton(
                  label: isParental ? 'Mudar para Pessoal' : 'Mudar para Pais',
                  variant: AppButtonVariant.secondary,
                  fullWidth: false,
                  onPressed: user == null ? null : () => _switchMode(context, ref),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Bloqueio de apps — permissões do sistema necessárias
          const SectionLabel('Bloqueio de apps'),
          AppCard(
            padding: EdgeInsets.zero,
            child: Column(
              children: [
                _PermissionStatusTile(
                  title: 'Serviço de acessibilidade',
                  subtitle: 'Detecta e bloqueia os apps da sua lista',
                  granted: perms.accessibilityEnabled,
                  onTap: BlockingChannel.openAccessibilitySettings,
                ),
                const Divider(color: AppColors.divider, height: 1),
                _PermissionStatusTile(
                  title: 'Sobrepor a outros apps',
                  subtitle: 'Mostra a tela de bloqueio por cima do app/site',
                  granted: perms.canDrawOverlays,
                  onTap: BlockingChannel.requestOverlayPermission,
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Logout
          AppCard(
            onTap: () => _confirmLogout(context, ref),
            child: Row(
              children: [
                const Icon(Icons.logout, color: AppColors.danger, size: 20),
                const SizedBox(width: 12),
                Text('Sair',
                    style: AppType.body.copyWith(color: AppColors.danger, fontWeight: FontWeight.w600)),
              ],
            ),
          ),
          const SizedBox(height: 16),
          // Zona de perigo — exclusão de conta (irreversível).
          AppCard(
            border: Border.all(color: AppColors.danger.withValues(alpha: 0.3)),
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Excluir conta permanentemente',
                    style: AppType.body.copyWith(
                        color: AppColors.danger, fontWeight: FontWeight.w600)),
                const SizedBox(height: 6),
                Text(
                  'Apaga sua conta, todos os bloqueios, os filhos vinculados (se houver) '
                  'e o login no Firebase. Não dá para desfazer.',
                  style: AppType.bodySm.copyWith(color: AppColors.textSecondary),
                ),
                const SizedBox(height: 12),
                AppButton(
                  label: 'Excluir conta',
                  icon: Icons.delete_outline,
                  variant: AppButtonVariant.danger,
                  fullWidth: false,
                  onPressed: () => _deleteAccount(context, ref),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          const Center(
            child: Text('DopaBlocker · v0.2', style: AppType.caption),
          ),
        ],
      ),
    );
  }

  /// Troca o modo (personal↔parental) sem recriar a conta. Ao sair do parental,
  /// confirma antes (avisando se há filhos vinculados — os vínculos continuam).
  Future<void> _switchMode(BuildContext context, WidgetRef ref) async {
    final auth = ref.read(authProvider);
    if (auth is! AuthAuthenticated) return;
    final isParental = auth.user.isParental;
    final target = isParental ? 'personal' : 'parental';
    final messenger = ScaffoldMessenger.of(context);

    if (isParental) {
      int childCount = 0;
      try {
        final devices = await ref.read(apiClientProvider).getDevices();
        childCount = devices.where((d) => d.isChild).length;
      } catch (_) {/* offline: confirma sem a contagem */}
      if (!context.mounted) return;
      final ok = await _confirmModeDialog(context, childCount);
      if (ok != true) return;
    }

    try {
      final updated = await ref.read(authProvider.notifier).updateMode(target);
      messenger.showSnackBar(SnackBar(
        content: Text(updated.isParental
            ? 'Modo alterado para Pais.'
            : 'Modo alterado para Pessoal.'),
      ));
    } catch (_) {
      messenger.showSnackBar(
        const SnackBar(content: Text('Não foi possível trocar o modo.')),
      );
    }
  }

  Future<bool?> _confirmModeDialog(BuildContext context, int childCount) {
    return showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        backgroundColor: AppColors.surface,
        title: const Text('Sair do modo Pais?'),
        content: Text(
          childCount > 0
              ? 'Você tem $childCount dispositivo(s) de filho vinculado(s). No modo '
                  'Pessoal você deixa de gerenciar os bloqueios deles (os vínculos '
                  'continuam). Dá para voltar para Pais quando quiser.'
              : 'No modo Pessoal os bloqueios passam a valer para você. Dá para '
                  'voltar para Pais quando quiser.',
          style: AppType.bodySm.copyWith(color: AppColors.textSecondary),
        ),
        actions: [
          AppButton(
            label: 'Cancelar',
            variant: AppButtonVariant.ghost,
            fullWidth: false,
            onPressed: () => Navigator.pop(dialogContext, false),
          ),
          AppButton(
            label: 'Mudar para Pessoal',
            fullWidth: false,
            onPressed: () => Navigator.pop(dialogContext, true),
          ),
        ],
      ),
    );
  }

  /// Fluxo de exclusão de conta (paridade com o desktop): confirma digitando
  /// "EXCLUIR" → `AuthNotifier.deleteAccount()` (Firebase + backend). Em sessão
  /// antiga (`requires-recent-login`), oferece relogin.
  Future<void> _deleteAccount(BuildContext context, WidgetRef ref) async {
    final confirmed = await _confirmDeleteDialog(context);
    if (confirmed != true || !context.mounted) return;
    final messenger = ScaffoldMessenger.of(context);
    try {
      await ref.read(authProvider.notifier).deleteAccount();
      // app.dart leva à Welcome automaticamente (estado signed_out).
      messenger.showSnackBar(const SnackBar(content: Text('Conta excluída.')));
    } on fb.FirebaseAuthException catch (e) {
      if (e.code == 'requires-recent-login') {
        if (!context.mounted) return;
        final relogin = await _reauthDialog(context);
        if (relogin == true) await ref.read(authProvider.notifier).logout();
      } else {
        messenger.showSnackBar(
          SnackBar(content: Text('Erro ao excluir: ${e.message ?? e.code}')),
        );
      }
    } catch (_) {
      messenger.showSnackBar(
        const SnackBar(content: Text('Não foi possível excluir a conta.')),
      );
    }
  }

  /// Confirmação forte: o botão só habilita quando o usuário digita "EXCLUIR".
  Future<bool?> _confirmDeleteDialog(BuildContext context) async {
    final controller = TextEditingController();
    try {
      return await showDialog<bool>(
        context: context,
        builder: (dialogContext) => StatefulBuilder(
          builder: (context, setState) {
            final canDelete = controller.text.trim().toUpperCase() == 'EXCLUIR';
            return AlertDialog(
              backgroundColor: AppColors.surface,
              title: const Text('Excluir conta?'),
              content: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Esta ação é irreversível. Digite EXCLUIR para confirmar.',
                    style: AppType.bodySm.copyWith(color: AppColors.textSecondary),
                  ),
                  const SizedBox(height: 14),
                  TextField(
                    controller: controller,
                    autofocus: true,
                    textCapitalization: TextCapitalization.characters,
                    onChanged: (_) => setState(() {}),
                    decoration: const InputDecoration(hintText: 'EXCLUIR'),
                  ),
                ],
              ),
              actions: [
                AppButton(
                  label: 'Cancelar',
                  variant: AppButtonVariant.ghost,
                  fullWidth: false,
                  onPressed: () => Navigator.pop(dialogContext, false),
                ),
                AppButton(
                  label: 'Excluir conta',
                  variant: AppButtonVariant.danger,
                  fullWidth: false,
                  onPressed: canDelete ? () => Navigator.pop(dialogContext, true) : null,
                ),
              ],
            );
          },
        ),
      );
    } finally {
      controller.dispose();
    }
  }

  /// Diálogo de reautenticação quando o Firebase exige login recente.
  Future<bool?> _reauthDialog(BuildContext context) {
    return showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        backgroundColor: AppColors.surface,
        title: const Text('Sessão antiga'),
        content: Text(
          'Por segurança, faça login de novo para excluir a conta.',
          style: AppType.bodySm.copyWith(color: AppColors.textSecondary),
        ),
        actions: [
          AppButton(
            label: 'Cancelar',
            variant: AppButtonVariant.ghost,
            fullWidth: false,
            onPressed: () => Navigator.pop(dialogContext, false),
          ),
          AppButton(
            label: 'Fazer login de novo',
            fullWidth: false,
            onPressed: () => Navigator.pop(dialogContext, true),
          ),
        ],
      ),
    );
  }
}

/// Tile de status de uma permissão do sistema (acessibilidade / overlay):
/// mostra um chip "ativo"/"ativar" e, ao tocar, abre a tela do sistema.
class _PermissionStatusTile extends StatelessWidget {
  final String title;
  final String subtitle;
  final bool granted;
  final VoidCallback onTap;

  const _PermissionStatusTile({
    required this.title,
    required this.subtitle,
    required this.granted,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: granted ? null : onTap,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: AppType.body.copyWith(fontWeight: FontWeight.w600)),
                  const SizedBox(height: 2),
                  Text(subtitle, style: AppType.caption.copyWith(color: AppColors.textFaint)),
                ],
              ),
            ),
            const SizedBox(width: 12),
            granted ? AppChip.success('ativo') : AppChip.warning('ativar'),
          ],
        ),
      ),
    );
  }
}
