import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../theme.dart';
import '../widgets/ui_kit.dart';

/// Tela do fluxo Filhos — apenas input de 6 dígitos, sem login/cadastro.
/// Espelha desktop/src/routes/onboarding/child/+page.svelte.
class ChildCodeScreen extends ConsumerStatefulWidget {
  const ChildCodeScreen({super.key});

  @override
  ConsumerState<ChildCodeScreen> createState() => _ChildCodeScreenState();
}

class _ChildCodeScreenState extends ConsumerState<ChildCodeScreen> {
  final List<TextEditingController> _ctrls =
      List.generate(6, (_) => TextEditingController());
  final List<FocusNode> _nodes = List.generate(6, (_) => FocusNode());
  bool _loading = false;
  String? _error;

  @override
  void dispose() {
    for (final c in _ctrls) c.dispose();
    for (final n in _nodes) n.dispose();
    super.dispose();
  }

  String get _code => _ctrls.map((c) => c.text).join();

  Future<void> _confirm() async {
    final code = _code;
    if (code.length < 6) {
      setState(() => _error = 'Digite os 6 dígitos completos.');
      return;
    }
    setState(() { _loading = true; _error = null; });
    try {
      await ref.read(authProvider.notifier).confirmChildCode(
            code,
            'Android do filho',
          );
    } catch (_) {
      setState(() => _error = 'Código inválido ou expirado. Peça um novo para o responsável.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Código de vinculação')),
      body: Padding(
        padding: const EdgeInsets.all(AppSpacing.x6),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: AppSpacing.x6),
            Text(
              'Digite o código de 6 dígitos gerado pelo responsável.',
              textAlign: TextAlign.center,
              style: AppType.body.copyWith(color: AppColors.textSecondary),
            ),
            const SizedBox(height: AppSpacing.x8),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: List.generate(6, (i) => _buildDigitField(i)),
            ),
            if (_error != null) ...[
              const SizedBox(height: AppSpacing.x4),
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const Icon(Icons.error_outline, size: 14, color: AppColors.danger),
                  const SizedBox(width: AppSpacing.x1),
                  Flexible(
                    child: Text(
                      _error!,
                      textAlign: TextAlign.center,
                      style: AppType.caption.copyWith(color: AppColors.danger),
                    ),
                  ),
                ],
              ),
            ],
            const SizedBox(height: AppSpacing.x8),
            AppButton(
              label: 'Confirmar',
              loading: _loading,
              onPressed: _loading ? null : _confirm,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDigitField(int index) {
    return Container(
      width: 46,
      height: 56,
      margin: const EdgeInsets.symmetric(horizontal: 4),
      child: TextField(
        controller: _ctrls[index],
        focusNode: _nodes[index],
        textAlign: TextAlign.center,
        keyboardType: TextInputType.number,
        maxLength: 1,
        inputFormatters: [FilteringTextInputFormatter.digitsOnly],
        style: AppType.mono(size: 22, weight: FontWeight.w600),
        decoration: InputDecoration(
          counterText: '',
          filled: true,
          fillColor: AppColors.surfaceInput,
          contentPadding: EdgeInsets.zero,
          enabledBorder: OutlineInputBorder(
            borderRadius: AppRadii.controlR,
            borderSide: const BorderSide(color: AppColors.border),
          ),
          focusedBorder: OutlineInputBorder(
            borderRadius: AppRadii.controlR,
            borderSide: const BorderSide(color: AppColors.primary, width: 1.6),
          ),
        ),
        onChanged: (v) {
          if (v.isNotEmpty && index < 5) {
            _nodes[index + 1].requestFocus();
          } else if (v.isEmpty && index > 0) {
            _nodes[index - 1].requestFocus();
          }
          setState(() => _error = null);
        },
      ),
    );
  }
}
