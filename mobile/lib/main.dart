// Entry point do app mobile DopaBlocker.
// Implementar: inicializar Firebase (WidgetsFlutterBinding.ensureInitialized,
// Firebase.initializeApp), configurar Riverpod (ProviderScope),
// rodar o widget App definido em app.dart.

import 'package:flutter/material.dart';

void main() {
  runApp(const MainApp());
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      home: Scaffold(
        body: Center(
          child: Text('DopaBlocker'),
        ),
      ),
    );
  }
}
