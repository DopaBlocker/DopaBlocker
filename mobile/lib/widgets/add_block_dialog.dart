import 'package:flutter/material.dart';

import '../theme.dart';

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

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  String get _hint => switch (_type) {
        'app' => 'com.instagram.android',
        'keyword' => 'apostas',
        _ => 'instagram.com',
      };

  void _confirm() {
    final value = _controller.text.trim().toLowerCase();
    if (value.isEmpty) {
      setState(() => _error = 'Informe um valor.');
      return;
    }
    Navigator.pop(context, AddBlockResult(value, _type));
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
