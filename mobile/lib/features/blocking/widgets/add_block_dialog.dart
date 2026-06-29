import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/models/installed_app.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/features/blocking/widgets/app_picker_sheet.dart';

/// Resultado do diálogo de adicionar bloqueio.
class AddBlockResult {
  final String value;
  final String itemType; // "domain" | "app" | "keyword"
  const AddBlockResult(this.value, this.itemType);
}

/// Diálogo para adicionar um novo bloqueio (site, app ou palavra-chave).
/// Retorna [AddBlockResult] via Navigator.pop, ou null se cancelado.
class AddBlockDialog extends StatefulWidget {
  const AddBlockDialog({super.key});

  static Future<AddBlockResult?> show(BuildContext context) =>
      showDialog<AddBlockResult>(context: context, builder: (_) => const AddBlockDialog());

  @override
  State<AddBlockDialog> createState() => _AddBlockDialogState();
}

class _AddBlockDialogState extends State<AddBlockDialog> {
  final _controller = TextEditingController();
  String _type = 'domain';
  String? _error;

  /// App escolhido pelo seletor visual (apenas para `_type == 'app'`). O bloqueio
  /// de app é feito só por seleção de ícone — não há digitação de package.
  InstalledApp? _selectedApp;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  String get _hint => _type == 'keyword' ? 'apostas' : 'instagram.com';

  void _confirm() {
    // App: bloqueia pelo package do app escolhido (sem lowercase — package names
    // são case-sensitive).
    if (_type == 'app') {
      final app = _selectedApp;
      if (app == null) {
        setState(() => _error = 'Escolha um app.');
        return;
      }
      Navigator.pop(context, AddBlockResult(app.packageName, 'app'));
      return;
    }

    final value = _controller.text.trim().toLowerCase();
    if (value.isEmpty) {
      setState(() => _error = 'Informe um valor.');
      return;
    }
    Navigator.pop(context, AddBlockResult(value, _type));
  }

  /// Abre o seletor visual de apps instalados; ao escolher, guarda o app para a
  /// linha de confirmação (ícone + nome). Não há digitação manual de package.
  Future<void> _pickApp() async {
    final app = await AppPickerSheet.show(context);
    if (app != null && app.packageName.isNotEmpty) {
      setState(() {
        _selectedApp = app;
        _error = null;
      });
    }
  }

  /// Linha de confirmação do app escolhido (ícone + nome + package).
  Widget _selectedAppRow(InstalledApp app) {
    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: AppColors.surfaceInput,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: AppColors.border),
      ),
      child: Row(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(8),
            child: app.icon != null
                ? Image.memory(app.icon!, width: 36, height: 36, gaplessPlayback: true)
                : const Icon(Icons.android, size: 36, color: AppColors.textSecondary),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  app.appName.isEmpty ? app.packageName : app.appName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                Text(
                  app.packageName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 11, color: AppColors.textSecondary),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AppColors.surface,
      title: const Text('Novo bloqueio', style: TextStyle(fontWeight: FontWeight.w700)),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SegmentedButton<String>(
            segments: const [
              ButtonSegment(value: 'domain', label: Text('Site'), icon: Icon(Icons.public, size: 16)),
              ButtonSegment(value: 'app', label: Text('App'), icon: Icon(Icons.smartphone, size: 16)),
              ButtonSegment(value: 'keyword', label: Text('Tema'), icon: Icon(Icons.tag, size: 16)),
            ],
            selected: {_type},
            onSelectionChanged: (s) => setState(() {
              _type = s.first;
              _error = null;
              if (_type != 'app') _selectedApp = null;
            }),
            style: ButtonStyle(
              visualDensity: VisualDensity.compact,
              backgroundColor: WidgetStateProperty.resolveWith(
                (states) => states.contains(WidgetState.selected)
                    ? AppColors.primary
                    : AppColors.surfaceInput,
              ),
              foregroundColor: WidgetStateProperty.resolveWith(
                (states) => states.contains(WidgetState.selected)
                    ? Colors.white
                    : AppColors.textSecondary,
              ),
            ),
          ),
          const SizedBox(height: 16),
          // App: bloqueio só por seleção visual de ícone (sem digitar package).
          if (_type == 'app') ...[
            if (_selectedApp != null) _selectedAppRow(_selectedApp!),
            OutlinedButton.icon(
              onPressed: _pickApp,
              icon: Icon(_selectedApp == null ? Icons.apps : Icons.swap_horiz, size: 18),
              label: Text(_selectedApp == null ? 'Escolher app instalado' : 'Trocar app'),
              style: OutlinedButton.styleFrom(
                foregroundColor: AppColors.primary,
                side: const BorderSide(color: AppColors.border),
                minimumSize: const Size.fromHeight(44),
              ),
            ),
            if (_error != null) ...[
              const SizedBox(height: 8),
              Text(_error!, style: const TextStyle(color: AppColors.danger, fontSize: 12)),
            ],
          ] else
            TextField(
              controller: _controller,
              autofocus: true,
              onSubmitted: (_) => _confirm(),
              decoration: InputDecoration(
                hintText: _hint,
                errorText: _error,
              ),
            ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancelar', style: TextStyle(color: AppColors.textSecondary)),
        ),
        FilledButton(
          onPressed: _confirm,
          style: FilledButton.styleFrom(backgroundColor: AppColors.primary),
          child: const Text('Adicionar'),
        ),
      ],
    );
  }
}
