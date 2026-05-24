import 'package:firebase_auth/firebase_auth.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';

/// Tela de login e cadastro para modos Pessoal e Pais.
/// Recebe `arguments` da rota como String: 'personal' ou 'parental'.
///
/// Fluxo de cadastro com email/senha:
///   1. POST /auth/email-code/start  → código enviado por email
///   2. POST /auth/email-code/verify → retorna emailVerificationToken
///   3. Firebase createAccount       → cria conta no Firebase
///   4. POST /auth/register          → cria conta local com o token
///
/// O estado [AuthPendingLocalRegistration] é usado quando o Firebase signup
/// ocorreu mas o /auth/register ainda não foi chamado.
class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs;
  final _emailCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  final _nameCtrl = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _loading = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabs.dispose();
    _emailCtrl.dispose();
    _passwordCtrl.dispose();
    _nameCtrl.dispose();
    super.dispose();
  }

  String get _mode =>
      (ModalRoute.of(context)?.settings.arguments as String?) ?? 'personal';

  void _setError(String? msg) => setState(() => _error = msg);

  Future<void> _login() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() { _loading = true; _error = null; });
    try {
      await ref.read(authProvider.notifier).loginWithEmail(
            _emailCtrl.text.trim(),
            _passwordCtrl.text,
          );
    } on FirebaseAuthException catch (e) {
      _setError(_firebaseMessage(e.code));
    } catch (_) {
      _setError('Não foi possível conectar. Verifique sua conexão.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _loginWithGoogle() async {
    setState(() { _loading = true; _error = null; });
    try {
      await ref.read(authProvider.notifier).loginWithGoogle();
    } on FirebaseAuthException catch (e) {
      _setError(_firebaseMessage(e.code));
    } catch (_) {
      _setError('Login com Google falhou.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  /// Cadastro com email/senha — inclui verificação de código por email.
  /// TODO: implementar o fluxo de 4 passos (email-code/start → verify → Firebase
  /// → /auth/register). Por ora dispara Firebase signup + register direto,
  /// sem email-code (funciona apenas com Google ou quando o backend não exige
  /// verificação — útil para dev/testes com EMAIL_DELIVERY_MODE=log).
  Future<void> _register() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() { _loading = true; _error = null; });
    try {
      // Para email/senha: implementar aqui o fluxo de email-code antes do signup.
      // Ref: docs/AUTH_STATE_MACHINE.md § "Fluxo de entrada"
      // ref.read(apiClientProvider).emailCodeStart(_emailCtrl.text.trim());
      // ... aguardar código, verificar, obter emailVerificationToken ...

      // Firebase signup
      await FirebaseAuth.instance.createUserWithEmailAndPassword(
        email: _emailCtrl.text.trim(),
        password: _passwordCtrl.text,
      );
      // O authProvider detecta o novo usuário Firebase e emite
      // AuthPendingLocalRegistration. Então chamamos register():
      await ref.read(authProvider.notifier).register(
            displayName: _nameCtrl.text.trim(),
            mode: _mode,
            // emailVerificationToken: token obtido no passo de verificação
          );
    } on FirebaseAuthException catch (e) {
      _setError(_firebaseMessage(e.code));
    } catch (_) {
      _setError('Cadastro falhou. Tente novamente.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final modeLabel = _mode == 'parental' ? 'Pais' : 'Pessoal';
    return Scaffold(
      appBar: AppBar(
        title: Text('DopaBlocker — $modeLabel'),
        bottom: TabBar(
          controller: _tabs,
          tabs: const [Tab(text: 'Entrar'), Tab(text: 'Cadastrar')],
        ),
      ),
      body: TabBarView(
        controller: _tabs,
        children: [
          _buildLoginTab(),
          _buildRegisterTab(),
        ],
      ),
    );
  }

  Widget _buildLoginTab() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: 16),
            TextFormField(
              controller: _emailCtrl,
              decoration: const InputDecoration(labelText: 'Email', border: OutlineInputBorder()),
              keyboardType: TextInputType.emailAddress,
              validator: (v) => (v == null || !v.contains('@')) ? 'Email inválido' : null,
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _passwordCtrl,
              decoration: const InputDecoration(labelText: 'Senha', border: OutlineInputBorder()),
              obscureText: true,
              validator: (v) => (v == null || v.length < 6) ? 'Mínimo 6 caracteres' : null,
            ),
            if (_error != null) ...[
              const SizedBox(height: 12),
              Text(_error!, style: const TextStyle(color: Colors.red)),
            ],
            const SizedBox(height: 24),
            FilledButton(
              onPressed: _loading ? null : _login,
              child: _loading ? const SizedBox(height: 18, width: 18, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white)) : const Text('Entrar'),
            ),
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: _loading ? null : _loginWithGoogle,
              icon: const Icon(Icons.login),
              label: const Text('Entrar com Google'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildRegisterTab() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SizedBox(height: 16),
          TextFormField(
            controller: _nameCtrl,
            decoration: const InputDecoration(labelText: 'Nome', border: OutlineInputBorder()),
            validator: (v) => (v == null || v.trim().isEmpty) ? 'Informe seu nome' : null,
          ),
          const SizedBox(height: 16),
          TextFormField(
            controller: _emailCtrl,
            decoration: const InputDecoration(labelText: 'Email', border: OutlineInputBorder()),
            keyboardType: TextInputType.emailAddress,
          ),
          const SizedBox(height: 16),
          TextFormField(
            controller: _passwordCtrl,
            decoration: const InputDecoration(labelText: 'Senha', border: OutlineInputBorder()),
            obscureText: true,
          ),
          if (_error != null) ...[
            const SizedBox(height: 12),
            Text(_error!, style: const TextStyle(color: Colors.red)),
          ],
          const SizedBox(height: 8),
          const Text(
            'Para email/senha: um código de verificação será enviado para seu email antes de criar a conta.',
            style: TextStyle(fontSize: 12, color: Colors.black54),
          ),
          const SizedBox(height: 24),
          FilledButton(
            onPressed: _loading ? null : _register,
            child: _loading ? const SizedBox(height: 18, width: 18, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white)) : const Text('Criar conta'),
          ),
          const SizedBox(height: 12),
          OutlinedButton.icon(
            onPressed: _loading ? null : _loginWithGoogle,
            icon: const Icon(Icons.login),
            label: const Text('Cadastrar com Google'),
          ),
        ],
      ),
    );
  }

  String _firebaseMessage(String code) => switch (code) {
        'user-not-found' || 'wrong-password' || 'invalid-credential' =>
          'Email ou senha incorretos.',
        'email-already-in-use' => 'Email já está em uso.',
        'weak-password' => 'Senha muito fraca.',
        'network-request-failed' => 'Sem conexão com a internet.',
        'sign_in_cancelled' => 'Login cancelado.',
        _ => 'Erro de autenticação ($code).',
      };
}
