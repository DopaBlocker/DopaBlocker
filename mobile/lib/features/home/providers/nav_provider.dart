import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Índice da aba ativa no [MainShell]. Exposto como provider para que telas
/// possam navegar entre abas (ex.: o hub Início → "Gerenciar bloqueios" /
/// "Ver filhos"). Os índices seguem a ordem das destinações:
/// 0 = Início, 1 = Bloqueios, 2 = Filhos (parental) ou Conta (pessoal).
final navIndexProvider = StateProvider<int>((ref) => 0);

/// Índices estáveis das abas, independentes do modo da conta.
abstract final class NavTab {
  static const inicio = 0;
  static const bloqueios = 1;
  static const filhos = 2; // só existe em conta parental
}
