import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Diálogos de confirmação da aba Conta (logout, troca de modo, exclusão e
/// reautenticação). São funções puras de UI: devolvem a escolha do usuário
/// (`true`/`false`/`null`) e quem chama (SettingsScreen) orquestra os efeitos.

/// Confirmação de logout.
Future<bool?> confirmLogoutDialog(BuildContext context) {
  return showDialog<bool>(
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
}

/// Confirmação ao sair do modo Pais (avisa se há filhos vinculados — os
/// vínculos continuam).
Future<bool?> confirmModeDialog(BuildContext context, int childCount) {
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

/// Confirmação forte de exclusão de conta: o botão só habilita quando o usuário
/// digita "EXCLUIR".
Future<bool?> confirmDeleteDialog(BuildContext context) async {
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
Future<bool?> reauthDialog(BuildContext context) {
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
