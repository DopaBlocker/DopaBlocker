import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'providers/auth_provider.dart';
import 'routes.dart';
import 'theme.dart';
import 'screens/splash_screen.dart';
import 'screens/welcome_screen.dart';
import 'screens/login_screen.dart';
import 'screens/child_code_screen.dart';
import 'screens/main_shell.dart';
import 'screens/child_blocked_screen.dart';
import 'screens/link_device_screen.dart';

class App extends ConsumerWidget {
  const App({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = ref.watch(authProvider);
    return MaterialApp(
      title: 'DopaBlocker',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.dark,
      themeMode: ThemeMode.dark,
      // A tela raiz é determinada pelo estado de auth; navegações internas
      // (login → home) acontecem via mudança de estado do StateNotifier.
      home: _resolveHome(auth),
      routes: {
        AppRoutes.welcome: (_) => const WelcomeScreen(),
        AppRoutes.login: (_) => const LoginScreen(),
        AppRoutes.childCode: (_) => const ChildCodeScreen(),
        AppRoutes.home: (_) => const MainShell(),
        AppRoutes.childBlocked: (_) => const ChildBlockedScreen(),
        AppRoutes.linkDevice: (_) => const LinkDeviceScreen(),
      },
    );
  }

  Widget _resolveHome(AuthState auth) => switch (auth) {
        AuthBooting() || AuthAuthenticating() => const SplashScreen(),
        AuthSignedOut() => const WelcomeScreen(),
        AuthPendingLocalRegistration() => const LoginScreen(),
        AuthBackendUnavailable() => const WelcomeScreen(),
        AuthAuthenticated() => const MainShell(),
        AuthChildSession() => const ChildBlockedScreen(),
      };
}
