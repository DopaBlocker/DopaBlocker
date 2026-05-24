import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api_client.dart';
import '../core/api_dtos.dart';

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
      final code = await ref.read(apiClientProvider).generateLinkCode();
      if (mounted) setState(() => _linkCode = code);
    } catch (e) {
      if (mounted) setState(() => _error = 'Não foi possível gerar o código.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Vincular dispositivo')),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: 16),
            const Text(
              'Compartilhe o código abaixo com o dispositivo do filho.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 16),
            ),
            const SizedBox(height: 32),
            if (_loading)
              const Center(child: CircularProgressIndicator())
            else if (_error != null)
              Text(_error!, textAlign: TextAlign.center, style: const TextStyle(color: Colors.red))
            else if (_linkCode != null) ...[
              GestureDetector(
                onTap: () {
                  Clipboard.setData(ClipboardData(text: _linkCode!.code));
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Código copiado!')),
                  );
                },
                child: Container(
                  padding: const EdgeInsets.symmetric(vertical: 20),
                  decoration: BoxDecoration(
                    border: Border.all(color: Colors.indigo, width: 2),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    _linkCode!.code,
                    textAlign: TextAlign.center,
                    style: const TextStyle(
                      fontSize: 40,
                      fontWeight: FontWeight.bold,
                      letterSpacing: 12,
                      color: Colors.indigo,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 12),
              const Text(
                'Toque para copiar • Válido por 5 minutos',
                textAlign: TextAlign.center,
                style: TextStyle(color: Colors.black54, fontSize: 12),
              ),
            ],
            const SizedBox(height: 32),
            OutlinedButton.icon(
              onPressed: _loading ? null : _generate,
              icon: const Icon(Icons.refresh),
              label: const Text('Gerar novo código'),
            ),
          ],
        ),
      ),
    );
  }
}
