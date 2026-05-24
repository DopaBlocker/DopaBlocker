import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'providers/auth_provider.dart';
import 'routes.dart';
import 'screens/splash_screen.dart';
import 'screens/welcome_screen.dart';
import 'screens/login_screen.dart';
import 'screens/child_code_screen.dart';
import 'screens/home_screen.dart';
import 'screens/child_blocked_screen.dart';
import 'screens/blocking_screen.dart';
import 'screens/parental_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/link_device_screen.dart';

class App extends ConsumerWidget {
  const App({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = ref.watch(authProvider);
    return MaterialApp(
      title: 'DopaBlocker',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF4F46E5)),
        useMaterial3: true,
      ),
      // A tela raiz é determinada pelo estado de auth.
      // Navegações internas (ex: login → home) são gerenciadas pelo próprio
      // state notifier via mudança de estado — o ConsumerWidget re-renderiza.
      home: _resolveHome(auth),
      routes: {
        AppRoutes.welcome: (_) => const WelcomeScreen(),
        AppRoutes.login: (_) => const LoginScreen(),
        AppRoutes.childCode: (_) => const ChildCodeScreen(),
        AppRoutes.home: (_) => const HomeScreen(),
        AppRoutes.childBlocked: (_) => const ChildBlockedScreen(),
        AppRoutes.blocking: (_) => const BlockingScreen(),
        AppRoutes.parental: (_) => const ParentalScreen(),
        AppRoutes.settings: (_) => const SettingsScreen(),
        AppRoutes.linkDevice: (_) => const LinkDeviceScreen(),
      },
    );
  }

  Widget _resolveHome(AuthState auth) => switch (auth) {
        AuthBooting() || AuthAuthenticating() => const SplashScreen(),
        AuthSignedOut() => const WelcomeScreen(),
        AuthPendingLocalRegistration() => const LoginScreen(),
        AuthBackendUnavailable() => const WelcomeScreen(), // exibe snackbar de erro na tela
        AuthAuthenticated() => const HomeScreen(),
        AuthChildSession() => const ChildBlockedScreen(),
      };
}
