import 'package:flutter/material.dart';

import '../channels/blocking_channel.dart';
import '../models/installed_app.dart';
import '../theme.dart';

/// Bottom sheet que lista os apps instalados (nome + ícone) para o usuário
/// escolher qual bloquear. Retorna o [InstalledApp] escolhido via Navigator.pop
/// (nome + package + ícone, para a confirmação no diálogo), ou null se cancelado.
/// Carrega a lista do nativo (`getInstalledApps`) de forma assíncrona, com busca
/// por nome/package.
class AppPickerSheet extends StatefulWidget {
  const AppPickerSheet({super.key});

  static Future<InstalledApp?> show(BuildContext context) =>
      showModalBottomSheet<InstalledApp>(
        context: context,
        isScrollControlled: true,
        backgroundColor: AppColors.surface,
        shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadii.xl)),
        ),
        builder: (_) => const AppPickerSheet(),
      );

  @override
  State<AppPickerSheet> createState() => _AppPickerSheetState();
}

class _AppPickerSheetState extends State<AppPickerSheet> {
  List<InstalledApp>? _apps;
  bool _failed = false;
  String _query = '';

  @override
  void initState() {
    super.initState();
    _loadApps();
  }

  Future<void> _loadApps() async {
    try {
      final apps = await BlockingChannel.getInstalledApps();
      if (mounted) setState(() => _apps = apps);
    } catch (_) {
      if (mounted) setState(() => _failed = true);
    }
  }

  List<InstalledApp> get _filtered {
    final apps = _apps;
    if (apps == null) return const [];
    final q = _query.trim().toLowerCase();
    if (q.isEmpty) return apps;
    return apps
        .where((a) =>
            a.appName.toLowerCase().contains(q) ||
            a.packageName.toLowerCase().contains(q))
        .toList();
  }

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Padding(
        padding: EdgeInsets.only(bottom: MediaQuery.of(context).viewInsets.bottom),
        child: SizedBox(
          height: MediaQuery.of(context).size.height * 0.7,
          child: Column(
            children: [
              const SizedBox(height: 10),
              Container(
                width: 36,
                height: 4,
                decoration: BoxDecoration(
                  color: AppColors.borderStrong,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 8, 4),
                child: Row(
                  children: [
                    const Expanded(
                      child: Text('Escolher app',
                          style: TextStyle(fontWeight: FontWeight.w700, fontSize: 16)),
                    ),
                    IconButton(
                      onPressed: () => Navigator.pop(context),
                      icon: const Icon(Icons.close, color: AppColors.textSecondary),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: TextField(
                  onChanged: (v) => setState(() => _query = v),
                  decoration: const InputDecoration(
                    hintText: 'Buscar app…',
                    prefixIcon: Icon(Icons.search, size: 20),
                  ),
                ),
              ),
              const SizedBox(height: 8),
              Expanded(child: _buildBody()),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildBody() {
    if (_failed) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Text(
            'Não foi possível listar os apps instalados.',
            textAlign: TextAlign.center,
            style: TextStyle(color: AppColors.textSecondary),
          ),
        ),
      );
    }
    if (_apps == null) {
      return const Center(child: CircularProgressIndicator());
    }
    final filtered = _filtered;
    if (filtered.isEmpty) {
      return const Center(
        child: Text('Nenhum app encontrado.',
            style: TextStyle(color: AppColors.textSecondary)),
      );
    }
    return ListView.builder(
      itemCount: filtered.length,
      itemBuilder: (_, i) {
        final app = filtered[i];
        return ListTile(
          leading: app.icon != null
              ? Image.memory(app.icon!, width: 36, height: 36, gaplessPlayback: true)
              : const Icon(Icons.android, size: 36, color: AppColors.textSecondary),
          title: Text(app.appName, maxLines: 1, overflow: TextOverflow.ellipsis),
          subtitle: Text(
            app.packageName,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(fontSize: 11, color: AppColors.textSecondary),
          ),
          onTap: () => Navigator.pop(context, app),
        );
      },
    );
  }
}
