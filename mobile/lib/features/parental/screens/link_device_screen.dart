import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/api_dtos.dart';
import 'package:dopablocker_mobile/core/api/devices_api.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Vinculação de dispositivos: exibe código de 6 dígitos com TTL de 5 min.
/// Fase 2: integrar com GET /devices/link-code e exibir countdown + QR code.
class LinkDeviceScreen extends ConsumerStatefulWidget {
  const LinkDeviceScreen({super.key});

  @override
  ConsumerState<LinkDeviceScreen> createState() => _LinkDeviceScreenState();
}

class _LinkDeviceScreenState extends ConsumerState<LinkDeviceScreen> {
  GenerateLinkCodeResponse? _linkCode;
  bool _loading = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _generate();
  }

  Future<void> _generate() async {
    setState(() { _loading = true; _error = null; });
    try {
      final code = await ref.read(devicesApiProvider).generateLinkCode();
      if (mounted) setState(() => _linkCode = code);
    } catch (e) {
      if (mounted) setState(() => _error = 'Não foi possível gerar o código.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _copyCode() {
    final code = _linkCode;
    if (code == null) return;
    Clipboard.setData(ClipboardData(text: code.code));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Código copiado!')),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Vincular dispositivo')),
      body: Padding(
        padding: const EdgeInsets.all(AppSpacing.x6),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: AppSpacing.x4),
            Text(
              'Compartilhe o código abaixo com o dispositivo do filho.',
              textAlign: TextAlign.center,
              style: AppType.body.copyWith(color: AppColors.textSecondary),
            ),
            const SizedBox(height: AppSpacing.x8),
            if (_loading)
              const Padding(
                padding: EdgeInsets.symmetric(vertical: AppSpacing.x8),
                child: Center(
                  child: CircularProgressIndicator(color: AppColors.primary),
                ),
              )
            else if (_error != null)
              Text(
                _error!,
                textAlign: TextAlign.center,
                style: AppType.body.copyWith(color: AppColors.danger),
              )
            else if (_linkCode != null) ...[
              AppCard(
                onTap: _copyCode,
                highlight: true,
                padding: const EdgeInsets.symmetric(
                  vertical: AppSpacing.x6,
                  horizontal: AppSpacing.x4,
                ),
                child: Text(
                  _linkCode!.code,
                  textAlign: TextAlign.center,
                  style: AppType.mono(
                    size: 40,
                    weight: FontWeight.w700,
                    color: AppColors.primary,
                    letterSpacing: 12,
                  ),
                ),
              ),
              const SizedBox(height: AppSpacing.x3),
              Text(
                'Toque para copiar • Válido por 5 minutos',
                textAlign: TextAlign.center,
                style: AppType.caption,
              ),
            ],
            const SizedBox(height: AppSpacing.x8),
            AppButton(
              label: 'Gerar novo código',
              variant: AppButtonVariant.secondary,
              icon: Icons.refresh,
              onPressed: _loading ? null : _generate,
            ),
          ],
        ),
      ),
    );
  }
}
