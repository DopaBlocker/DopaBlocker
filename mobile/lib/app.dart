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
      // AnimatedSwitcher dá um crossfade suave entre os estados (splash →
      // welcome → home → filho) em vez de um corte seco.
      home: AnimatedSwitcher(
        duration: AppDurations.enter,
        switchInCurve: AppCurves.out,
        switchOutCurve: AppCurves.in_,
        child: KeyedSubtree(
          key: ValueKey(auth.runtimeType),
          child: _resolveHome(auth),
        ),
      ),
      onGenerateRoute: _onGenerateRoute,
    );
  }

  /// Rotas nomeadas com transição consistente (fade + leve slide, Expo-out).
  /// `settings` é repassado para preservar os argumentos (ex.: login recebe
  /// 'personal'/'parental'). Respeita reduced-motion.
  Route<dynamic>? _onGenerateRoute(RouteSettings settings) {
    final Widget? page = switch (settings.name) {
      AppRoutes.welcome => const WelcomeScreen(),
      AppRoutes.login => const LoginScreen(),
      AppRoutes.childCode => const ChildCodeScreen(),
      AppRoutes.home => const MainShell(),
      AppRoutes.childBlocked => const ChildBlockedScreen(),
      AppRoutes.linkDevice => const LinkDeviceScreen(),
      _ => null,
    };
    if (page == null) return null;
    return PageRouteBuilder(
      settings: settings,
      transitionDuration: AppDurations.enter,
      reverseTransitionDuration: AppDurations.exit,
      pageBuilder: (_, _, _) => page,
      transitionsBuilder: (context, anim, _, child) {
        final reduce = MediaQuery.maybeOf(context)?.disableAnimations ?? false;
        if (reduce) return child;
        final curved = CurvedAnimation(parent: anim, curve: AppCurves.out);
        return FadeTransition(
          opacity: curved,
          child: SlideTransition(
            position: Tween(begin: const Offset(0, 0.03), end: Offset.zero).animate(curved),
            child: child,
          ),
        );
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
