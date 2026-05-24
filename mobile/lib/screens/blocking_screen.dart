import 'package:flutter/material.dart';

/// Gestão de bloqueios.
/// Fase 2: lista de itens bloqueados, toggle de conteúdo adulto,
/// botão master de bloqueio (modo pessoal).
class BlockingScreen extends StatelessWidget {
  const BlockingScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Bloqueios')),
      body: const Center(
        child: Text('Bloqueios — em construção', style: TextStyle(color: Colors.black54)),
      ),
    );
  }
}
