import 'package:flutter/material.dart';

import '../routes.dart';

/// Controle parental: lista de dispositivos filhos, geração de código de vinculação.
/// Fase 2: chamar GET /devices, exibir filhos, navegar para LinkDeviceScreen.
class ParentalScreen extends StatelessWidget {
  const ParentalScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Controle Parental')),
      body: const Center(
        child: Text('Controle Parental — em construção', style: TextStyle(color: Colors.black54)),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => Navigator.pushNamed(context, AppRoutes.linkDevice),
        icon: const Icon(Icons.add_link),
        label: const Text('Vincular filho'),
      ),
    );
  }
}
